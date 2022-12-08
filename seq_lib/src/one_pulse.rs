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
use seq_tools::pulse::{CompositeHardpulse, HalfSin, Hardpulse, Pulse, Trapezoid};
use seq_tools::rf_event::RfEvent;
use seq_tools::rf_state::{PhaseCycleStrategy, RfStateType};
use seq_tools::_utils::{sec_to_clock};
use crate::pulse_sequence::{Build, PPLBaseParams, SequenceParameters, Setup, DiffusionWeighted, DiffusionPulseShape, CompressedSense, b_val_to_dac, Simulate, AcqDimensions, AcqDims, Initialize, DWSequenceParameters, MrdToKspace, MrdToKspaceParams, MrdFormat, ScoutConfig, AdjustmentParameters};
use serde_json;
use serde::{Serialize,Deserialize};
use cs_table::cs_table::CSTable;
use headfile::headfile::{DWHeadfile, DWHeadfileParams, AcqHeadfile, AcqHeadfileParams};
use seq_tools::ppl::Orientation::CivmStandard;
use crate::pulse_sequence;

impl Simulate for OnePulseParams {
    fn set_sim_repetitions(&mut self) {
        self.n_repetitions = 2;
    }
}

impl Initialize for OnePulseParams {
    fn default() -> Self {
        OnePulseParams {
            name: "one_pulse".to_string(),
            samples:4096,
            sample_discards: 0,
            spectral_width: SpectralWidth::SW100kH,
            rf_duration: 10E-6,
            echo_time: 300E-6,
            obs_freq_offset: 0.0,
            rep_time: 1.0,
            n_repetitions: 128,
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


impl AdjustmentParameters for OnePulseParams {
    fn set_freq_offset(&mut self, offset_hertz: f32) {
        self.obs_freq_offset = offset_hertz as f64;
    }

    fn name(&self) -> String {
        String::from("one_pulse")
    }
    fn write(&self,params_file: &Path){
        let str = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        let mut f = File::create(params_file).expect("cannot create file");
        f.write_all(str.as_bytes()).expect("trouble writing to file");
    }
    fn instantiate(&self) -> Box<dyn Build> {
        Box::new(OnePulse::new(self.clone()))
    }
}

impl Build for OnePulse {
    fn place_events(&self) -> EventQueue {
        self.place_events()
    }
    fn base_params(&self) -> PPLBaseParams {
        PPLBaseParams {
            n_averages: 1,
            n_repetitions: self.params.n_repetitions,
            rep_time: self.params.rep_time,
            base_frequency: BaseFrequency::civm9p4t(0.0),
            orientation: CivmStandard,
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
pub struct OnePulseParams {
    pub name: String,
    pub samples: u16,
    pub sample_discards: u16,
    pub spectral_width: SpectralWidth,
    pub rf_duration: f32,
    pub echo_time: f32,
    pub rep_time: f32,
    pub n_repetitions: u32,
    pub obs_freq_offset: f64,
}

#[derive(Clone)]
pub struct OnePulse {
    params: OnePulseParams,
    events: OnePulseEvents,
}

#[derive(Clone)]
pub struct OnePulseEvents {
    excitation: RfEvent<Hardpulse>,
    acquire: AcqEvent,
}

struct Waveforms {
    excitation: Hardpulse,
}

impl OnePulse {

    pub fn new(params: OnePulseParams) -> OnePulse {
        let events = Self::events(&params);
        Self {
            events,
            params
        }
    }

    fn waveforms(params: &OnePulseParams) -> Waveforms {
        let excitation = Hardpulse::new(params.rf_duration);
        Waveforms {
            excitation
        }
    }


    fn events(params: &OnePulseParams) -> OnePulseEvents {
        let w = Self::waveforms(params);

        let excitation = RfEvent::new(
            "excitation",
            1,
            w.excitation,
            RfStateType::Adjustable(100, None),
            RfStateType::Static(0)
        );

        let acquire = AcqEvent::new(
            "acquire",
            params.spectral_width.clone(),
            params.samples,
            params.sample_discards,
            RfStateType::Static(0)
        );

        OnePulseEvents {
            excitation,
            acquire,
        }
    }

    fn place_events(&self) -> EventQueue {
        let excitation = Event::new(self.events.excitation.as_reference(), Origin);
        let acquire = Event::new(self.events.acquire.as_reference(), After(excitation.clone(), _utils::sec_to_clock(self.params.echo_time) as u32));
        EventQueue::new(
            &vec![
                excitation,
                acquire,
            ]
        )
    }
}
