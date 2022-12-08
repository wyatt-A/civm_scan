use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::fs::{File};
use std::io::{Read, Write};
use seq_tools::{grad_cal, _utils};
use seq_tools::acq_event::{AcqEvent, SpectralWidth};
use seq_tools::event_block::{Event, EventQueue, GradEventType};
use seq_tools::event_block::EventPlacementType::{After, Before, ExactFromOrigin, Origin};
use seq_tools::execution::ExecutionBlock;
use seq_tools::gradient_event::GradEvent;
use seq_tools::gradient_matrix::{DacValues, Dimension, DriverVar, EncodeStrategy, LinTransform, Matrix, MatrixDriver, MatrixDriverType};
use seq_tools::ppl::{GradClock, Orientation, PhaseUnit,BaseFrequency};
use seq_tools::pulse::{CompositeHardpulse, HalfSin, Hardpulse, Pulse, SliceSelective, Trapezoid};
use seq_tools::rf_event::RfEvent;
use seq_tools::rf_state::{PhaseCycleStrategy, RfDriver, RfDriverType, RfStateType};
use seq_tools::_utils::{sec_to_clock};
use crate::pulse_sequence::{Build, PPLBaseParams, SequenceParameters, Setup, DiffusionWeighted, DiffusionPulseShape, CompressedSense, b_val_to_dac, Simulate, AcqDimensions, AcqDims, Initialize, DWSequenceParameters, MrdToKspace, MrdToKspaceParams, MrdFormat, ScoutConfig, AdjustmentParameters};
use serde_json;
use serde::{Serialize,Deserialize};
use cs_table::cs_table::CSTable;
use headfile::headfile::{DWHeadfile, DWHeadfileParams, AcqHeadfile, AcqHeadfileParams};
use seq_tools::ppl::Orientation::CivmStandard;
use seq_tools::rf_frame::RF_MAX_DAC;
use crate::pulse_sequence;


impl Setup for RfCalParams {
    fn set_mode(&mut self) {
        self.setup_mode = true;
    }
    fn set_repetitions(&mut self) {
        self.n_repetitions = 2000;
    }
}

impl Simulate for RfCalParams {
    fn set_sim_repetitions(&mut self) {
        self.n_repetitions = 2;
    }
}

impl Initialize for RfCalParams {
    fn default() -> Self {
        RfCalParams {
            name: "rf_cal".to_string(),
            start_rf_dac: 512,
            end_rf_dac: 1536,
            n_repetitions: 2,
            samples:512,
            sample_discards: 0,
            spectral_width: SpectralWidth::SW100kH,
            rf_duration: 140E-6,
            slice_thickness: 10.0,
            echo_time: 30E-3,
            stabilize_time: 1.0E-3,
            obs_freq_offset: 0.0,
            rep_time: 1.0,
            ramp_time: 100E-6,
            filling_time: 30E-3,
            setup_mode: false,
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


impl AdjustmentParameters for RfCalParams {
    fn set_freq_offset(&mut self, offset_hertz: f32) {
        self.obs_freq_offset = offset_hertz;
    }
    fn name(&self) -> String {
        String::from("rf_cal")
    }
    fn write(&self,params_file: &Path){
        let str = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        let mut f = File::create(params_file).expect("cannot create file");
        f.write_all(str.as_bytes()).expect("trouble writing to file");
    }
    fn instantiate(&self) -> Box<dyn Build> {
        Box::new(RfCal::new(self.clone()))
    }
}

impl Build for RfCal {
    fn place_events(&self) -> EventQueue {
        self.place_events()
    }
    fn base_params(&self) -> PPLBaseParams {
        PPLBaseParams {
            n_averages: 1,
            n_repetitions: self.params.n_repetitions,
            rep_time: self.params.rep_time,
            base_frequency: BaseFrequency::civm9p4t(self.params.obs_freq_offset),
            orientation: CivmStandard,
            grad_clock: GradClock::CPS20,
            phase_unit: PhaseUnit::Min,
            view_acceleration: 1,
            waveform_sample_period_us: 10
        }
    }
    fn param_export(&self, filepath: &Path) {
        let params = self.params.clone();
        let name = params.name.clone();
        params.write(&filepath.join(name).with_extension("json"));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RfCalParams {
    pub name: String,
    pub start_rf_dac: i16,
    pub end_rf_dac: i16,
    pub samples: u16,
    pub sample_discards: u16,
    pub spectral_width: SpectralWidth,
    pub rf_duration: f32,
    pub slice_thickness: f32,
    pub echo_time: f32,
    pub stabilize_time: f32,
    pub filling_time: f32,
    pub rep_time: f32,
    pub ramp_time: f32,
    pub n_repetitions: u32,
    pub obs_freq_offset: f32,
    pub setup_mode: bool,
}

#[derive(Clone)]
pub struct RfCal {
    params: RfCalParams,
    events: RfCalEvents,
}

#[derive(Clone)]
pub struct RfCalEvents {
    rf_pulse: RfEvent<Hardpulse>,
    acquire: AcqEvent,
    slice_select: GradEvent<Trapezoid>
}

struct Waveforms {
    rf_pulse: Hardpulse,
    slice_select: Trapezoid,
}

struct GradMatrices {
    slice_select: Matrix
}

fn slice_select_plateau_time(params: &RfCalParams) -> f32 {
    let tau = params.echo_time/2.0;
    let acq_time = params.spectral_width.sample_time(params.samples);
    3.0*tau + params.filling_time + acq_time/2.0 + params.stabilize_time
}

impl RfCal {

    pub fn new(params: RfCalParams) -> RfCal {
        let events = Self::events(&params);
        Self {
            events,
            params
        }
    }

    fn waveforms(params: &RfCalParams) -> Waveforms {
        let rf_pulse = Hardpulse::new(params.rf_duration);
        let ss_plat = slice_select_plateau_time(params);
        let slice_select = Trapezoid::new(params.ramp_time,ss_plat);
        Waveforms {
            rf_pulse,
            slice_select
        }
    }

    fn gradient_matrices(params: &RfCalParams) -> GradMatrices {

        let tracker = Matrix::new_tracker();

        let w = Self::waveforms(params);
        let grad_strength = w.rf_pulse.grad_strength_hzpmm(params.slice_thickness);
        let dac = grad_cal::grad_to_dac(grad_strength);

        let slice_select = Matrix::new_static(
          "c_slice_select_mat",
            DacValues::new(None,None,Some(dac)),
            (false,false,false),
            false,
            &tracker,
        );
        GradMatrices {
            slice_select
        }
    }


    fn events(params: &RfCalParams) -> RfCalEvents {
        let w = Self::waveforms(params);
        let m = Self::gradient_matrices(params);

        let rf_pulse = match params.setup_mode {
            true => {
                RfEvent::new(
                    "excitation",
                    1,
                    w.rf_pulse,
                    RfStateType::Adjustable(100,None),
                    RfStateType::Static(0)
                )
            }
            false => {
                // set up the rf power driver based on repetitions
                if params.start_rf_dac > params.end_rf_dac {
                    panic!("start rf dac must be less than end rf dac");
                }

                let dac_per_rep = match params.n_repetitions {
                    1 => 0,
                    _=> (params.end_rf_dac - params.start_rf_dac)/(params.n_repetitions-1) as i16
                };

                let max_rf_dac = dac_per_rep*(params.n_repetitions-1) as i16 + params.start_rf_dac;
                if max_rf_dac > RF_MAX_DAC {
                    panic!("max rf dac is exceeded!");
                }
                let dt = RfDriverType::PowerRamp(dac_per_rep,params.start_rf_dac);
                let d = RfDriver::new(DriverVar::Repetition,dt,None);
                RfEvent::new(
                    "excitation",
                    1,
                    w.rf_pulse,
                    RfStateType::Driven(d.clone()),
                    RfStateType::Static(0)
                )
            }
        };


        let acquire = AcqEvent::new(
            "acquire",
            params.spectral_width.clone(),
            params.samples,
            params.sample_discards,
            RfStateType::Static(0)
        );

        let slice_select = GradEvent::new(
            (None,None,Some(w.slice_select)),
            &m.slice_select,
            GradEventType::NonBlocking,
            "slice_select"
        );

        RfCalEvents {
            rf_pulse,
            acquire,
            slice_select
        }
    }

    fn place_events(&self) -> EventQueue {

        let tau = self.params.echo_time/2.0;
        let tau_clocks = _utils::sec_to_clock(tau);
        let t_fill_clocks = _utils::sec_to_clock(self.params.filling_time);

        // check for obvious errors before solving event placement
        if tau_clocks < self.events.acquire.block_duration()/2 + self.events.rf_pulse.block_duration()/2 {
            panic!("echo time must be increased because of sample time")
        }
        if t_fill_clocks <= self.events.acquire.block_duration()/2 + self.events.rf_pulse.block_duration()/2 {
            panic!("filling time must be increased because of sample time")
        }

        let ss_plat = slice_select_plateau_time(&self.params);
        let t_center_ss = ss_plat/2.0 - self.params.stabilize_time;

        let ss_center_clocks = _utils::sec_to_clock(t_center_ss);

        let slice_select = Event::new(self.events.slice_select.as_reference(),ExactFromOrigin(ss_center_clocks));
        let pulse1 = Event::new(self.events.rf_pulse.as_reference(), Origin);
        let pulse2 = Event::new(self.events.rf_pulse.as_reference(), ExactFromOrigin(tau_clocks));
        let acq1 = Event::new(self.events.acquire.as_reference(), ExactFromOrigin(2*tau_clocks));
        let pulse3 = Event::new(self.events.rf_pulse.as_reference(), ExactFromOrigin(2*tau_clocks + t_fill_clocks));
        let acq2 = Event::new(self.events.acquire.as_reference(), ExactFromOrigin(3*tau_clocks + t_fill_clocks));

        EventQueue::new(
            &vec![
                slice_select,
                pulse1,
                pulse2,
                pulse3,
                acq1,
                acq2
            ]
        )
    }
}


#[test]
fn rf_cal_test(){
    let params = RfCalParams::default();
    let mut b = params.instantiate();
    //b.ppl_export(Path::new("."),"rf_cal",false,false);
    let q = b.place_events();

    let g = q.graphs_dynamic(2,0);



    println!("{:?}",g);

}