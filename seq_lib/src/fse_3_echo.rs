use std::cell::RefCell;
use std::rc::Rc;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use seq_tools::grad_cal;
use seq_tools::acq_event::{AcqEvent, SpectralWidth};
use seq_tools::event_block::{Event, EventQueue, GradEventType};
use seq_tools::event_block::EventPlacementType::{After, Before, ExactFromOrigin, Origin};
use seq_tools::execution::ExecutionBlock;
use seq_tools::gradient_event::GradEvent;
use seq_tools::gradient_matrix::{DacValues, Dimension, DriverVar, EncodeStrategy, LinTransform, Matrix, MatrixDriver, MatrixDriverType};
use seq_tools::ppl::BaseFrequency::Civm9p4T;
use seq_tools::ppl::{BaseFrequency, GradClock, Orientation, PhaseUnit, PPL};
use seq_tools::pulse::{CompositeHardpulse, Hardpulse, Pulse, Trapezoid};
use seq_tools::rf_event::RfEvent;
use seq_tools::rf_state::{PhaseCycleStrategy, RfDriver, RfDriverType, RfStateType};
use seq_tools::seqframe::SeqFrame;
use seq_tools::utils;
use seq_tools::utils::us_to_clock;

#[test]
fn test(){
    //let mut mep = MultiEcho3D::default_params();
    let mut mep = MultiEcho3D::low_res_params();
    mep.setup_mode = true;
    let sim_mode = false;
    let acceleration = 2;
    let output_dir = Path::new("/mnt/d/dev/221011");
    let me = MultiEcho3D::new(mep);
    //me.plot_export(4,100,"/mnt/d/dev/plotter/output");
    //me.plot_export(4,0,"output");
    let ppl = me.ppl_export(Civm9p4T(0.0),Orientation::CivmStandard,acceleration,sim_mode);

    let filename = output_dir.join("multi_echo.ppl");
    let mut outfile = File::create(filename).expect("cannot create file");
    //let mut outfile = File::create("multi_echo.ppl").expect("cannot create file");
    outfile.write_all(ppl.print().as_bytes()).expect("cannot write to file");
    me.seq_export(4,output_dir.to_str().unwrap());
    //me.seq_export(4,".");
}

#[derive(Clone)]
pub struct MultiEcho3DParams {
    fov: (f32, f32, f32),
    samples: (u16, u16, u16),
    sample_discards: u16,
    spectral_width: SpectralWidth,
    ramp_time: f32,
    phase_encode_time:f32,
    echo_time:f32,
    echo_time2:f32,
    rep_time:f32,
    setup_mode:bool
}

pub struct MultiEcho3D {
    params:MultiEcho3DParams,
    events:MultiEcho3DEvents,
}

pub struct MultiEcho3DEvents{
    excitation:RfEvent<Hardpulse>,

    diffusion:GradEvent<Trapezoid>,

    refocus1:RfEvent<CompositeHardpulse>,
    refocus2:RfEvent<CompositeHardpulse>,
    refocus3:RfEvent<CompositeHardpulse>,
    //refocus4:RfEvent<CompositeHardpulse>,

    phase_encode_1:GradEvent<Trapezoid>,
    phase_encode_2:GradEvent<Trapezoid>,
    phase_encode_3:GradEvent<Trapezoid>,
    //phase_encode_4:GradEvent<Trapezoid>,

    readout:GradEvent<Trapezoid>,
    acquire:AcqEvent,

    rewinder1:GradEvent<Trapezoid>,
    rewinder2:GradEvent<Trapezoid>,
    rewinder3:GradEvent<Trapezoid>,

    spoiler:GradEvent<Trapezoid>,

}

impl MultiEcho3D {

    pub fn default_params() -> MultiEcho3DParams {
        MultiEcho3DParams {
            fov:(19.7,12.0,12.0),
            samples:(788,480,480),
            sample_discards:0,
            spectral_width:SpectralWidth::SW200kH,
            ramp_time:100E-6,
            phase_encode_time:500E-6,
            echo_time:13.0E-3,
            echo_time2:6.530E-3,
            rep_time:100.0E-3,
            setup_mode:false
        }
    }

    pub fn low_res_params() -> MultiEcho3DParams {
        MultiEcho3DParams {
            fov:(19.7,12.0,12.0),
            samples:(788,256,256),
            sample_discards:0,
            spectral_width:SpectralWidth::SW200kH,
            ramp_time:100E-6,
            phase_encode_time:500E-6,
            echo_time:13.0E-3,
            echo_time2:7.0E-3,
            rep_time:80.0E-3,
            setup_mode:false
        }
    }

    pub fn _45um_params() -> MultiEcho3DParams {
        MultiEcho3DParams {
            fov:(19.7,12.0,12.0),
            samples:(420,256,256),
            sample_discards:0,
            spectral_width:SpectralWidth::SW200kH,
            ramp_time:100E-6,
            phase_encode_time:500E-3,
            echo_time:14.0E-3,
            echo_time2:7E-3,
            rep_time:100E-3,
            setup_mode:false
        }
    }

    pub fn new(params:MultiEcho3DParams) -> MultiEcho3D {
        let events = Self::build_events(params.clone());
        Self {
            events,
            params
        }
    }

    pub fn build_events(params:MultiEcho3DParams) -> MultiEcho3DEvents {
        let n_read = params.samples.0;
        let n_discards = params.sample_discards;
        let n_phase = params.samples.1;
        let n_slice = params.samples.2;
        let fov_read = params.fov.0;
        let fov_phase = params.fov.1;
        let fov_slice = params.fov.2;
        let spectral_width = params.spectral_width;
        let read_ramp_time = params.ramp_time;
        let phase_encode_ramp_time = params.ramp_time;
        let phase_encode_time = params.phase_encode_time;


        /* mat_count is a pointer that keeps track of which matrices have been created to ensure they each get a unique id */
        let mat_count = Matrix::new_tracker();


        /* READOUT */
        // Calculate sample time
        let read_sample_time_sec = spectral_width.sample_time(n_read + n_discards);
        // Find correct readout dac value for the field of view and spectral width
        let read_grad_dac = spectral_width.fov_to_dac(fov_read);
        // define the matrix
        let read_matrix = Matrix::new_static("read_mat",DacValues::new(Some(read_grad_dac),None,None),&mat_count);
        // define the waveform
        let read_waveform = Trapezoid::new(read_ramp_time,read_sample_time_sec);
        /* define the event only on the read channel. It is non-blocking to allow an acquisition event
        to occur at the same time
        */
        let readout = GradEvent::new(
            (Some(read_waveform),
             None,
             None),
            &read_matrix,
            GradEventType::NonBlocking,
            "readout"
        );

        /* ACQUISITION */
        /* define the acq event with a static phase of 0 */
        let acquire = AcqEvent::new(
            "acquire",
            spectral_width,
            n_read,
            n_discards,
            RfStateType::Static(0)
        );

        /* EXCITATION */
        /* excitation pulse is a single hard pulse with a user-adjustable power and static phase set
        to 0
        */
        let excite_waveform = Hardpulse::new(100E-6);
        let excite_power = RfStateType::Adjustable(513);
        let excite_phase = RfStateType::Static(0);
        let excitation = RfEvent::new(
            "excitation",
            1,
            excite_waveform,
            excite_power,
            excite_phase
        );

        let echo_index = vec![0,1,1];

        /* REFOCUS */
        /* Composite hard pulse that gets a different phase depending on k-space coordinate read from a LUT */
        let refocus_waveform = CompositeHardpulse::new_180(200E-6);
        let refocus_power = RfStateType::Adjustable(897);
        let rf_phase_cycle_strategy = PhaseCycleStrategy::CycleCPMG(2);
        // 90  270  90  270
        // 270  90  270  90
        // 90  270  90  270
        let cycle = RfDriverType::PhaseCycle3D(rf_phase_cycle_strategy);

        let ref_driver1 = RfDriver::new(DriverVar::Repetition, cycle.clone(), Some(0));
        //let ref_driver1 = RfDriver::new(DriverVar::Repetition, cycle.clone(), Some(echo_index[0]));
        let refocus_phase1 = RfStateType::Driven(ref_driver1);

        let ref_driver2 = RfDriver::new(DriverVar::Repetition, cycle.clone(), Some(1));
        //let ref_driver2 = RfDriver::new(DriverVar::Repetition, cycle.clone(), Some(echo_index[1]));
        let refocus_phase2 = RfStateType::Driven(ref_driver2);

        let ref_driver3 = RfDriver::new(DriverVar::Repetition, cycle.clone(), Some(2));
        //let ref_driver3 = RfDriver::new(DriverVar::Repetition, cycle.clone(), Some(echo_index[2]));
        let refocus_phase3 = RfStateType::Driven(ref_driver3);

        //let ref_driver4 = RfDriver::new(DriverVar::Repetition, cycle.clone(), Some(echo_index[3]));
        //let refocus_phase4 = RfStateType::Driven(ref_driver4);



        let refocus1 = RfEvent::new(
            "refocus1",
            2,
            refocus_waveform.clone(),
            refocus_power.clone(),
            refocus_phase1,
        );

        let refocus2 = RfEvent::new(
            "refocus2",
            3,
            refocus_waveform.clone(),
            refocus_power.clone(),
            refocus_phase2,
        );

        let refocus3 = RfEvent::new(
            "refocus3",
            4,
            refocus_waveform.clone(),
            refocus_power.clone(),
            refocus_phase3,
        );

        // let refocus4 = RfEvent::new(
        //     "refocus4",
        //     5,
        //     refocus_waveform.clone(),
        //     refocus_power.clone(),
        //     refocus_phase4,
        // );


        /* GRADIENT SPOILER */
        /* the spoiler is active on all channels at the end of the echo train to de-phase any residual signal
         */
        let spoiler_waveform = Trapezoid::new(100E-6,1E-3);
        let spoiler_matrix = Matrix::new_static("spoiler_mat",DacValues::new(Some(4000),Some(0),Some(0)),&mat_count);

        let spoiler = GradEvent::new(
            (Some(spoiler_waveform),
             Some(spoiler_waveform),
             Some(spoiler_waveform)),
            &spoiler_matrix,
            GradEventType::Blocking,
            "spoiler"
        );


        /* PHASE ENCODE */
        /* first phase encode ahead of the first echo in the echo train.
         this is zeroed if setup mode is set to true.
         */
        let crusher_dac = 4000;
        let phase_encode_waveform = Trapezoid::new(phase_encode_ramp_time,phase_encode_time);
        // determine the phase step increments based in fov
        let (phase_grad_step,slice_grad_step) = match params.setup_mode {
            false => {
                let phase_grad_step = phase_encode_waveform.magnitude_net(1.0/fov_phase);
                let slice_grad_step = phase_encode_waveform.magnitude_net(1.0/fov_slice);
                (phase_grad_step,slice_grad_step)
            }
            true => {
                (0.0,0.0)
            }
        };
        // 3D phase encoding via lookup table. This table can be simulated here with this vector
        let lut = vec![240;230400];
        let phase_encode_strategy = EncodeStrategy::LUT(Dimension::_3D,lut);
        // set the phase encoding driver to the view loop counter //todo!(make an enum of the available driver variables)


        let pe_driver1 = MatrixDriver::new(DriverVar::Repetition,MatrixDriverType::PhaseEncode(phase_encode_strategy.clone()),Some(echo_index[0]));
        let pe_driver2 = MatrixDriver::new(DriverVar::Repetition,MatrixDriverType::PhaseEncode(phase_encode_strategy.clone()),Some(echo_index[1]));
        let pe_driver3 = MatrixDriver::new(DriverVar::Repetition,MatrixDriverType::PhaseEncode(phase_encode_strategy.clone()),Some(echo_index[2]));
        //let pe_driver4 = MatrixDriver::new(DriverVar::Repetition,MatrixDriverType::PhaseEncode(phase_encode_strategy.clone()),Some(echo_index[3]));



        // this also includes a read pre-phase for the first readout (half the power of the readout waveform)
        let read_pre_phase_dac = phase_encode_waveform.magnitude_net(0.5*read_waveform.power_net(read_grad_dac as f32)) as i16;

        let phase_multiplier = grad_cal::grad_to_dac(phase_grad_step) as f32;
        let slice_multiplier = grad_cal::grad_to_dac(slice_grad_step) as f32;

        // transform for the k-space coordinates read in from the LUT. This phase encode driver only operates on the phase and slice channels
        let transform = LinTransform::new((None,Some(phase_multiplier),Some(slice_multiplier)),(None,None,None));
        // the read pre-phasing is static and gets a simple dac value
        let static_dac_vals = DacValues::new(Some(-read_pre_phase_dac),None,None);
        // define the matrix
        let mut phase_encode_matrix1 = Matrix::new_driven(
            "c_pe_mat1",
            pe_driver1.clone(),
            transform,
            static_dac_vals,
            &mat_count
        );
        phase_encode_matrix1.adjustable = (true,false,false);

        let static_crusher = DacValues::new(Some(crusher_dac),None,None);
        let phase_encode_matrix2 = Matrix::new_driven(
            "c_pe_mat2",
            pe_driver2.clone(),
            transform,
            DacValues::new(Some(crusher_dac),None,None),
            &mat_count
        );

        let phase_encode_matrix3 = Matrix::new_driven(
            "c_pe_mat3",
            pe_driver3.clone(),
            transform,
            DacValues::new(Some(crusher_dac),None,None),
            &mat_count
        );

        // let phase_encode_matrix4 = Matrix::new_driven(
        //     "c_pe_mat4",
        //     pe_driver4.clone(),
        //     transform,
        //     DacValues::new(Some(crusher_dac),None,None),
        //     &mat_count
        // );

        // build event
        let phase_encode_1 = GradEvent::new(
            (Some(phase_encode_waveform),
             Some(phase_encode_waveform),
             Some(phase_encode_waveform)),
            &phase_encode_matrix1,
            GradEventType::Blocking,
            "phase_encode_1"
        );

        let phase_encode_2 = GradEvent::new(
            (Some(phase_encode_waveform),
             Some(phase_encode_waveform),
             Some(phase_encode_waveform)),
            &phase_encode_matrix2,
            GradEventType::Blocking,
            "phase_encode_2"
        );

        let phase_encode_3 = GradEvent::new(
            (Some(phase_encode_waveform),
             Some(phase_encode_waveform),
             Some(phase_encode_waveform)),
            &phase_encode_matrix3,
            GradEventType::Blocking,
            "phase_encode_3"
        );

        // let phase_encode_4 = GradEvent::new(
        //     (Some(phase_encode_waveform),
        //      Some(phase_encode_waveform),
        //      Some(phase_encode_waveform)),
        //     &phase_encode_matrix4,
        //     GradEventType::Blocking,
        //     "phase_encode_4"
        // );


        /* RE-WINDERS */
        /* rewinders are derived from the phase encode events, effectively reversing what they've done */
        let mut rewinder_matrix1 = Matrix::new_derived(
            "c_rewind_mat1",
            &Rc::new(phase_encode_matrix1.clone()),
            LinTransform::new((Some(0.0),Some(-1.0),Some(-1.0)),(Some(crusher_dac), Some(0), Some(0))),
            &mat_count
        );
        rewinder_matrix1.adjustable = (true,false,false);

        let mut rewinder_matrix2 = Matrix::new_derived(
            "c_rewind_mat2",
            &Rc::new(phase_encode_matrix2.clone()),
            LinTransform::new((Some(0.0),Some(-1.0),Some(-1.0)),(Some(crusher_dac), Some(0), Some(0))),
            &mat_count
        );
        rewinder_matrix2.adjustable = (true,false,false);

        let rewinder_matrix3 = Matrix::new_derived(
            "c_rewind_mat3",
            &Rc::new(phase_encode_matrix3.clone()),
            LinTransform::new((Some(0.0),Some(-1.0),Some(-1.0)),(Some(crusher_dac), Some(0), Some(0))),
            &mat_count
        );


        let rewinder1 = GradEvent::new(
            (Some(phase_encode_waveform),
             Some(phase_encode_waveform),
             Some(phase_encode_waveform)),
            &rewinder_matrix1,
            GradEventType::Blocking,
            "rewinder1"
        );

        let rewinder2 = GradEvent::new(
            (Some(phase_encode_waveform),
             Some(phase_encode_waveform),
             Some(phase_encode_waveform)),
            &rewinder_matrix2,
            GradEventType::Blocking,
            "rewinder2"
        );

        let rewinder3 = GradEvent::new(
            (Some(phase_encode_waveform),
             Some(phase_encode_waveform),
             Some(phase_encode_waveform)),
            &rewinder_matrix3,
            GradEventType::Blocking,
            "rewinder3"
        );



        /* DIFFUSION */
        /* the diffusion pulse is a simple trapezoid with a static dac value on the read channel.
         this is for tuning rf power to minimize the stimulated echo signal
         */
        let diffusion_waveform = Trapezoid::new(100E-6,2E-3);
        let diffusion_dac = match params.setup_mode {
            true => 500,
            false => 0
        };
        let diffusion_mat = Matrix::new_static("diffusion_mat",DacValues::new(Some(diffusion_dac),Some(0),Some(0)),&mat_count);
        let diffusion = GradEvent::new(
            (Some(diffusion_waveform),
             Some(diffusion_waveform),
             Some(diffusion_waveform)),
            &diffusion_mat,
            GradEventType::Blocking,
            "diffusion"
        );

        /* export events for placement */
        MultiEcho3DEvents{
            excitation,
            diffusion,
            refocus1,refocus2,refocus3,
            phase_encode_1,phase_encode_2,phase_encode_3,
            readout,
            acquire,
            rewinder1,rewinder2,rewinder3,
            spoiler
        }
    }
    fn place_events(&self) -> EventQueue{

        let te_us = utils::sec_to_us(self.params.echo_time);
        let hte = te_us/2;

        let te_us2 = utils::sec_to_us(self.params.echo_time2);
        let hte2 = te_us2/2;
        let offset = te_us;

        let excite = Event::new(self.events.excitation.as_reference(),Origin);

        let d1 = Event::new(self.events.diffusion.as_reference(),After(excite.clone(),0));

        let read_locations = vec![
            te_us,
            te_us2 + offset,
            2*te_us2 + offset,
            3*te_us2 + offset,
        ];
        let refocus_locations = vec![
            read_locations[0] - hte,
            read_locations[1] - hte2,
            read_locations[2] - hte2,
            read_locations[3] - hte2,
        ];

        let read:Vec<Rc<RefCell<Event>>> = read_locations.iter().map(|t| Event::new(self.events.readout.as_reference(),ExactFromOrigin(us_to_clock(*t))) ).collect();
        let acq:Vec<Rc<RefCell<Event>>> = read_locations.iter().map(|t| Event::new(self.events.acquire.as_reference(),ExactFromOrigin(us_to_clock(*t))) ).collect();
        //let refocus:Vec<Rc<RefCell<Event>>> = refocus_locations.iter().map(|t| Event::new(self.events.refocus.as_reference(),ExactFromOrigin(us_to_clock(*t))) ).collect();

        let refocus1 =  Event::new(self.events.refocus1.as_reference(),ExactFromOrigin(us_to_clock(refocus_locations[0])));
        let refocus2 =  Event::new(self.events.refocus2.as_reference(),ExactFromOrigin(us_to_clock(refocus_locations[1])));
        let refocus3 =  Event::new(self.events.refocus3.as_reference(),ExactFromOrigin(us_to_clock(refocus_locations[2])));
        //let refocus4 =  Event::new(self.events.refocus4.as_reference(),ExactFromOrigin(us_to_clock(refocus_locations[3])));

        let pe1 = Event::new(self.events.phase_encode_1.as_reference(),Before(read[0].clone(),0));
        let d2 = Event::new(self.events.diffusion.as_reference(),Before(pe1.clone(),0));
        let re1 = Event::new(self.events.rewinder1.as_reference(),Before(refocus2.clone(),0));
        let pe2 = Event::new(self.events.phase_encode_2.as_reference(),After(refocus2.clone(),0));
        let re2 = Event::new(self.events.rewinder2.as_reference(),Before(refocus3.clone(),0));

        let pe3 = Event::new(self.events.phase_encode_3.as_reference(),After(refocus3.clone(),0));

        //let re3 = Event::new(self.events.rewinder3.as_reference(),Before(refocus4.clone(),0));
        //let pe4 = Event::new(self.events.phase_encode_4.as_reference(),After(refocus4.clone(),0));

        let grad_spoil = Event::new(self.events.spoiler.as_reference(),After(acq[2].clone(),0));

        let mut events = vec![excite,d1,d2];

        let refocus_sub = vec![refocus1,refocus2,refocus3];
        let read_sub = &read[0..3];
        let acq_sub = &acq[0..3];

        events.extend(refocus_sub.to_owned());
        events.extend(read_sub.to_owned());
        events.extend(acq_sub.to_owned());
        events.extend(vec![pe1,re1,pe2,re2,pe3,//re3,pe4,
                           grad_spoil,
        ]);
        EventQueue::new(&events)
    }
    // pub fn plot_export(&self,sample_period_us:usize,driver_val:u32,filename:&str){
    //     let file = Path::new(filename);
    //     let graphs = self.place_events().graphs_dynamic(sample_period_us,driver_val);
    //     let s = serde_json::to_string_pretty(&graphs).expect("cannot serialize");
    //     let mut f = File::create(file).expect("cannot create file");
    //     f.write_all(&s.as_bytes()).expect("trouble writing to file");
    // }
    pub fn ppl_export(&self,base_frequency:BaseFrequency,orientation:Orientation,acceleration:u16,simulation_mode:bool) -> PPL {
        let averages = 100;
        //let repetitions = (self.params.samples.1 as u32*self.params.samples.2 as u32);
        let repetitions = 8192;
        PPL::new(
            &mut self.place_events(),repetitions,averages,self.params.rep_time,base_frequency,
            r"d:\dev\221011\civm_grad.seq",r"d:\dev\221011\civm_rf.seq",
            orientation,GradClock::CPS20,PhaseUnit::Min,acceleration,simulation_mode)
    }

    pub fn seq_export(&self,sample_period_us:usize,filepath:&str){
        let q = self.place_events();
        let (grad_params,rf_params) = q.ppl_seq_params(sample_period_us);
        //let path = std::env::current_dir().expect("cannot get current dir");
        let path = Path::new(filepath);
        let grad_param = Path::new("civm_grad_params").with_extension("txt");
        let grad_param_path = path.join(grad_param);
        let rf_param = Path::new("civm_rf_params").with_extension("txt");
        let rf_param_path = path.join(rf_param);
        let mut rf_seq_file = File::create(rf_param_path).expect("cannot create file");
        rf_seq_file.write_all(&SeqFrame::format_as_bytes(&rf_params.unwrap())).expect("trouble writing to file");
        let mut grad_seq_file = File::create(grad_param_path).expect("cannot create file");
        grad_seq_file.write_all(&SeqFrame::format_as_bytes(&grad_params.unwrap())).expect("trouble writing to file");
    }
}
