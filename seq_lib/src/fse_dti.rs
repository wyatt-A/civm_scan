use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Read, Write};
use seq_tools::grad_cal;
use seq_tools::acq_event::{AcqEvent, SpectralWidth};
use seq_tools::event_block::{Event, EventQueue, GradEventType};
use seq_tools::event_block::EventPlacementType::{After, Before, ExactFromOrigin, Origin};
use seq_tools::execution::ExecutionBlock;
use seq_tools::gradient_event::GradEvent;
use seq_tools::gradient_matrix::{DacValues, Dimension, DriverVar, EncodeStrategy, LinTransform, Matrix, MatrixDriver, MatrixDriverType};
use seq_tools::ppl::BaseFrequency;
use seq_tools::ppl::{GradClock, Orientation, PhaseUnit};
use seq_tools::pulse::{CompositeHardpulse, HalfSin, Hardpulse, Pulse, Trapezoid};
use seq_tools::rf_event::RfEvent;
use seq_tools::rf_state::{PhaseCycleStrategy, RfDriver, RfDriverType, RfStateType};
use seq_tools::utils::{clock_to_sec, sec_to_clock, us_to_clock};
use crate::pulse_sequence::{Build, PPLBaseParams, PulseSequence};
use serde_json;
use serde::{Serialize,Deserialize};


/*
civm_scan new_study -> "/d/smis/20221031"
(creates a study directory and sets up file structure)

civm_scan scout "/d/smis/20221031"
(runs a basic scout localizer, writing to cfl images)

civm_scan pre_scan "/d/smis/20221031" -> "/d/smis/20221031/setup.txt"
(finds base frequency offset and runs rf power calibration. Writes to a setup text file)

civm_scan new_protocol 20.5xFAD.01 -> "/d/smis/20221031/_01"
(creates a sub-directory for the protocol, loads in parameters and writes sequences to the directory)

civm_scan setup_protocol "/d/smis/20221031/_01"
(starts the setup sequence for the manual tuning process)

civm_scan run_protocol "/d/smis/20221031/_01"
()


 */

#[test]
fn test(){
    // let mut mep = SpinEchoDW::_25um();
    // mep.n_averages = 1;
    // mep.n_repetitions = 2000;
    // mep.setup_mode = false;
    // let sim_mode = false;
    // mep.grad_off = false;
    // mep.phase_encode_time = 800E-6;
    //
    // mep.n_repetitions = 2;
    // let mut me = SpinEchoDW::new(mep.clone());
    // me.ppl_export(Path::new(r"d:\dev\221031\fse\sim"),"setup",true,true);
    //
    // // Rf Power/Gradient tuning
    // mep.grad_off = false;
    // mep.n_averages = 2000;
    // mep.setup_mode = false;
    // let mut me = SpinEchoDW::new(mep.clone());
    // me.ppl_export(Path::new(r"d:\dev\221031\fse\acquire"),"setup",false,true);
    //
    // // Rf Power/Gradient tuning
    // mep.grad_off = false;
    // mep.n_averages = 1;
    // mep.n_repetitions = 2000;
    // mep.setup_mode = true;
    // let mut me = SpinEchoDW::new(mep.clone());
    // me.ppl_export(Path::new(r"d:\dev\221031\fse\rf_power"),"setup",false,true);
    //
    // // Spin Echo Tuning
    // mep.grad_off = true;
    // mep.n_averages = 50;
    // mep.n_repetitions = 2;
    // let mut me = SpinEchoDW::new(mep.clone());
    // me.ppl_export(Path::new(r"d:\dev\221031\fse\se_timing"),"setup",false,true);


    let mut sequence = SpinEchoDW::default();

    sequence.params.setup_mode = false;
    sequence.params.n_repetitions = 2000;
    sequence.to_file(Path::new(r"D:\dev\221031\protocols\20.5xfad.01.json"));

    let b_table = Path::new(r"C:\workstation\data\diffusion_table\ICO61_6b0.txt");

    let mut f = File::open(b_table).expect("b_vec table not found");
    let mut file_string = String::new();
    f.read_to_string(&mut file_string).expect("trouble reading from file");
    println!("{}",file_string);



    let mut b_table = Vec::<(f32,f32,f32,f32)>::new();
    file_string.lines().for_each(|line| {
        if !line.starts_with("#") && !line.is_empty() {
            let s = line.split(", ");
            let values:Vec<f32> = s.map(|elem| elem.parse().expect(&format!("unable to parse {}",elem))).collect();
            if values.len() == 4 {
                b_table.push((values[0],values[1],values[2],values[3]));
            }
        }
    });

    let mut s = SpinEchoDW::from_file(Path::new(r"D:\dev\221031\protocols\20.5xfad.01.json"));
    let b_val = s.params.b_value;
    let experiments:Vec<SpinEchoDW> = b_table.iter().map(|exp| {
        let scale = exp.0;
        let direction = (exp.1,exp.2,exp.3);
        s.params.b_value = b_val*scale;
        s.params.b_vec = direction;
        s.clone()
    }).collect();



}

impl DiffusionSeries for SpinEchoDW {}

pub trait DiffusionSeries:DiffusionWeighted {
    fn read_table(table:&Path) -> Vec<(f32,f32,f32,f32)>{
        let mut f = File::open(table).expect("b_vec table not found");
        let mut file_string = String::new();
        f.read_to_string(&mut file_string).expect("trouble reading from file");
        let mut b_table = Vec::<(f32,f32,f32,f32)>::new();
        file_string.lines().for_each(|line| {
            if !line.starts_with("#") && !line.is_empty() {
                let s = line.split(", ");
                let values:Vec<f32> = s.map(|elem| elem.parse().expect(&format!("unable to parse {}",elem))).collect();
                if values.len() == 4 {
                    b_table.push((values[0],values[1],values[2],values[3]));
                }
            }
        });
        b_table
    }
    fn build(&self,table_path:&Path) -> Vec<Rc<&Self>>{
        let tab = Self::read_table(table_path);
        let mut s = self.clone();
        let bval = s.bvalue();
        tab.iter().map(|exp| {
            let scale = exp.0;
            let direction = (exp.1,exp.2,exp.3);
            s.set_balue(bval*scale);
            s.set_bvec(direction);
            Rc::new(s.clone())
        }).collect()

    }
}


pub trait DiffusionWeighted: Build {
    fn bvalue(&self) -> f32;
    fn set_balue(&mut self,b_value:f32);
    fn b_vec(&self) -> (f32,f32,f32);
    fn set_bvec(&mut self,b_vec:(f32,f32,f32));
}

impl DiffusionWeighted for SpinEchoDW {
    fn bvalue(&self) -> f32 {
        self.params.b_value
    }
    fn set_balue(&mut self, b_value: f32) {
        self.params.b_value = b_value;
    }
    fn b_vec(&self) -> (f32, f32, f32) {
        self.params.b_vec.clone()
    }
    fn set_bvec(&mut self, b_vec: (f32, f32, f32)) {
        self.params.b_vec = b_vec;
    }
}


impl PulseSequence for SpinEchoDW {
    fn name() -> String {
        String::from("fse_dw")
    }
    fn default() -> Self {
        let params = SpinEchoDWParams {
            name:SpinEchoDW::name(),
            view_table:Path::new(r"C:\workstation\data\petableCS_stream\fse\stream_CS480_8x_pa18_pb54").to_owned(),
            b_value:3000.0,
            b_vec:(1.0,0.0,0.0),
            fov:(19.7,12.0,12.0),
            samples:(788,480,480),
            sample_discards:0,
            spectral_width:SpectralWidth::SW200kH,
            rf_90_duration:140E-6,
            rf_180_duration:280E-6,
            diff_pulse_duration:3.5E-3,
            diff_pulse_separation:4E-3,
            spoil_duration:600E-6,
            ramp_time:140E-6,
            read_extension: 0.0,
            phase_encode_time:550E-6,
            echo_time:13.98E-3,
            echo_spacing:7.2E-3,
            obs_freq_offset: 0.0,
            rep_time:80E-3,
            n_averages: 1,
            n_repetitions: 28800,
            setup_mode:false,
            grad_off:false
        };
        SpinEchoDW::new(params)
    }
    fn to_file(&self,filepath:&Path) {
        let json_str = serde_json::to_string_pretty(&self.params).unwrap();
        let mut f  = File::create(filepath).expect(&format!("Cannot create file {:?}",filepath));
        f.write_all(json_str.as_bytes()).expect("trouble writing to file");
    }
    fn from_file(filepath:&Path) -> Self {
        let mut f = File::open(filepath).expect("cannot open file");
        let mut str = String::new();
        f.read_to_string(&mut str).expect("trouble reading from file");
        let params:SpinEchoDWParams = serde_json::from_str(&str).expect("cannot deserialize data format");
        Self::new(params)
    }
}


impl Build for SpinEchoDW {

    fn place_events(&self) -> EventQueue {
        self.place_events()
    }
    fn base_params(&self) -> PPLBaseParams {
        PPLBaseParams {
            n_averages: self.params.n_averages,
            n_repetitions: self.params.n_repetitions,
            rep_time: self.params.rep_time,
            base_frequency: BaseFrequency::civm9p4t(self.params.obs_freq_offset),
            orientation: Orientation::CivmStandard,
            grad_clock: GradClock::CPS20,
            phase_unit: PhaseUnit::Min,
            view_acceleration: 2,
            waveform_sample_period_us: 2
        }
    }
}

#[derive(Clone,Serialize,Deserialize)]
pub struct SpinEchoDWParams {
    name:String,
    view_table:PathBuf,
    b_value:f32,
    b_vec:(f32,f32,f32),
    fov: (f32, f32, f32),
    samples: (u16, u16, u16),
    sample_discards: u16,
    spectral_width: SpectralWidth,
    rf_90_duration:f32,
    rf_180_duration:f32,
    diff_pulse_duration:f32,
    diff_pulse_separation:f32,
    spoil_duration:f32,
    ramp_time:f32,
    read_extension:f32,
    phase_encode_time:f32,
    echo_time:f32,
    echo_spacing:f32,
    obs_freq_offset:f32,
    rep_time:f32,
    n_averages:u16,
    n_repetitions:u32,
    setup_mode:bool,
    grad_off:bool,
}

#[derive(Clone)]
pub struct SpinEchoDW {
    params: SpinEchoDWParams,
    events: SpinEchoDWEvents,
}

#[derive(Clone)]
pub struct SpinEchoDWEvents {
    excitation: RfEvent<Hardpulse>,
    diffusion: GradEvent<HalfSin>,
    refocus1: RfEvent<CompositeHardpulse>,
    refocus2: RfEvent<CompositeHardpulse>,
    phase_encode1: GradEvent<Trapezoid>,
    phase_encode2: GradEvent<Trapezoid>,
    readout: GradEvent<Trapezoid>,
    acquire: AcqEvent,
    spoiler: GradEvent<Trapezoid>,
    refocus3: RfEvent<CompositeHardpulse>,
    phase_encode3: GradEvent<Trapezoid>,
    rewind1: GradEvent<Trapezoid>,
    rewind2: GradEvent<Trapezoid>,
}

struct Waveforms {
    excitation:Hardpulse,
    diffusion:HalfSin,
    refocus:CompositeHardpulse,
    phase_encode:Trapezoid,
    readout:Trapezoid,
    spoiler:Trapezoid,
}

struct GradMatrices {
    diffusion:Matrix,
    phase_encode1:Matrix,
    phase_encode2:Matrix,
    readout:Matrix,
    spoiler:Matrix,
    rewind1: Matrix,
    rewind2: Matrix,
    phase_encode3: Matrix,
}

impl SpinEchoDW {

    // pub fn _25um() -> SpinEchoDWParams {
    //     SpinEchoDWParams {
    //         view_table:Path::new(r"C:\workstation\data\petableCS_stream\fse\stream_CS480_8x_pa18_pb54").to_owned(),
    //         fov:(19.7,12.0,12.0),
    //         samples:(788,480,480),
    //         sample_discards:0,
    //         spectral_width:SpectralWidth::SW200kH,
    //         rf_90_duration:140E-6,
    //         rf_180_duration:280E-6,
    //         diff_pulse_duration:3.5E-3,
    //         diff_pulse_separation:4E-3,
    //         spoil_duration:600E-6,
    //         ramp_time:140E-6,
    //         read_extension: 0.0,
    //         phase_encode_time:550E-6,
    //         echo_time:13.98E-3,
    //         echo_spacing:7.2E-3,
    //         obs_freq_offset: 0.0,
    //         rep_time:80E-3,
    //         n_averages: 1,
    //         n_repetitions: 28800,
    //         setup_mode:false,
    //         grad_off:false
    //     }
    // }
    //
    // pub fn _45um() -> SpinEchoDWParams {
    //     SpinEchoDWParams {
    //         view_table:Path::new(r"C:\workstation\data\petableCS_stream\fse\stream_CS256_8x_pa18_pb54").to_owned(),
    //         fov:(19.7,12.0,12.0),
    //         samples:(420,256,256),
    //         sample_discards:0,
    //         spectral_width:SpectralWidth::SW133kH,
    //         rf_90_duration:140E-6,
    //         rf_180_duration:280E-6,
    //         diff_pulse_duration:4.28E-3,
    //         diff_pulse_separation: 5.0E-3,
    //         spoil_duration:600E-6,
    //         ramp_time:100E-6,
    //         read_extension: 0.0,
    //         phase_encode_time:600E-6,
    //         echo_time:14.13E-3,
    //         echo_spacing: 7.2E-3,
    //         obs_freq_offset: 0.0,
    //         rep_time:100.0E-3,
    //         n_averages: 1,
    //         n_repetitions: 28800,
    //         setup_mode:false,
    //         grad_off:false
    //     }
    // }

    pub fn new(params: SpinEchoDWParams) -> SpinEchoDW {
        let events = Self::events(&params);
        Self {
            events,
            params
        }
    }

    fn waveforms(params:&SpinEchoDWParams) -> Waveforms {
        let n_read = params.samples.0;
        let read_sample_time_sec = params.spectral_width.sample_time(n_read + params.sample_discards) + params.read_extension;
        let excitation = Hardpulse::new(params.rf_90_duration);
        let refocus = CompositeHardpulse::new_180(params.rf_180_duration);
        let readout = Trapezoid::new(params.ramp_time,read_sample_time_sec);
        let diffusion = HalfSin::new(params.diff_pulse_duration);
        let phase_encode = Trapezoid::new(params.ramp_time,params.phase_encode_time);
        let spoiler = Trapezoid::new(params.ramp_time,params.spoil_duration);
        Waveforms {
            excitation,
            diffusion,
            refocus,
            phase_encode,
            readout,
            spoiler
        }
    }

    fn gradient_matrices(params:&SpinEchoDWParams) -> GradMatrices {

        let waveforms = Self::waveforms(params);
        let mat_count = Matrix::new_tracker();
        let n_read = params.samples.0;
        let n_discards = params.sample_discards;
        let fov_read = params.fov.0;
        let non_adjustable = (false,false,false);

        /* READOUT */
        let read_sample_time_sec = params.spectral_width.sample_time(n_read + n_discards);
        let read_grad_dac = params.spectral_width.fov_to_dac(fov_read);
        let readout = Matrix::new_static("read_mat",DacValues::new(Some(read_grad_dac),None,None),non_adjustable,params.grad_off,&mat_count);

        /* PHASE ENCODING */
        let lut = vec![240;230400];
        let phase_encode_strategy = EncodeStrategy::LUT(Dimension::_3D,lut);

        let pe_driver1 = MatrixDriver::new(DriverVar::Repetition,MatrixDriverType::PhaseEncode(phase_encode_strategy.clone()),Some(0));
        let pe_driver2 = MatrixDriver::new(DriverVar::Repetition,MatrixDriverType::PhaseEncode(phase_encode_strategy.clone()),Some(1));
        let pe_driver3 = MatrixDriver::new(DriverVar::Repetition,MatrixDriverType::PhaseEncode(phase_encode_strategy),Some(1));
        let read_pre_phase_dac = waveforms.phase_encode.magnitude_net(0.5*waveforms.readout.power_net(read_grad_dac as f32)) as i16;
        let (phase_grad_step,slice_grad_step) = match params.setup_mode {
            false => {
                let phase_grad_step = waveforms.phase_encode.magnitude_net(1.0/params.fov.1);
                let slice_grad_step = waveforms.phase_encode.magnitude_net(1.0/params.fov.2);
                (phase_grad_step,slice_grad_step)
            }
            true => (0.0,0.0)
        };
        let phase_multiplier = grad_cal::grad_to_dac(phase_grad_step) as f32;
        let slice_multiplier = grad_cal::grad_to_dac(slice_grad_step) as f32;
        let transform = LinTransform::new((None,Some(phase_multiplier),Some(slice_multiplier)),(None,None,None));
        let static_dac_vals = DacValues::new(Some(-read_pre_phase_dac),None,None);
        let phase_encode1 = Matrix::new_driven(
            "c_pe_mat1",
            pe_driver1,
            transform,
            DacValues::new(Some(-read_pre_phase_dac),None,None),
            (true,false,false),
            params.grad_off,
            &mat_count
        );
        let phase_encode2 = Matrix::new_driven(
            "c_pe_mat2",
            pe_driver2,
            transform,
            DacValues::new(Some(-read_pre_phase_dac),None,None),
            (true,false,false),
            params.grad_off,
            &mat_count
        );

        let phase_encode3 = Matrix::new_driven(
            "c_pe_mat3",
            pe_driver3,
            transform,
            DacValues::new(Some(-read_pre_phase_dac),None,None),
            (true,false,false),
            params.grad_off,
            &mat_count
        );

        /* REWINDER */
        let rewind1 = Matrix::new_derived(
            "rewind_mat1",
            &Rc::new(phase_encode1.clone()),
            LinTransform::new((Some(1.0),Some(-1.0),Some(-1.0)),(Some(0),Some(0),Some(0))),
            (true,false,false),
            params.grad_off,
            &mat_count
        );

        let rewind2 = Matrix::new_derived(
            "rewind_mat2",
            &Rc::new(phase_encode2.clone()),
            LinTransform::new((Some(1.0),Some(-1.0),Some(-1.0)),(Some(0),Some(0),Some(0))),
            (true,false,false),
            params.grad_off,
            &mat_count
        );

        /* DIFFUSION */
        let diffusion = match params.setup_mode {
            true => Matrix::new_static("diffusion_mat",DacValues::new(Some(500),None,None),(true,false,false),params.grad_off,&mat_count),
            false => Matrix::new_static("diffusion_mat",DacValues::new(Some(0),Some(0),Some(0)),(true,false,false),params.grad_off,&mat_count),
        };

        /* SPOILER */
        let spoiler = Matrix::new_static("spoiler_mat",DacValues::new(Some(read_grad_dac),Some(read_grad_dac),Some(read_grad_dac)),non_adjustable,params.grad_off,&mat_count);


        GradMatrices {
            diffusion,
            phase_encode1,
            rewind1,
            rewind2,
            phase_encode2,
            phase_encode3,
            readout,
            spoiler
        }

    }

    fn events(params: &SpinEchoDWParams) -> SpinEchoDWEvents {
        let w = Self::waveforms(params);
        let m = Self::gradient_matrices(params);

        let excitation = RfEvent::new(
            "excitation",
            1,
            w.excitation,
            RfStateType::Adjustable(400,None),
            RfStateType::Static(0)
        );

        let refocus1 = RfEvent::new(
            "refocus1",
            2,
            w.refocus.clone(),
            RfStateType::Adjustable(800,None),
            RfStateType::Adjustable(400,Some(PhaseCycleStrategy::CycleCPMG(2))),
            //RfStateType::Driven(RfDriver::new(DriverVar::Repetition,RfDriverType::PhaseCycle3D(PhaseCycleStrategy::CycleCPMG(1)),None)),
        );

        let refocus2 = RfEvent::new(
            "refocus2",
            3,
            w.refocus.clone(),
            RfStateType::Adjustable(800,None),
            RfStateType::Adjustable(120,None),
            //RfStateType::Driven(RfDriver::new(DriverVar::Repetition,RfDriverType::PhaseCycle3D(PhaseCycleStrategy::CycleCPMG(1)),None)),
        );

        let refocus3 = RfEvent::new(
            "refocus3",
            4,
            w.refocus.clone(),
            RfStateType::Adjustable(800,None),
            RfStateType::Adjustable(80,None),
            //RfStateType::Driven(RfDriver::new(DriverVar::Repetition,RfDriverType::PhaseCycle3D(PhaseCycleStrategy::CycleCPMG(1)),None)),
        );

        let phase_encode1 = GradEvent::new(
            (Some(w.phase_encode),Some(w.phase_encode),Some(w.phase_encode)),
            &m.phase_encode1,
            GradEventType::Blocking,
            "phase_encode1"
        );

        let phase_encode2 = GradEvent::new(
            (Some(w.phase_encode),Some(w.phase_encode),Some(w.phase_encode)),
            &m.phase_encode2,
            GradEventType::Blocking,
            "phase_encode2"
        );

        let phase_encode3 = GradEvent::new(
            (Some(w.phase_encode),Some(w.phase_encode),Some(w.phase_encode)),
            &m.phase_encode3,
            GradEventType::Blocking,
            "phase_encode3"
        );

        let rewind1 = GradEvent::new(
            (Some(w.phase_encode),Some(w.phase_encode),Some(w.phase_encode)),
            &m.rewind1,
            GradEventType::Blocking,
            "rewind1"
        );

        let rewind2 = GradEvent::new(
            (Some(w.phase_encode),Some(w.phase_encode),Some(w.phase_encode)),
            &m.rewind2,
            GradEventType::Blocking,
            "rewind2"
        );

        let readout = GradEvent::new(
            (Some(w.readout),None,None),
            &m.readout,
            GradEventType::NonBlocking,
            "readout"
        );

        let diffusion = GradEvent::new(
            (Some(w.diffusion),Some(w.diffusion),Some(w.diffusion)),
            &m.diffusion,
            GradEventType::Blocking,
            "diffusion"
        );

        let acquire = AcqEvent::new(
            "acquire",
            params.spectral_width.clone(),
            params.samples.0,
            params.sample_discards,
            RfStateType::Static(0)
        );

        let spoiler = GradEvent::new(
            (Some(w.spoiler),Some(w.spoiler),Some(w.spoiler)),
            &m.spoiler,
            GradEventType::Blocking,
            "spoiler"
        );

        SpinEchoDWEvents {
            excitation,
            diffusion,
            refocus1,
            refocus2,
            refocus3,
            phase_encode1,
            phase_encode2,
            phase_encode3,
            rewind1,
            rewind2,
            readout,
            acquire,
            spoiler,
        }

    }


    fn place_events(&self) -> EventQueue {

        let te = self.params.echo_time;
        let te2 = self.params.echo_spacing;
        let tau = self.params.echo_time/2.0;
        let tau2 = (te + te2 + te)/2.0;

        let adj = 100E-6;

        let excitation = Event::new(self.events.excitation.as_reference(),Origin);
        let refocus1 = Event::new(self.events.refocus1.as_reference(),ExactFromOrigin(sec_to_clock(tau)));
        let readout1 = Event::new(self.events.readout.as_reference(),ExactFromOrigin(sec_to_clock(te)));
        let acquire1 = Event::new(self.events.acquire.as_reference(),ExactFromOrigin(sec_to_clock(te-38E-6)));

        let refocus2 = Event::new(self.events.refocus2.as_reference(),ExactFromOrigin(sec_to_clock(tau2 + adj)));
        let readout2 = Event::new(self.events.readout.as_reference(),ExactFromOrigin(sec_to_clock(te + 1.0*te2)));
        let acquire2 = Event::new(self.events.acquire.as_reference(),ExactFromOrigin(sec_to_clock(te + 1.0*te2 - 38E-6)));

        let refocus3 = Event::new(self.events.refocus3.as_reference(),ExactFromOrigin(sec_to_clock(te + 2.0*te2 - te2/2.0 + adj)));
        let readout3 = Event::new(self.events.readout.as_reference(),ExactFromOrigin(sec_to_clock(te + 2.0*te2)));
        let acquire3 = Event::new(self.events.acquire.as_reference(),ExactFromOrigin(sec_to_clock(te + 2.0*te2 - 38E-6)));

        let phase_encode1 = Event::new(self.events.phase_encode1.as_reference(),Before(readout1.clone(),0));
        let phase_encode2 = Event::new(self.events.phase_encode2.as_reference(),Before(readout2.clone(),0));
        let phase_encode3 = Event::new(self.events.phase_encode3.as_reference(),Before(readout3.clone(),0));

        let rewind1 = Event::new(self.events.rewind1.as_reference(),After(acquire1.clone(),0));
        let rewind2 = Event::new(self.events.rewind2.as_reference(),After(acquire2.clone(),0));

        let spoiler = Event::new(self.events.spoiler.as_reference(),After(acquire3.clone(),0));

        let diffusion2 = Event::new(self.events.diffusion.as_reference(),Before(phase_encode1.clone(),0));
        let c2 = diffusion2.borrow().center();
        let sep = sec_to_clock(self.params.diff_pulse_separation);
        let c1 = c2 - sep;
        let diffusion1 = Event::new(self.events.diffusion.as_reference(),ExactFromOrigin(c1));
        EventQueue::new(
            &vec![
                excitation,
                diffusion1,
                refocus1,refocus2,refocus3,
                diffusion2,
                phase_encode1,phase_encode2,phase_encode3,
                rewind1,rewind2,
                readout1,readout2,readout3,
                acquire1,acquire2,acquire3,
                spoiler,
            ]
        )
    }

}
