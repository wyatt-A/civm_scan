use crate::execution::{BlockExecution, ExecutionBlock, PlotTrace, WaveformData, EventType};
use crate::command_string::CommandString;
use crate::rf_state::{RfState, RfStateType};
use crate::{ppl_function, utils};
use crate::pulse_function::{Function,FunctionParams};
use crate::ppl::ScrollBar;
use crate::grad_cal;
use crate::pulse::Trapezoid;
use crate::gradient_event::GradEvent;
use std::cell::RefCell;
use std::rc::Rc;
use serde::{Deserialize, Serialize};
use crate::gradient_matrix::{Matrix, DacValues};
use crate::event_block::GradEventType;
use crate::utils::sec_to_clock;

// time allocated for setting reciever phase
const TIME_BLOCK_1:i32 = 500;
// time required after call to acquire before samples are collected
const TIME_BLOCK_2:i32 = 221;

const NO_SAMPLES_VAR:&str = "no_samples";
const SAMPLE_PERIOD_VAR:&str = "sample_period";
const NO_DISCARD_VAR:&str = "no_discard";
const MIN_DISCARD:u16 = 0;
const MAX_DISCARD:u16 = 16;
const MIN_SAMPLES:u16 = 8;
const MAX_SAMPLES:u16 = 65535;

#[derive(Clone,PartialEq,Serialize,Deserialize)]
pub enum SpectralWidth {
    SW100kH,
    SW200kH,
    SW133kH,
    SW80kH,
}

impl SpectralWidth {
    pub fn hertz(&self) -> i32 {
        match self {
            SpectralWidth::SW100kH => 100_000,
            SpectralWidth::SW200kH => 200_000,
            SpectralWidth::SW133kH => 133_333,
            SpectralWidth::SW80kH => 80_000
        }
    }
    // delay required after sampling is complete, but before control is returned
    pub fn post_delay(&self) -> i32 {
        let sample_rate = self.hertz();
        // delay is longer for sample rates less than or equal to 100kH
        if sample_rate <= 100_000 {3582} else {3582}
    }
    pub fn fov_to_dac(&self,fov_mm:f32) -> i16 {
        let grad_hz_per_mm = self.hertz() as f32/fov_mm;
        grad_cal::grad_to_dac(grad_hz_per_mm)
    }
    pub fn sample_time(&self,n_samples:u16) -> f32 {
        let sample_period = 1.0/self.hertz() as f32;
        sample_period*n_samples as f32
    }

    pub fn ppr_string(&self) -> String {
        match self {
            SpectralWidth::SW200kH => format!("{}, {}, \"200  KHz   5 µs\"",50,25),
            SpectralWidth::SW100kH => format!("{}, {}, \"100  KHz  10 µs\"",100,23),
            SpectralWidth::SW133kH => format!("{}, {}, \"133  KHz 7.5 µs\"",75,24),
            SpectralWidth::SW80kH => format!("{}, {}, \"80.0 KHz 12.5 µs\"",125,22),
        }
    }
}

#[derive(Clone)]
pub struct AcqEvent {
    sample_rate: SpectralWidth,
    n_samples:u16,
    n_discards:u16,
    phase_state:RfState,
    label:String
}

impl AcqEvent{
    pub fn new(label:&str, sample_rate: SpectralWidth, n_samples:u16, n_discards:u16, phase:RfStateType) -> AcqEvent {
        AcqEvent{
            sample_rate,
            n_samples,
            n_discards,
            phase_state:RfState::new_phase_only(label,phase),
            label:label.to_owned()
        }
    }
    pub fn n_samples(&self) -> u16 {
        self.n_samples
    }
    pub fn n_discards(&self) -> u16 {
        self.n_discards
    }
    pub fn n_samples_var(&self) -> String {
        NO_SAMPLES_VAR.to_string()
    }
    pub fn sample_period_var(&self) -> String {
        SAMPLE_PERIOD_VAR.to_string()
    }
    pub fn sample_period_clocks(&self) -> i32 {
        10_000_000/self.sample_rate.hertz()
    }
    pub fn sample_time_clocks(&self) -> i32 {
        // /sec_to_clock(self.sample_rate.sample_time(self.n_samples()+self.n_discards()))
            self.n_samples() as i32*self.sample_period_clocks()
    }
    pub fn readout_event(&self,fov_mm:f32) -> (f32,i16) {
        let dac_val = self.sample_rate.fov_to_dac(fov_mm);
        let sample_time = utils::clock_to_sec(self.sample_time_clocks());
        (sample_time,dac_val)
    }
}

impl ExecutionBlock for AcqEvent {
    fn block_duration(&self) -> i32 {
        self.sample_time_clocks() + TIME_BLOCK_1 + TIME_BLOCK_2 + self.sample_rate.post_delay()
    }
    fn time_to_start(&self) -> i32 {
        TIME_BLOCK_1 + TIME_BLOCK_2
    }
    fn time_to_end(&self) -> i32 {
        self.time_to_start() + self.sample_time_clocks()
    }
    fn time_to_center(&self) -> i32 {
        self.time_to_start() + self.sample_time_clocks()/2
    }
    fn block_execution(&self,post_delay_clock:i32) -> BlockExecution {
        let cmd = CommandString::new_hardware_exec(
            &vec![
                ppl_function::start_timer(),
                ppl_function::resync(),
                ppl_function::set_rec_phase_with_var(&self.phase_state.phase_var()),
                ppl_function::wait_timer(TIME_BLOCK_1),
                ppl_function::acquire(&self.n_samples_var(),&self.sample_period_var())
            ].join("\n")
        );
        BlockExecution::new(cmd,post_delay_clock)
    }
    fn block_header_adjustments(&self) -> Option<Vec<ScrollBar>> {
        None
    }
    fn block_constant_initialization(&self) -> Option<CommandString> {
        None
    }
    fn block_initialization(&self) -> CommandString {
        CommandString::new_init(
            &self.phase_state.init_phase_var()
        )
    }
    fn block_declaration(&self) -> CommandString {
        CommandString::new_declare(
            &self.phase_state.declare_phase_var()
        )
    }
    fn block_calculation(&self) -> Option<CommandString> {
        Some(CommandString::new_calculation(
            &self.phase_state.set_phase()
        ))
    }
    fn as_reference(&self) -> Box<dyn ExecutionBlock> {
        Box::new(self.clone())
    }
    fn label(&self) -> String {
        self.label.clone()
    }
    fn render_normalized(&self, time_step_us:usize) -> WaveformData {
        let step = utils::clock_to_sec(self.sample_period_clocks());
        let n = self.n_samples();
        let t:Vec<f32> = (0..n).map(|x| x as f32 * step).collect();
        let a:Vec<f32> = vec![1.0;n as usize];
        WaveformData::Acq(PlotTrace::new(t,a))
    }

    fn kind(&self) -> EventType {
        EventType::Acq(self.sample_rate.clone(),self.n_samples,self.n_discards)
    }
    fn blocking(&self) -> bool {
        true
    }
    fn seq_params(&self, sample_period_us: usize) -> Option<String> {
        None
    }
    fn render_magnitude(&self, time_step_us: usize, driver_value: u32) -> WaveformData {
        self.render_normalized(time_step_us)
    }
}

#[test]
fn test(){

    let phase = RfStateType::Static(0);
    let acq = AcqEvent::new("acq", SpectralWidth::SW100kH, 128, 0, phase);

    //let header = acq.block_header_statements();
    let dec = acq.block_declaration();
    let consts = acq.block_constant_initialization();
    let init = acq.block_initialization();
    let calc = acq.block_calculation();
    let exec = acq.block_execution(64).cmd_string();
    println!("{}",dec.commands);
    println!("{}",init.commands);
    println!("{}",calc.unwrap().commands);
    println!("{}",exec.commands);
}