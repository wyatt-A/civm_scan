use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::fs::{File};
use std::io::{Read, Write};
use std::rc::Rc;
use seq_tools::{grad_cal};
use seq_tools::acq_event::{AcqEvent, SpectralWidth};
use seq_tools::event_block::{Event, EventQueue, GradEventType};
use seq_tools::event_block::EventPlacementType::{After, Before, ExactFromOrigin, Origin};
use seq_tools::execution::ExecutionBlock;
use seq_tools::gradient_event::GradEvent;
use seq_tools::gradient_matrix::{DacValues, Dimension, DriverVar, EncodeStrategy, LinTransform, Matrix, MatrixDriver, MatrixDriverType};
use seq_tools::pulse::{Hardpulse, Pulse, Trapezoid};
use seq_tools::rf_event::RfEvent;
use seq_tools::rf_state::{RfStateType};
use seq_tools::_utils::{sec_to_clock};
use crate::pulse_sequence::{Build, PPLBaseParams, SequenceParameters, Setup, CompressedSense, Simulate, AcqDimensions, AcqDims, Initialize, MrdToKspace, MrdToKspaceParams, MrdFormat, SequenceLoadError, UseAdjustments};
use serde_json;
use serde::{Serialize,Deserialize};
use cs_table::cs_table::CSTable;
use headfile::headfile::{AcqHeadfile, AcqHeadfileParams};
use seq_tools::ppl::{GradClock, Orientation, PhaseUnit};
use seq_tools::ppl_header::BaseFrequency;


const SEQUENCE_NAME:&str = "mgre";

impl Setup for MgreParams {
    fn set_mode(&mut self) {
        self.setup_mode = true;
    }
    fn set_repetitions(&mut self) {
        self.n_repetitions = 2000;
    }
}

impl CompressedSense for MgreParams {
    fn is_cs(&self) -> bool {
        match self.cs_table {
            Some(_) => true,
            None => false
        }
    }
    // set repetitions
    fn set_cs_table(&mut self) {
        self.n_repetitions = match &self.cs_table {
            Some(table) => {
                CSTable::open(&table).n_views() as u32
            },
            None => {
                self.samples.1 as u32 * self.samples.2 as u32
            }
        };
    }
    fn cs_table(&self) -> Option<PathBuf> {
        self.cs_table.clone()
    }
}
impl Simulate for MgreParams {
    fn set_sim_repetitions(&mut self) {
        self.n_repetitions = 2;
    }
}
impl AcqDimensions for MgreParams {
    fn acq_dims(&self) -> AcqDims {
        AcqDims {
            n_read: self.samples.0 as i32,
            n_phase1: self.samples.1 as i32,
            n_phase2: self.samples.2 as i32,
            n_slices: 1,
            n_echos: self.n_echos as i32,
            n_experiments: 1
        }
    }
}
impl AcqHeadfile for MgreParams {
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
            alpha: self.rf_alpha_flip_angle,
            bw: self.spectral_width.hertz() as f32 /2.0,
            n_echos: self.n_echos as i32,
            s_psdname: self.name()
        }
    }
}
impl Initialize for MgreParams {
    fn default() -> Self {
        MgreParams {
            name: String::from(SEQUENCE_NAME),
            cs_table: Some(Path::new(r"C:\workstation\data\petableCS_stream\stream_CS480_8x_pa18_pb54").to_owned()),
            fov: (19.7, 12.0, 12.0),
            samples: (788, 480, 480),
            sample_discards: 0,
            spectral_width: SpectralWidth::SW200kH,
            readout_padding:100E-6,
            rf_alpha_flip_angle:30.0,
            rf_alpha_duration: 140E-6,
            tr_spoil_duration: 700E-6,
            read_rewind_duration: 1.14E-3,
            ramp_time: 100E-6,
            phase_encode_time: 600E-6,
            echo_time: 3.24E-3,
            n_echos: 4,
            echo_spacing: 6E-3,
            rep_time: 50E-3,
            n_averages: 1,
            n_repetitions: 2000,
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
impl MrdToKspace for MgreParams {
    fn mrd_to_kspace_params(&self) -> MrdToKspaceParams {

        let n_views = match self.is_cs(){
            true => {
                let table_compression = 8;
                (self.samples.1 as usize*self.samples.2 as usize)/table_compression
            },
            false => {
                self.samples.1 as usize*self.samples.2 as usize
            }
        };

        let mrd_format = match self.is_cs(){
            true => MrdFormat::StandardCSVol,
            false => MrdFormat::StandardVol
        };

        MrdToKspaceParams {
            mrd_format,
            n_read: self.samples.0 as usize,
            n_phase1: self.samples.1 as usize,
            n_phase2: self.samples.2 as usize,
            n_views,
            view_acceleration: 1,
            dummy_excitations: 0,
            n_objects: self.n_echos as usize
        }
    }
}
impl UseAdjustments for MgreParams {
    fn set_adjustment_file(&mut self, adj_file: &Path) {
        self.adjustment_file = Some(adj_file.to_owned());
    }
    fn adjustment_file(&self) -> Option<PathBuf> {
        self.adjustment_file.clone()
    }
}
impl SequenceParameters for MgreParams {
    fn name(&self) -> String {
        String::from(SEQUENCE_NAME)
    }
    fn write(&self,params_file: &Path){
        let str = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        let mut f = File::create(params_file).expect("cannot create file");
        f.write_all(str.as_bytes()).expect("trouble writing to file");
    }
    fn instantiate(&self) -> Box<dyn Build> {
        Box::new(Mgre::new(self.clone()))
    }
}
impl Build for Mgre {
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
pub struct MgreParams {
    name: String,
    cs_table: Option<PathBuf>,
    fov: (f32, f32, f32),
    samples: (u16, u16, u16),
    sample_discards: u16,
    spectral_width: SpectralWidth,
    readout_padding: f32,
    rf_alpha_flip_angle: f32,
    rf_alpha_duration: f32,
    tr_spoil_duration: f32,
    read_rewind_duration:f32,
    ramp_time: f32,
    phase_encode_time: f32,
    echo_time: f32,
    n_echos: u16,
    echo_spacing: f32,
    rep_time: f32,
    n_averages: u16,
    n_repetitions: u32,
    setup_mode: bool,
    grad_off: bool,
    adjustment_file:Option<PathBuf>,
}

#[derive(Clone)]
pub struct Mgre {
    params: MgreParams,
    events: MgreEvents,
}

#[derive(Clone)]
pub struct MgreEvents {
    tr_spoiler: GradEvent<Trapezoid>,
    excitation: RfEvent<Hardpulse>,
    phase_encode_start: GradEvent<Trapezoid>,
    phase_encode_mid: GradEvent<Trapezoid>,
    phase_encode_end: GradEvent<Trapezoid>,
    readout: GradEvent<Trapezoid>,
    acquire: AcqEvent,
}

struct Waveforms {
    excitation: Hardpulse,
    phase_encode: Trapezoid,
    read_rewind: Trapezoid,
    readout: Trapezoid,
    tr_spoiler: Trapezoid,
}

struct GradMatrices {
    phase_encode_start: Matrix,
    phase_encode_mid: Matrix,
    phase_encode_end: Matrix,
    readout: Matrix,
    tr_spoiler: Matrix,
}

impl Mgre {

    pub fn new(params: MgreParams) -> Mgre {
        let events = Self::events(&params);
        Self {
            events,
            params
        }
    }

    fn waveforms(params: &MgreParams) -> Waveforms {
        let read_sample_time_sec = params.spectral_width.sample_time(params.samples.0 + params.sample_discards) + params.readout_padding;
        let excitation = Hardpulse::new(params.rf_alpha_duration);
        let readout = Trapezoid::new(params.ramp_time, read_sample_time_sec);
        let phase_encode = Trapezoid::new(params.ramp_time, params.phase_encode_time);
        let tr_spoiler = Trapezoid::new(params.ramp_time, params.tr_spoil_duration);
        let read_rewind = Trapezoid::new(params.ramp_time,params.read_rewind_duration);
        Waveforms {
            excitation,
            phase_encode,
            read_rewind,
            readout,
            tr_spoiler,
        }
    }

    fn gradient_matrices(params: &MgreParams) -> GradMatrices {
        let waveforms = Self::waveforms(params);
        let mat_count = Matrix::new_tracker();
        let fov_read = params.fov.0;
        let non_adjustable = (false, false, false);

        /* READOUT */
        let read_grad_dac = params.spectral_width.fov_to_dac(fov_read);
        let readout = Matrix::new_static("read_mat", DacValues::new(Some(read_grad_dac), None, None), non_adjustable, params.grad_off, &mat_count);

        /* PHASE ENCODING */

        let phase_encode_strategy = match params.is_cs() {
            true =>{
                let lut = CSTable::open(&params.cs_table.clone().unwrap()).elements();
                EncodeStrategy::LUT(Dimension::_3D,lut)
            }
            false => EncodeStrategy::FullySampled(Dimension::_3D,params.samples.1 as usize,Some(params.samples.2 as usize))
        };

        // phase encoding is driven by the excitation count (rep count)
        let phase_encode_driver = MatrixDriver::new(DriverVar::Repetition,MatrixDriverType::PhaseEncode(phase_encode_strategy),None);



        // calculate required dac value for read pre-phase
        let read_pre_phase_dac = waveforms.phase_encode.magnitude_net(0.5 * waveforms.readout.power_net(read_grad_dac as f32)) as i16;


        // in setup mode, we want to disable phase encoding
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
        let phase_encode_matrix_transform = LinTransform::new((None, Some(phase_multiplier), Some(slice_multiplier)), (None, None, None));

        // primary phase encoding matrix
        let phase_encode_start = Matrix::new_driven(
            "c_pe_start_mat",
            phase_encode_driver,
            phase_encode_matrix_transform,
            DacValues::new(Some(-read_pre_phase_dac), None, None),
            (true, false, false),
            params.grad_off,
            &mat_count
        );

        let read_rewind_dac = waveforms.read_rewind.magnitude_net(1.0 * waveforms.readout.power_net(read_grad_dac as f32)) as i16;

        let phase_encode_mid = Matrix::new_static(
            "c_pe_mid_mat",
            DacValues::new(Some(-read_rewind_dac),None,None),
            (true,false,false),
            params.grad_off,
            &mat_count
        );

        // rewinder matrix at end of echo train
        let rewind_transform = LinTransform::new((None,Some(-1.0),Some(-1.0)),(None,None,None));

        let phase_encode_end = Matrix::new_derived(
            "c_pe_end_mat",
            &Rc::new(phase_encode_start.clone()),
            rewind_transform,
            non_adjustable,
            params.grad_off,
            &mat_count,
        );


        /* TR SPOILER */
        let spoil_dac = 1000;
        let tr_spoiler = Matrix::new_static(
            "c_spoiler_mat",
            DacValues::new(Some(spoil_dac),Some(spoil_dac),Some(spoil_dac)),
            (true,true,true),
            params.grad_off,
            &mat_count,
        );

        GradMatrices {
            phase_encode_start,
            phase_encode_mid,
            phase_encode_end,
            readout,
            tr_spoiler
        }
    }

    fn events(params: &MgreParams) -> MgreEvents {
        let w = Self::waveforms(params);
        let m = Self::gradient_matrices(params);

        // find the appropriate excitation rf dac value based on adjustment settings
        let excitation_dac = params.rf_dac(params.rf_alpha_flip_angle,Box::new(w.excitation.clone())).unwrap_or(100);
        let excitation = RfEvent::new(
            "excitation",
            1,
            w.excitation,
            RfStateType::Adjustable(excitation_dac, None),
            RfStateType::Static(0)
        );

        let phase_encode_start = GradEvent::new(
            (Some(w.phase_encode), Some(w.phase_encode), Some(w.phase_encode)),
            &m.phase_encode_start,
            GradEventType::Blocking,
            "phase_encode_start"
        );

        let phase_encode_mid = GradEvent::new(
            (Some(w.read_rewind), None, None),
            &m.phase_encode_mid,
            GradEventType::Blocking,
            "phase_encode_mid"
        );

        let phase_encode_end = GradEvent::new(
            (None, Some(w.phase_encode), Some(w.phase_encode)),
            &m.phase_encode_end,
            GradEventType::Blocking,
            "phase_encode_end"
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

        let tr_spoiler = GradEvent::new(
            (Some(w.tr_spoiler), Some(w.tr_spoiler), Some(w.tr_spoiler)),
            &m.tr_spoiler,
            GradEventType::Blocking,
            "tr_spoiler"
        );

        MgreEvents {
            tr_spoiler,
            excitation,
            phase_encode_start,
            phase_encode_mid,
            phase_encode_end,
            readout,
            acquire
        }
    }


    fn place_events(&self) -> EventQueue {

        let te_clocks = sec_to_clock(self.params.echo_time);
        let spacing_clocks = sec_to_clock(self.params.echo_spacing);

        let spoiler_spacing = sec_to_clock(500E-6) as u32;


        // temporary storage for sequence events
        let mut event_buffer = Vec::<Rc<RefCell<Event>>>::new();


        let excitation = Event::new(self.events.excitation.as_reference(),Origin);
        let tr_spoiler = Event::new(self.events.tr_spoiler.as_reference(),Before(excitation.clone(),spoiler_spacing));

        let read1 = Event::new(self.events.readout.as_reference(),ExactFromOrigin(te_clocks));
        let acq1 = Event::new(self.events.acquire.as_reference(),ExactFromOrigin(te_clocks));

        let pe1 = Event::new(self.events.phase_encode_start.as_reference(),Before(read1.clone(),0));

        event_buffer.push(excitation);
        event_buffer.push(tr_spoiler);
        event_buffer.push(read1);
        event_buffer.push(acq1.clone());
        event_buffer.push(pe1);

        // keep track of the last acq event for final placement phase rewinder gradient
        let mut last_acq = acq1;

        // populate the remaining echos
        for echo_idx in 1..self.params.n_echos {
            let echo_center = te_clocks + (echo_idx as i32)*spacing_clocks;

            let read = Event::new(self.events.readout.as_reference(),ExactFromOrigin(echo_center));
            let acq = Event::new(self.events.acquire.as_reference(),ExactFromOrigin(echo_center));
            let read_rewind = Event::new(self.events.phase_encode_mid.as_reference(),Before(read.clone(),0));

            last_acq = acq.clone();

            event_buffer.extend(vec![
                read,acq,read_rewind
            ]);
        }

        let phase_rewind = Event::new(self.events.phase_encode_end.as_reference(),After(last_acq,0));
        event_buffer.push(phase_rewind);

        EventQueue::new(
            &event_buffer
        )
    }
}
