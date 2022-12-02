use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::fs::{File};
use std::io::{Read, Write};
use seq_tools::{grad_cal, utils};
use seq_tools::acq_event::{AcqEvent, SpectralWidth};
use seq_tools::event_block::{Event, EventQueue, GradEventType};
use seq_tools::event_block::EventPlacementType::{After, Before, ExactFromOrigin, Origin};
use seq_tools::execution::ExecutionBlock;
use seq_tools::gradient_event::GradEvent;
use seq_tools::gradient_matrix::{DacValues, Dimension, DriverVar, EncodeStrategy, LinTransform, Matrix, MatrixDriver, MatrixDriverType};
use seq_tools::ppl::{GradClock, Orientation, PhaseUnit,BaseFrequency};
use seq_tools::pulse::{CompositeHardpulse, HalfSin, Hardpulse, Pulse, Trapezoid};
use seq_tools::rf_event::RfEvent;
use seq_tools::rf_state::{PhaseCycleStrategy, RfStateType};
use seq_tools::utils::{sec_to_clock};
use crate::pulse_sequence::{Build, PPLBaseParams, SequenceParameters, Setup, DiffusionWeighted, DiffusionPulseShape, CompressedSense, b_val_to_dac, Simulate, AcqDimensions, AcqDims, Initialize, DWSequenceParameters, MrdToKspace, MrdToKspaceParams, MrdFormat, ScoutConfig};
use serde_json;
use serde::{Serialize,Deserialize};
use cs_table::cs_table::CSTable;
use headfile::headfile::{DWHeadfile, DWHeadfileParams, AcqHeadfile, AcqHeadfileParams};
use crate::pulse_sequence;

impl Simulate for ScoutParams {
    fn set_sim_repetitions(&mut self) {
        self.n_repetitions = 2;
    }
}

impl AcqDimensions for ScoutParams {
    fn acq_dims(&self) -> AcqDims {
        AcqDims {
            n_read: self.samples.0 as i32,
            n_phase1: self.samples.1 as i32,
            n_phase2: 1,
            n_slices: 1,
            n_echos: 1,
            n_experiments: 1
        }
    }
}

impl AcqHeadfile for ScoutParams {
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
            n_echos: 1,
            S_PSDname: self.name()
        }
    }
}

impl Initialize for ScoutParams {
    fn default() -> Self {
        ScoutParams {
            name: "scout".to_string(),
            fov: (19.7, 12.0),
            samples: (210, 128),
            slice_thickness: 1.0,
            sample_discards: 0,
            orientation: Orientation::Scout1,
            spectral_width: SpectralWidth::SW100kH,
            rf_duration: 140E-6,
            ramp_time: 140E-6,
            phase_encode_time: 550E-6,
            echo_time: 5E-3,
            obs_freq_offset: 0.0,
            rep_time: 50E-3,
            n_averages: 1,
            n_repetitions: 128,
            grad_off: false
        }
    }
    fn load(params_file: &Path) -> Self {
        let mut f = File::open(params_file).expect("cannot open file");
        let mut json_str = String::new();
        f.read_to_string(&mut json_str).expect("trouble reading file");
        serde_json::from_str(&json_str).expect("cannot deserialize string")
    }
    fn write_default(params_file: &Path){
        let params = Self::default();
        let str = serde_json::to_string_pretty(&params).expect("cannot serialize struct");
        let mut f = File::create(params_file).expect("cannot create file");
        f.write_all(str.as_bytes()).expect("trouble writing to file");
    }
}

impl MrdToKspace for ScoutParams {
    fn mrd_to_kspace_params(&self) -> MrdToKspaceParams {
        MrdToKspaceParams {
            mrd_format:MrdFormat::StandardSlice,
            n_read: self.samples.0 as usize,
            n_phase1: self.samples.1 as usize,
            n_phase2: 1,
            n_views: self.samples.1 as usize,
            view_acceleration: 1,
            dummy_excitations: 0,
            n_objects: 1
        }
    }
}

impl ScoutConfig for ScoutParams {
    fn set_orientation(&mut self, orient: &Orientation) {
        self.orientation = orient.clone();
    }

    fn set_fov(&mut self, fov: (f32, f32)) {
        self.fov = fov;
    }

    fn set_samples(&mut self, samples: (u16, u16)) {
        self.samples = samples;
    }
}

impl CompressedSense for ScoutParams {
    fn is_cs(&self) -> bool {
        false
    }

    fn set_cs_table(&mut self) {
    }

    fn cs_table(&self) -> Option<PathBuf> {
        None
    }
}

impl Setup for ScoutParams {
    fn set_mode(&mut self) {
    }

    fn set_repetitions(&mut self) {
    }
}

impl SequenceParameters for ScoutParams {

    fn name(&self) -> String {
        String::from("scout")
    }
    fn write(&self,params_file: &Path){
        let str = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        let mut f = File::create(params_file).expect("cannot create file");
        f.write_all(str.as_bytes()).expect("trouble writing to file");
    }
    fn instantiate(&self) -> Box<dyn Build> {
        Box::new(Scout::new(self.clone()))
    }
}

impl Build for Scout {
    fn place_events(&self) -> EventQueue {
        self.place_events()
    }
    fn base_params(&self) -> PPLBaseParams {
        PPLBaseParams {
            n_averages: self.params.n_averages,
            n_repetitions: self.params.samples.1 as u32,
            rep_time: self.params.rep_time,
            base_frequency: BaseFrequency::civm9p4t(0.0),
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
pub struct ScoutParams {
    name: String,
    fov: (f32, f32),
    samples: (u16, u16),
    slice_thickness:f32,
    sample_discards: u16,
    orientation:Orientation,
    spectral_width: SpectralWidth,
    rf_duration: f32,
    ramp_time: f32,
    phase_encode_time: f32,
    echo_time: f32,
    rep_time: f32,
    n_averages: u16,
    n_repetitions: u32,
    grad_off: bool,
    pub obs_freq_offset: f64,
}

#[derive(Clone)]
pub struct Scout {
    params: ScoutParams,
    events: ScoutEvents,
}

#[derive(Clone)]
pub struct ScoutEvents {
    slice_sel: GradEvent<Trapezoid>,
    slice_ref: GradEvent<Trapezoid>,
    excitation: RfEvent<Hardpulse>,
    phase_encode: GradEvent<Trapezoid>,
    readout: GradEvent<Trapezoid>,
    acquire: AcqEvent,
    rewinder: GradEvent<Trapezoid>,
}

struct Waveforms {
    excitation: Hardpulse,
    phase_encode: Trapezoid,
    readout: Trapezoid,
    slice_sel:Trapezoid,
    slice_ref:Trapezoid,
}

struct GradMatrices {
    phase_encode1: Matrix,
    readout: Matrix,
    rewinder: Matrix,
    slice_sel: Matrix,
    slice_ref: Matrix,
}

impl Scout {

    pub fn new(params: ScoutParams) -> Scout {
        let events = Self::events(&params);
        Self {
            events,
            params
        }
    }

    fn waveforms(params: &ScoutParams) -> Waveforms {
        let n_read = params.samples.0;
        let read_sample_time_sec = params.spectral_width.sample_time(n_read + params.sample_discards);
        let excitation = Hardpulse::new(params.rf_duration);
        let readout = Trapezoid::new(params.ramp_time, read_sample_time_sec);
        let phase_encode = Trapezoid::new(params.ramp_time, params.phase_encode_time);
        let slice_sel = Trapezoid::new(params.ramp_time,2.0*params.rf_duration);
        let slice_ref = Trapezoid::new(params.ramp_time,params.rf_duration);

        Waveforms {
            excitation,
            phase_encode,
            readout,
            slice_sel,
            slice_ref,
        }
    }

    fn gradient_matrices(params: &ScoutParams) -> GradMatrices {
        let waveforms = Self::waveforms(params);
        let mat_count = Matrix::new_tracker();
        let n_read = params.samples.0;
        let n_discards = params.sample_discards;
        let fov_read = params.fov.0;
        let non_adjustable = (false, false, false);

        /* READOUT */
        let read_sample_time_sec = params.spectral_width.sample_time(n_read + n_discards);
        let read_grad_dac = params.spectral_width.fov_to_dac(fov_read);
        let readout = Matrix::new_static("read_mat", DacValues::new(Some(read_grad_dac), None, None), non_adjustable, params.grad_off, &mat_count);

        /* PHASE ENCODING */
        let phase_encode_strategy = EncodeStrategy::FullySampled(Dimension::_2D,params.samples.1 as usize,None);
        let pe_driver1 = MatrixDriver::new(DriverVar::Repetition, MatrixDriverType::PhaseEncode(phase_encode_strategy.clone()), Some(0));
        let read_pre_phase_dac = waveforms.phase_encode.magnitude_net(0.5 * waveforms.readout.power_net(read_grad_dac as f32)) as i16;
        let phase_grad_step = waveforms.phase_encode.magnitude_net(1.0 / params.fov.1);
        let phase_multiplier = grad_cal::grad_to_dac(phase_grad_step) as f32;
        let transform = LinTransform::new((None, Some(phase_multiplier), None), (None, None, None));

        let phase_encode1 = Matrix::new_driven(
            "c_pe_mat1",
            pe_driver1,
            transform,
            DacValues::new(Some(-read_pre_phase_dac), None, None),
            (true, false, false),
            params.grad_off,
            &mat_count
        );

        let re_trans = LinTransform::new((None, Some(-1.0), None), (None, None, None));
        let rewinder = phase_encode1.derive("c_re_mat",re_trans,(false, false, false),false,&mat_count);


        let grad = 10.0*waveforms.excitation.bandwidth_hz()/params.slice_thickness;
        let slice_dac = grad_cal::grad_to_dac(grad);

        let slice_sel = Matrix::new_static(
            "slice_sel_mat",
            DacValues::new(None,None,Some(slice_dac)),
            (false,false,false),
            false,
            &mat_count
        );

        let slice_ref = slice_sel.derive(
            "slice_ref_mat",
            LinTransform::new((None,None,Some(-1.0)),(None,None,None)),
            (false,false,false),
            false,
            &mat_count
        );

        GradMatrices {
            phase_encode1,
            readout,
            rewinder,
            slice_sel,
            slice_ref,
        }
    }

    fn events(params: &ScoutParams) -> ScoutEvents {
        let w = Self::waveforms(params);
        let m = Self::gradient_matrices(params);


        let slice_sel = GradEvent::new(
            (None,None,Some(w.slice_sel)),
            &m.slice_sel,
            GradEventType::NonBlocking,
            "slice_sel"
        );


        let excitation = RfEvent::new(
            "excitation",
            1,
            w.excitation,
            RfStateType::Adjustable(100, None),
            RfStateType::Static(0)
        );

        let slice_ref = GradEvent::new(
            (None,None,Some(w.slice_ref)),
            &m.slice_ref,
            GradEventType::NonBlocking,
            "slice_ref"
        );

        let phase_encode = GradEvent::new(
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
            (None, Some(w.phase_encode),None),
            &m.rewinder,
            GradEventType::Blocking,
            "rewind"
        );

        ScoutEvents {
            slice_sel,
            slice_ref,
            excitation,
            phase_encode,
            readout,
            acquire,
            rewinder,
        }
    }


    fn place_events(&self) -> EventQueue {
        let te = self.params.echo_time;

        let sd = utils::sec_to_clock(2.0*self.params.ramp_time + 2.0*self.params.rf_duration) as u32;

        let excitation = Event::new(self.events.excitation.as_reference(), Origin);

        let slice_sel = Event::new(self.events.slice_sel.as_reference(),ExactFromOrigin(300));
        let slice_ref = Event::new(self.events.slice_ref.as_reference(),After(excitation.clone(),sd));

        let readout = Event::new(self.events.readout.as_reference(), ExactFromOrigin(sec_to_clock(te)));
        let acquire1 = Event::new(self.events.acquire.as_reference(), ExactFromOrigin(sec_to_clock(te)));
        let phase_encode1 = Event::new(self.events.phase_encode.as_reference(), Before(readout.clone(), 0));
        let rewinder = Event::new(self.events.rewinder.as_reference(), After(acquire1.clone(), 0));

        EventQueue::new(
            &vec![
                excitation,
                slice_sel,
                slice_ref,
                phase_encode1,
                readout,
                acquire1,
                rewinder,
            ]
        )
    }
}
