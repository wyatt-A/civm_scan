use std::path::{Path, PathBuf};
use std::fs::{File};
use std::io::{Read, Write};
use seq_tools::grad_cal;
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
use seq_tools::_utils::{sec_to_clock};
use crate::pulse_sequence::{Build, PPLBaseParams, SequenceParameters, Setup, DiffusionWeighted, DiffusionPulseShape, CompressedSense, b_val_to_dac, Simulate, AcqDimensions, AcqDims, Initialize, DWSequenceParameters, MrdToKspace, MrdToKspaceParams, MrdFormat, SequenceLoadError, UseAdjustments};
use serde_json;
use serde::{Serialize,Deserialize};
use cs_table::cs_table::CSTable;
use headfile::headfile::{DWHeadfile, DWHeadfileParams, AcqHeadfile, AcqHeadfileParams};

impl Setup for SeDtiParams {
    fn set_mode(&mut self) {
        self.setup_mode = true;
    }
    fn set_repetitions(&mut self) {
        self.n_repetitions = 2000;
    }
}

impl DiffusionWeighted for SeDtiParams {
    fn b_value(&self) -> f32 {
        self.b_value
    }
    fn set_b_value(&mut self, b_value: f32) {
        self.b_value = b_value;
    }
    fn b_vec(&self) -> (f32, f32, f32) {
        self.b_vec.clone()
    }
    fn set_b_vec(&mut self, b_vec: (f32, f32, f32)) {
        self.b_vec = b_vec;
    }
    fn pulse_shape(&self) -> DiffusionPulseShape {
        DiffusionPulseShape::HalfSin
    }
    fn pulse_separation(&self) -> f32 {
        self.diff_pulse_separation
    }
    fn pulse_duration(&self) -> f32 {
        self.diff_pulse_duration
    }
}

impl CompressedSense for SeDtiParams {
    fn is_cs(&self) -> bool {
        true
    }
    fn set_cs_table(&mut self) {
        let n_reps = CSTable::open(
            &self.cs_table().unwrap(),
            self.samples.1 as i16,
            self.samples.2 as i16,
        ).n_views() as u32/self.view_acceleration as u32;
        self.n_repetitions = n_reps;
    }
    fn cs_table(&self) -> Option<PathBuf> {
        Some(self.cs_table.clone())
    }
}


impl Simulate for SeDtiParams {
    fn set_sim_repetitions(&mut self) {
        self.n_repetitions = 2;
    }
}

impl AcqDimensions for SeDtiParams {
    fn acq_dims(&self) -> AcqDims {
        AcqDims {
            n_read: self.samples.0 as i32,
            n_phase1: self.samples.1 as i32,
            n_phase2: self.samples.2 as i32,
            n_slices: 1,
            n_echos: 1,
            n_experiments: 1
        }
    }
}

impl AcqHeadfile for SeDtiParams {
    fn acq_params(&self) -> AcqHeadfileParams {
        AcqHeadfileParams {
            dim_x: self.samples.0 as i32,
            dim_y: self.samples.1 as i32,
            dim_z: self.samples.2 as i32,
            fovx_mm: self.fov.0,
            fovy_mm: self.fov.1,
            fovz_mm: self.fov.2,
            te_ms: 1E3*self.echo_time,
            tr_us: 1E6*self.rep_time,
            alpha: 90.0,
            bw: self.spectral_width.hertz() as f32 /2.0,
            n_echos: 1,
            S_PSDname: self.name()
        }
    }
}

impl DWHeadfile for SeDtiParams {
    fn diffusion_params(&self) -> DWHeadfileParams {
        DWHeadfileParams {
            bvalue: self.b_value,
            bval_dir: self.b_vec,
        }
    }
}

impl Initialize for SeDtiParams {
    fn default() -> Self {
        SeDtiParams {
            name: "se_dti".to_string(),
            cs_table: Path::new(r"C:\workstation\data\petableCS_stream\stream_CS480_8x_pa18_pb54").to_owned(),
            b_value: 3000.0,
            b_vec: (1.0, 0.0, 0.0),
            fov: (19.7, 12.0, 12.0),
            samples: (788, 480, 480),
            sample_discards: 0,
            spectral_width: SpectralWidth::SW200kH,
            rf_90_duration: 140E-6,
            rf_180_duration: 280E-6,
            diff_pulse_duration: 3.5E-3,
            diff_pulse_separation: 5E-3,
            spoil_duration: 600E-6,
            ramp_time: 140E-6,
            read_extension: 0.0,
            phase_encode_time: 550E-6,
            echo_time: 13.98E-3,
            rep_time: 80E-3,
            n_averages: 1,
            n_repetitions: 2000,
            view_acceleration : 1,
            setup_mode: false,
            grad_off: false,
            adjustment_file: None,
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

impl DWSequenceParameters for SeDtiParams{}

impl MrdToKspace for SeDtiParams {
    fn mrd_to_kspace_params(&self) -> MrdToKspaceParams {
        let table_compression = 8;
        let n_views = (self.samples.1 as usize*self.samples.2 as usize)/table_compression;
        MrdToKspaceParams {
            mrd_format:MrdFormat::StandardCSVol,
            n_read: self.samples.0 as usize,
            n_phase1: self.samples.1 as usize,
            n_phase2: self.samples.2 as usize,
            n_views,
            view_acceleration: self.view_acceleration as usize,
            dummy_excitations: 0,
            n_objects: 1
        }
    }
}

impl UseAdjustments for SeDtiParams {
    fn set_adjustment_file(&mut self, adj_file: &Path) {
        self.adjustment_file = Some(adj_file.to_owned());
    }
    fn adjustment_file(&self) -> Option<PathBuf> {
        self.adjustment_file.clone()
    }
}

impl SequenceParameters for SeDtiParams {

    fn name(&self) -> String {
        String::from("se_dti")
    }
    fn write(&self,params_file: &Path){
        let str = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        let mut f = File::create(params_file).expect("cannot create file");
        f.write_all(str.as_bytes()).expect("trouble writing to file");
    }
    fn instantiate(&self) -> Box<dyn Build> {
        Box::new(SeDti::new(self.clone()))
    }
}


impl Build for SeDti {
    fn place_events(&self) -> EventQueue {
        self.place_events()
    }
    fn base_params(&self) -> PPLBaseParams {
        PPLBaseParams {
            n_averages: self.params.n_averages,
            n_repetitions: self.params.n_repetitions,
            rep_time: self.params.rep_time,
            base_frequency: BaseFrequency::civm9p4t(self.params.obs_freq_offset().unwrap_or(0.0)),
            orientation: Orientation::CivmStandard,
            grad_clock: GradClock::CPS20,
            phase_unit: PhaseUnit::Min,
            view_acceleration: self.params.view_acceleration,
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
pub struct SeDtiParams {
    name: String,
    cs_table: PathBuf,
    b_value: f32,
    b_vec: (f32, f32, f32),
    fov: (f32, f32, f32),
    samples: (u16, u16, u16),
    sample_discards: u16,
    spectral_width: SpectralWidth,
    rf_90_duration: f32,
    rf_180_duration: f32,
    diff_pulse_duration: f32,
    diff_pulse_separation: f32,
    spoil_duration: f32,
    ramp_time: f32,
    read_extension: f32,
    phase_encode_time: f32,
    echo_time: f32,
    rep_time: f32,
    n_averages: u16,
    n_repetitions: u32,
    view_acceleration : u16,
    setup_mode: bool,
    grad_off: bool,
    adjustment_file:Option<PathBuf>,
}

#[derive(Clone)]
pub struct SeDti {
    params: SeDtiParams,
    events: SeDtiEvents,
}

#[derive(Clone)]
pub struct SeDtiEvents {
    excitation: RfEvent<Hardpulse>,
    diffusion1: GradEvent<HalfSin>,
    diffusion2: GradEvent<HalfSin>,
    refocus1: RfEvent<CompositeHardpulse>,
    phase_encode1: GradEvent<Trapezoid>,
    readout: GradEvent<Trapezoid>,
    acquire: AcqEvent,
    spoiler: GradEvent<Trapezoid>,
}

struct Waveforms {
    excitation: Hardpulse,
    diffusion: HalfSin,
    refocus: CompositeHardpulse,
    phase_encode: Trapezoid,
    readout: Trapezoid,
    spoiler: Trapezoid,
}

struct GradMatrices {
    diffusion1: Matrix,
    diffusion2: Matrix,
    phase_encode1: Matrix,
    readout: Matrix,
    spoiler: Matrix,
}

impl SeDti {

    pub fn new(params: SeDtiParams) -> SeDti {
        let events = Self::events(&params);
        Self {
            events,
            params
        }
    }

    fn waveforms(params: &SeDtiParams) -> Waveforms {
        let n_read = params.samples.0;
        let read_sample_time_sec = params.spectral_width.sample_time(n_read + params.sample_discards) + params.read_extension;
        let excitation = Hardpulse::new(params.rf_90_duration);
        let refocus = CompositeHardpulse::new_180(params.rf_180_duration);
        let readout = Trapezoid::new(params.ramp_time, read_sample_time_sec);
        let diffusion = HalfSin::new(params.diff_pulse_duration);
        let phase_encode = Trapezoid::new(params.ramp_time, params.phase_encode_time);
        let spoiler = Trapezoid::new(params.ramp_time, params.spoil_duration);
        Waveforms {
            excitation,
            diffusion,
            refocus,
            phase_encode,
            readout,
            spoiler
        }
    }

    fn gradient_matrices(params: &SeDtiParams) -> GradMatrices {
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
        let lut = vec![240; 230400];
        let phase_encode_strategy = EncodeStrategy::LUT(Dimension::_3D, lut);

        let pe_driver1 = MatrixDriver::new(DriverVar::Repetition, MatrixDriverType::PhaseEncode(phase_encode_strategy.clone()), Some(0));
        let pe_driver2 = MatrixDriver::new(DriverVar::Repetition, MatrixDriverType::PhaseEncode(phase_encode_strategy.clone()), Some(1));
        let pe_driver3 = MatrixDriver::new(DriverVar::Repetition, MatrixDriverType::PhaseEncode(phase_encode_strategy), Some(1));
        let read_pre_phase_dac = waveforms.phase_encode.magnitude_net(0.5 * waveforms.readout.power_net(read_grad_dac as f32)) as i16;
        let (phase_grad_step, slice_grad_step) = match params.setup_mode {
            false => {
                let phase_grad_step = waveforms.phase_encode.magnitude_net(1.0 / params.fov.1);
                let slice_grad_step = waveforms.phase_encode.magnitude_net(1.0 / params.fov.2);
                (phase_grad_step, slice_grad_step)
            }
            true => (0.0, 0.0)
        };
        let phase_multiplier = grad_cal::grad_to_dac(phase_grad_step) as f32;
        let slice_multiplier = grad_cal::grad_to_dac(slice_grad_step) as f32;
        let transform = LinTransform::new((None, Some(phase_multiplier), Some(slice_multiplier)), (None, None, None));
        let static_dac_vals = DacValues::new(Some(-read_pre_phase_dac), None, None);
        let phase_encode1 = Matrix::new_driven(
            "c_pe_mat1",
            pe_driver1,
            transform,
            DacValues::new(Some(-read_pre_phase_dac), None, None),
            (true, false, false),
            params.grad_off,
            &mat_count
        );

        /* DIFFUSION */
        let (diffusion1,diffusion2) = match params.setup_mode {
            true =>{
                println!("SETUP MODE ON");
                let diff_dacs = b_val_to_dac(DiffusionPulseShape::HalfSin,0.0,params.diff_pulse_duration,params.diff_pulse_separation,(1.0,0.0,0.0));
                (Matrix::new_static("diffusion_mat1", DacValues::new(Some(diff_dacs.0), None, None), (true, true, true), params.grad_off, &mat_count),
                 Matrix::new_static("diffusion_mat2", DacValues::new(Some(diff_dacs.0), None, None), (true, true, true), params.grad_off, &mat_count))
            },
            false =>{
                let diff_dacs = b_val_to_dac(DiffusionPulseShape::HalfSin,params.b_value,params.diff_pulse_duration,params.diff_pulse_separation,params.b_vec);

                // the magnitude of the second diffusion pulse needs to be reduced to avoid a shift in echo location
                let r_corrected = (diff_dacs.0 as f32 * 0.99905).round() as i16;
                let p_corrected = (diff_dacs.1 as f32 * 0.99780).round() as i16;
                let s_corrected = (diff_dacs.2 as f32 * 0.99975).round() as i16;

                (Matrix::new_static("diffusion_mat1", DacValues::new(Some(diff_dacs.0), Some(diff_dacs.1), Some(diff_dacs.2)), (false, false, false), params.grad_off, &mat_count),
                 Matrix::new_static("diffusion_mat2", DacValues::new(Some(r_corrected), Some(p_corrected), Some(s_corrected)), (false, false, false), params.grad_off, &mat_count))
            }
        };

        /* SPOILER */
        let spoiler = Matrix::new_static("spoiler_mat", DacValues::new(Some(read_grad_dac), Some(read_grad_dac), Some(read_grad_dac)), non_adjustable, params.grad_off, &mat_count);

        GradMatrices {
            diffusion1,
            diffusion2,
            phase_encode1,
            readout,
            spoiler
        }
    }

    fn events(params: &SeDtiParams) -> SeDtiEvents {
        let w = Self::waveforms(params);
        let m = Self::gradient_matrices(params);

        let excitation_dac = params.rf_dac(90.0,Box::new(w.excitation.clone())).unwrap_or(400);
        let excitation = RfEvent::new(
            "excitation",
            1,
            w.excitation,
            RfStateType::Adjustable(excitation_dac, None),
            RfStateType::Static(0)
        );

        let refocus_dac = params.rf_dac(180.0,Box::new(w.refocus.clone())).unwrap_or(800);
        let refocus1 = RfEvent::new(
            "refocus1",
            2,
            w.refocus.clone(),
            RfStateType::Adjustable(refocus_dac, None),
            RfStateType::Adjustable(0, Some(PhaseCycleStrategy::CycleCPMG(2))),
            //RfStateType::Driven(RfDriver::new(DriverVar::Repetition,RfDriverType::PhaseCycle3D(PhaseCycleStrategy::CycleCPMG(1)),None)),
        );

        let phase_encode1 = GradEvent::new(
            (Some(w.phase_encode), Some(w.phase_encode), Some(w.phase_encode)),
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

        let diffusion1 = GradEvent::new(
            (Some(w.diffusion), Some(w.diffusion), Some(w.diffusion)),
            &m.diffusion1,
            GradEventType::Blocking,
            "diffusion1"
        );

        let diffusion2 = GradEvent::new(
            (Some(w.diffusion), Some(w.diffusion), Some(w.diffusion)),
            &m.diffusion2,
            GradEventType::Blocking,
            "diffusion2"
        );

        let acquire = AcqEvent::new(
            "acquire",
            params.spectral_width.clone(),
            params.samples.0,
            params.sample_discards,
            RfStateType::Static(0)
        );

        let spoiler = GradEvent::new(
            (Some(w.spoiler), Some(w.spoiler), Some(w.spoiler)),
            &m.spoiler,
            GradEventType::Blocking,
            "spoiler"
        );

        SeDtiEvents {
            excitation,
            diffusion1,
            diffusion2,
            refocus1,
            phase_encode1,
            readout,
            acquire,
            spoiler,
        }
    }


    fn place_events(&self) -> EventQueue {
        let te = self.params.echo_time;
        let tau = self.params.echo_time / 2.0;

        let excitation = Event::new(self.events.excitation.as_reference(), Origin);
        let refocus1 = Event::new(self.events.refocus1.as_reference(), ExactFromOrigin(sec_to_clock(tau)));
        let readout1 = Event::new(self.events.readout.as_reference(), ExactFromOrigin(sec_to_clock(te)));
        let acquire1 = Event::new(self.events.acquire.as_reference(), ExactFromOrigin(sec_to_clock(te - 38E-6)));

        let phase_encode1 = Event::new(self.events.phase_encode1.as_reference(), Before(readout1.clone(), 0));

        let spoiler = Event::new(self.events.spoiler.as_reference(), After(acquire1.clone(), 0));

        let diffusion2 = Event::new(self.events.diffusion2.as_reference(), Before(phase_encode1.clone(), 0));
        let c2 = diffusion2.borrow().center();
        let sep = sec_to_clock(self.params.diff_pulse_separation);
        let c1 = c2 - sep;
        let diffusion1 = Event::new(self.events.diffusion1.as_reference(), ExactFromOrigin(c1));

        EventQueue::new(
            &vec![
                excitation,
                diffusion1,
                refocus1,
                diffusion2,
                phase_encode1,
                readout1,
                acquire1,
                spoiler,
            ]
        )
    }
}
