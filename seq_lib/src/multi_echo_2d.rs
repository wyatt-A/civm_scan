use std::cell::RefCell;
use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::fs::{File};
use std::io::{Read, Write};
use seq_tools::{grad_cal, _utils};
use seq_tools::acq_event::{AcqEvent, SpectralWidth};
use seq_tools::event_block::{Event, EventQueue, GradEventType};
use seq_tools::event_block::EventPlacementType::{Before, ExactFromOrigin, Origin};
use seq_tools::execution::ExecutionBlock;
use seq_tools::gradient_event::GradEvent;
use seq_tools::gradient_matrix::{DacValues, Dimension, DriverVar, EncodeStrategy, LinTransform, Matrix, MatrixDriver, MatrixDriverType};
use seq_tools::pulse::{Pulse, SincPulse, SliceSelective, Trapezoid};
use seq_tools::rf_event::RfEvent;
use seq_tools::rf_state::{RfStateType};
use crate::pulse_sequence::{Build, PPLBaseParams, SequenceParameters, Setup, CompressedSense, Simulate, AcqDimensions, AcqDims, Initialize, MrdToKspace, MrdToKspaceParams, MrdFormat, SequenceLoadError, UseAdjustments};
use serde_json;
use serde::{Serialize,Deserialize};
use headfile::headfile::{AcqHeadfile, AcqHeadfileParams};
use seq_tools::ppl::{GradClock, Orientation, PhaseUnit};
use seq_tools::ppl_header::BaseFrequency;

impl Simulate for Me2DParams {
    fn set_sim_repetitions(&mut self) {
        self.n_repetitions = Some(2);
    }
}

impl AcqDimensions for Me2DParams {
    fn acq_dims(&self) -> AcqDims {
        AcqDims {
            n_read: self.samples.0 as i32,
            n_phase1: self.samples.1 as i32,
            n_phase2: 1,
            n_slices: 1,
            n_echos: self.n_echos as i32,
            n_experiments: 1
        }
    }
}

impl AcqHeadfile for Me2DParams {
    fn acq_params(&self) -> AcqHeadfileParams {
        AcqHeadfileParams {
            dim_x: self.samples.0 as i32,
            dim_y: self.samples.1 as i32,
            dim_z: 1,
            fovx_mm: self.fov.0,
            fovy_mm: self.fov.1,
            fovz_mm: self.slice_thickness,
            te_ms: 1E3*self.echo_time,
            tr_us: 1E6*self.rep_time,
            alpha: 90.0,
            bw: self.spectral_width.hertz() as f32 /2.0,
            n_echos: self.n_echos as i32,
            s_psdname: self.name()
        }
    }
}

impl Initialize for Me2DParams {
    fn default() -> Self {
        Me2DParams {
            name: "multi_echo_2d".to_string(),
            fov: (19.7, 12.0),
            samples: (210, 128),
            n_echos: 10,
            slice_thickness: 1.0,
            sample_discards: 0,
            orientation: Orientation::CivmStandard,
            spectral_width: SpectralWidth::SW100kH,
            rf_duration: 1E-3,
            excite_flip_angle: 90.0,
            refocus_flip_angle: 180.0,
            crush_duration: 500E-6,
            ramp_time: 140E-6,
            phase_encode_time: 550E-6,
            echo_time: 20E-3,
            rep_time: 500E-3,
            n_averages: 1,
            n_repetitions:None,
            grad_off: false,
            setup_mode : false,
            adjustment_file:None,
        }
    }
    fn load(params_file: &Path) -> Result<Self,SequenceLoadError> {
        let mut f = File::open(params_file).expect("cannot open file");
        let mut json_str = String::new();
        f.read_to_string(&mut json_str).expect("trouble reading file");
        match serde_json::from_str(&json_str) {
            Ok(params) => Ok(params),
            Err(_) => Err(SequenceLoadError::InvalidFormat)
        }
    }
    fn write_default(params_file: &Path){
        let params = Self::default();
        let str = serde_json::to_string_pretty(&params).expect("cannot serialize struct");
        let mut f = File::create(params_file).expect("cannot create file");
        f.write_all(str.as_bytes()).expect("trouble writing to file");
    }
}

impl MrdToKspace for Me2DParams {
    fn mrd_to_kspace_params(&self) -> MrdToKspaceParams {
        MrdToKspaceParams {
            mrd_format:MrdFormat::StandardSlice,
            n_read: self.samples.0 as usize,
            n_phase1: self.samples.1 as usize,
            n_phase2: 1,
            n_views: self.samples.1 as usize,
            view_acceleration: 1,
            dummy_excitations: 0,
            n_objects: self.n_echos as usize
        }
    }
}


impl CompressedSense for Me2DParams {
    fn is_cs(&self) -> bool {
        false
    }

    fn set_cs_table(&mut self) {
    }

    fn cs_table(&self) -> Option<PathBuf> {
        None
    }
}

impl Setup for Me2DParams {
    fn set_mode(&mut self) {
        self.setup_mode = true;
    }

    fn set_repetitions(&mut self) {
        self.n_repetitions = Some(2000);
    }
}

impl SequenceParameters for Me2DParams {

    fn name(&self) -> String {
        String::from("multi_echo_2d")
    }
    fn write(&self,params_file: &Path){
        let str = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        let mut f = File::create(params_file).expect("cannot create file");
        f.write_all(str.as_bytes()).expect("trouble writing to file");
    }
    fn instantiate(&self) -> Box<dyn Build> {
        Box::new(Me2D::new(self.clone()))
    }
}

impl UseAdjustments for Me2DParams {
    fn set_adjustment_file(&mut self, adj_file: &Path) {
        self.adjustment_file = Some(adj_file.to_owned());
    }

    fn adjustment_file(&self) -> Option<PathBuf> {
        self.adjustment_file.clone()
    }
}

impl Build for Me2D {
    fn place_events(&self) -> EventQueue {
        self.place_events()
    }
    fn base_params(&self) -> PPLBaseParams {
        PPLBaseParams {
            n_averages: self.params.n_averages,
            n_repetitions: self.params.n_repetitions.unwrap_or(self.params.samples.1 as u32),
            rep_time: self.params.rep_time,
            base_frequency: BaseFrequency::civm9p4t(self.params.obs_freq_offset().unwrap_or(0.0)),
            orientation: self.params.orientation.clone(),
            grad_clock: GradClock::CPS20,
            phase_unit: PhaseUnit::Min,
            view_acceleration: 1,
            waveform_sample_period_us: 2
        }
    }

    fn param_export(&self, filepath: &Path) {
        let params = self.params.clone();
        let name = params.name.clone();
        params.write(&filepath.join(name).with_extension("json"));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Me2DParams {
    name: String,
    fov: (f32, f32),
    samples: (u16, u16),
    n_echos:u16,
    slice_thickness:f32,
    sample_discards: u16,
    orientation:Orientation,
    spectral_width: SpectralWidth,
    rf_duration: f32,
    excite_flip_angle:f32,
    refocus_flip_angle:f32,
    crush_duration: f32,
    ramp_time: f32,
    phase_encode_time: f32,
    echo_time: f32,
    rep_time: f32,
    n_averages: u16,
    n_repetitions:Option<u32>,
    grad_off: bool,
    setup_mode: bool,
    adjustment_file:Option<PathBuf>,
}

#[derive(Clone)]
pub struct Me2D {
    params: Me2DParams,
    events: Me2DEvents,
}

#[derive(Clone)]
pub struct Me2DEvents {
    slice_sel: GradEvent<Trapezoid>,
    slice_ref: GradEvent<Trapezoid>,
    excitation: RfEvent<SincPulse>,
    refocus: RfEvent<SincPulse>,
    ref_slice_sel: GradEvent<Trapezoid>,
    phase_encode1: GradEvent<Trapezoid>,
    readout: GradEvent<Trapezoid>,
    acquire: AcqEvent,
    rewinder: GradEvent<Trapezoid>,
    crusher: GradEvent<Trapezoid>,
}

struct Waveforms {
    excitation: SincPulse,
    refocus: SincPulse,
    phase_encode: Trapezoid,
    readout: Trapezoid,
    slice_sel: Trapezoid,
    slice_ref: Trapezoid,
    ref_slice_sel: Trapezoid,
    crusher: Trapezoid,
}

struct GradMatrices {
    phase_encode1: Matrix,
    readout: Matrix,
    rewinder: Matrix,
    slice_sel: Matrix,
    slice_ref: Matrix,
    ref_slice_sel: Matrix,
    crusher: Matrix,
}

impl Me2D {

    pub fn new(params: Me2DParams) -> Me2D {
        let events = Self::events(&params);
        Self {
            events,
            params
        }
    }

    fn waveforms(params: &Me2DParams) -> Waveforms {
        let n_read = params.samples.0;
        let read_sample_time_sec = params.spectral_width.sample_time(n_read + params.sample_discards);
        let excitation = SincPulse::new(params.rf_duration,5);
        let refocus = SincPulse::new(params.rf_duration,5);
        let readout = Trapezoid::new(params.ramp_time, read_sample_time_sec);
        let phase_encode = Trapezoid::new(params.ramp_time, params.phase_encode_time);
        let slice_sel = Trapezoid::new(params.ramp_time,params.rf_duration);
        let slice_ref = Trapezoid::new(params.ramp_time,params.rf_duration);
        let ref_slice_sel = Trapezoid::new(params.ramp_time,params.rf_duration + params.crush_duration);
        let crusher = Trapezoid::new(params.ramp_time,0.75*params.phase_encode_time);

        Waveforms {
            excitation,
            refocus,
            phase_encode,
            readout,
            slice_sel,
            slice_ref,
            ref_slice_sel,
            crusher
        }
    }

    fn gradient_matrices(params: &Me2DParams) -> GradMatrices {
        let waveforms = Self::waveforms(params);
        let mat_count = Matrix::new_tracker();
        let fov_read = params.fov.0;
        let non_adjustable = (false, false, false);

        /* READOUT */
        let read_grad_dac = params.spectral_width.fov_to_dac(fov_read);
        let readout = Matrix::new_static("read_mat", DacValues::new(Some(read_grad_dac), None, None), non_adjustable, params.grad_off, &mat_count);

        /* PHASE ENCODING */
        let phase_encode_strategy = EncodeStrategy::FullySampled(Dimension::_2D,params.samples.1 as usize,None);
        let pe_driver1 = MatrixDriver::new(DriverVar::Repetition, MatrixDriverType::PhaseEncode(phase_encode_strategy.clone()), Some(0));

        //let read_pre_phase_dac = waveforms.phase_encode.magnitude_net(0.5 * waveforms.readout.power_net(read_grad_dac as f32)) as i16;
        let read_pre_phase_dac = waveforms.phase_encode.magnitude_net(0.5 * waveforms.slice_ref.power_net(read_grad_dac as f32)) as i16;


        let phase_grad_step = waveforms.phase_encode.magnitude_net(1.0 / params.fov.1);
        let phase_multiplier = grad_cal::grad_to_dac(phase_grad_step) as f32;
        let transform = LinTransform::new((None, Some(phase_multiplier), None), (None, None, None));


        let phase_encode1 = match params.setup_mode {
            true => {
                Matrix::new_static(
                    "c_pe_mat1",
                    DacValues::new(None, None, None),
                    (true, false, false),
                    true,
                    &mat_count
                )
            }
            false => {
                Matrix::new_driven(
                    "c_pe_mat1",
                    pe_driver1.clone(),
                    transform,
                    DacValues::new(None, None, None),
                    (true, false, false),
                    params.grad_off,
                    &mat_count
                )
            }
        };

        let re_trans = LinTransform::new((None, Some(-1.0), None), (None, None, None));
        let rewinder = phase_encode1.derive("c_re_mat",re_trans,(false, false, false),params.grad_off,&mat_count);


        let crusher = Matrix::new_static(
            "c_crush_mat1",
            DacValues::new(Some(1000), None, None),
            (true, false, false),
            params.grad_off,
            &mat_count
        );

        // disable phase encoding in setup mode
        // let (phase_encode1,phase_encode2) = match params.setup_mode {
        //     true => {
        //         let phase_encode1 = Matrix::new_static(
        //             "c_pe_mat1",
        //             DacValues::new(Some(-read_pre_phase_dac), None, None),
        //             (true, false, false),
        //             params.grad_off,
        //             &mat_count
        //         );
        //         let phase_encode2 = Matrix::new_driven(
        //             "c_pe_mat2",
        //             pe_driver1,
        //             transform,
        //             DacValues::new(None, None, None),
        //             (false, false, false),
        //             true,
        //             &mat_count
        //         );
        //         (phase_encode1,phase_encode2)
        //     }
        //     false => {
        //         let phase_encode1 = Matrix::new_driven(
        //             "c_pe_mat1",
        //             pe_driver1.clone(),
        //             transform,
        //             DacValues::new(Some(-read_pre_phase_dac), None, None),
        //             (true, false, false),
        //             params.grad_off,
        //             &mat_count
        //         );
        //
        //         let phase_encode2 = Matrix::new_driven(
        //             "c_pe_mat2",
        //             pe_driver1,
        //             transform,
        //             DacValues::new(None, None, None),
        //             (false, false, false),
        //             params.grad_off,
        //             &mat_count
        //         );
        //         (phase_encode1,phase_encode2)
        //     }
        // };




        let grad = waveforms.excitation.grad_strength_hzpmm(params.slice_thickness);

        let slice_dac = grad_cal::grad_to_dac(grad);


        let slice_sel = Matrix::new_static(
            "slice_sel_mat",
            DacValues::new(None,None,Some(slice_dac)),
            (false,false,false),
            params.grad_off,
            &mat_count
        );

        let slice_ref = Matrix::new_static(
            "slice_ref_mat",
            DacValues::new(Some(read_pre_phase_dac),None,Some(-slice_dac/2)),
            (false,false,false),
            params.grad_off,
            &mat_count
        );

        let ref_slice_sel = Matrix::new_static(
            "ref_slice_sel_mat",
            DacValues::new(None,None,Some(slice_dac)),
            (false,false,false),
            params.grad_off,
            &mat_count
        );

        GradMatrices {
            phase_encode1,
            readout,
            rewinder,
            slice_sel,
            slice_ref,
            ref_slice_sel,
            crusher,
        }
    }

    fn events(params: &Me2DParams) -> Me2DEvents {
        let w = Self::waveforms(params);
        let m = Self::gradient_matrices(params);


        let slice_sel = GradEvent::new(
            (None,None,Some(w.slice_sel)),
            &m.slice_sel,
            GradEventType::NonBlocking,
            "slice_sel"
        );

        let ref_slice_sel = GradEvent::new(
            (None,None,Some(w.ref_slice_sel)),
            &m.ref_slice_sel,
            GradEventType::NonBlocking,
            "ref_slice_sel"
        );

        let excite_dac = params.rf_dac(params.excite_flip_angle,Box::new(w.excitation.clone())).unwrap_or(400);
        let excitation = RfEvent::new(
            "excitation",
            1,
            w.excitation,
            RfStateType::Adjustable(excite_dac, None),
            RfStateType::Static(0)
        );

        let refocus_dac = params.rf_dac(params.refocus_flip_angle,Box::new(w.refocus.clone())).unwrap_or(800);
        let refocus = RfEvent::new(
            "refocus",
            2,
            w.refocus,
            RfStateType::Adjustable(refocus_dac, None),
            RfStateType::Adjustable(400,None)
        );

        let slice_ref = GradEvent::new(
            (Some(w.slice_ref),None,Some(w.slice_ref)),
            &m.slice_ref,
            GradEventType::NonBlocking,
            "slice_ref"
        );

        let phase_encode1 = GradEvent::new(
            (Some(w.phase_encode), Some(w.phase_encode), None),
            &m.phase_encode1,
            GradEventType::Blocking,
            "phase_encode1"
        );

        let readout = GradEvent::new(
            (Some(w.readout), None, None),
            &m.readout,
            GradEventType::NonBlocking,
            "readout"
        );

        let acquire = AcqEvent::new(
            "acquire",
            params.spectral_width.clone(),
            params.samples.0,
            params.sample_discards,
            RfStateType::Static(0)
        );

        let rewinder = GradEvent::new(
            (Some(w.phase_encode), Some(w.phase_encode),None),
            &m.rewinder,
            GradEventType::Blocking,
            "rewind"
        );

        let crusher = GradEvent::new(
            (Some(w.crusher), None,None),
            &m.crusher,
            GradEventType::Blocking,
            "crusher"
        );

        Me2DEvents {
            slice_sel,
            slice_ref,
            excitation,
            phase_encode1,
            readout,
            acquire,
            rewinder,
            refocus,
            ref_slice_sel,
            crusher
        }
    }


    fn place_events(&self) -> EventQueue {
        let te = self.params.echo_time;
        let tau = te/2.0;
        let tau_clocks = _utils::sec_to_clock(tau);
        let te_clocks = _utils::sec_to_clock(te);

        let sd = _utils::sec_to_clock(2.0*self.params.ramp_time + 1.1*self.params.rf_duration) as i32;

        let mut event_vector = Vec::<Rc<RefCell<Event>>>::new();


        let excitation = Event::new(self.events.excitation.as_reference(), Origin);
        let ss = Event::new(self.events.slice_sel.as_reference(), ExactFromOrigin(0 + 300));
        let ss_ref = Event::new(self.events.slice_ref.as_reference(), ExactFromOrigin(sd));
        event_vector.extend(
            vec![
                excitation,
                ss,
                ss_ref,
            ]
        );

        for echo in 1 as i32..(self.params.n_echos as i32+1) {

            let t_echo = echo*te_clocks;
            let t_tau = echo*te_clocks - tau_clocks;
            let midpoint2 = (echo*te_clocks + te_clocks - tau_clocks + t_echo)/2;


            let read = Event::new(self.events.readout.as_reference(),ExactFromOrigin(t_echo));
            event_vector.push(read.clone());
            let acq = Event::new(self.events.acquire.as_reference(),ExactFromOrigin(t_echo));
            event_vector.push(acq.clone());

            let pe = Event::new(self.events.phase_encode1.as_reference(),Before(read,200));

            let refoc = Event::new(self.events.refocus.as_reference(),ExactFromOrigin(t_tau));
            let slice_ref = Event::new(self.events.ref_slice_sel.as_reference(),ExactFromOrigin(t_tau));
            event_vector.push(refoc.clone());

            event_vector.push(Event::new(self.events.crusher.as_reference(),Before(pe.clone(),100)));

            event_vector.push(Event::new(self.events.crusher.as_reference(),Before(slice_ref.clone(),100)));

            event_vector.push(slice_ref.clone());



            event_vector.push(pe.clone());

            // if echo == 1{
            //     event_vector.push(Event::new(self.events.phase_encode1.as_reference(),ExactFromOrigin(midpoint1)));
            // }else {
            //     event_vector.push(Event::new(self.events.phase_encode1.as_reference(),ExactFromOrigin(midpoint1)));
            // }



            event_vector.push(Event::new(self.events.rewinder.as_reference(),ExactFromOrigin(midpoint2)));
        }

        EventQueue::new(
            &event_vector
        )
    }
}
