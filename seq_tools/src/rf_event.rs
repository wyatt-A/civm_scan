use std::cell::RefCell;
use std::rc::Rc;
use crate::rf_frame::RfFrame;
use crate::rf_state::{PhaseCycleStrategy, RfDriver, RfDriverType, RfState, RfStateType};
use crate::command_string::CommandString;
use crate::pulse::{CompositeHardpulse, Pulse};
use crate::utils;
use crate::execution::{ExecutionBlock, WaveformData, PlotTrace, BlockExecution, EventType};
use crate::ppl_function;
use crate::pulse_function::render_function_vector;
use crate::utils::us_to_clock;
use crate::ppl::Adjustment;
use crate::seqframe::{RF_SEQ_FILE_LABEL, SeqFrame};

//const TIME_BLOCK:i32 = 150; // clock cycles (100ns)
const TIME_BLOCK:i32 = 400; // clock cycles (100ns)
const RFSTART_PREDELAY:i32 = 600; // clock cycles
const RFSTART_POSTDELAY:i32 = 84; // clock cycles (found experimentally)

#[derive(Clone)]
pub struct RfEvent<RF> where RF:RfFrame{
    rf_frame:RF,
    rf_state:RfState,
    label:String,
    uid:u8
}

impl<RF> RfEvent<RF> where RF:RfFrame {
    pub fn new(label:&str,uid:u8,pulse:RF,rf_power:RfStateType,rf_phase:RfStateType) -> RfEvent<RF> {
        RfEvent {
            rf_frame:pulse,
            rf_state:RfState::new(label,rf_power,rf_phase),
            label:label.to_owned(),
            uid,
        }
    }
    pub fn pulse_duration_us(&self) -> i32 {
        utils::sec_to_us(self.rf_frame.duration())
    }
    pub fn pulse_duration_clocks(&self) -> i32 {
        utils::us_to_clock(self.pulse_duration_us())
    }
    pub fn init_list(&self) -> String {
        let d_us = self.pulse_duration_us();
        format!("NEWSHAPE_MAC({},{},\"{}\",{},{})",self.uid,RF_SEQ_FILE_LABEL,self.label,d_us,d_us/2)
        //NEWSHAPE_MAC(2,civm_rf,"excitation_rfwav",100,50)
    }
    pub fn label(&self) -> String {
        self.label.clone()
    }
    pub fn seq_frame(&self,sample_period_us:usize) -> (SeqFrame,SeqFrame) {
        self.rf_frame.rf_seq_frame(&self.label.clone(),sample_period_us)
    }
}

impl<RF: 'static> ExecutionBlock for RfEvent<RF> where RF:RfFrame + Clone{
    fn kind(&self) -> EventType {
        EventType::Rf
    }
    fn block_duration(&self) -> i32 {
        TIME_BLOCK + RFSTART_PREDELAY + us_to_clock(self.pulse_duration_us()) + RFSTART_POSTDELAY
    }
    fn block_execution(&self,post_delay:i32) -> BlockExecution {
        let cmd_str = vec![
            ppl_function::start_timer(),
            ppl_function::resync(),
            ppl_function::set_phase_with_var(&self.rf_state.phase_var()),
            ppl_function::wait_timer(TIME_BLOCK),
            ppl_function::rf_start(self.uid,self.pulse_duration_us() as u16,&self.rf_state.power_var(),utils::clock_to_us(RFSTART_PREDELAY) as u16),
        ];
        let cmd_str = CommandString::new_hardware_exec(&cmd_str.join("\n"));
        BlockExecution::new(cmd_str,post_delay)
    }
    fn block_declaration(&self) -> CommandString {
        let cmd_str = vec![
            self.rf_state.declare_power_var().unwrap_or("".to_string()),
            self.rf_state.declare_phase_var(),
        ].join("\n");
        CommandString::new_declare(&cmd_str)
    }
    fn block_calculation(&self) -> Option<CommandString> {
        let mut cmds = Vec::<String>::new();
        cmds.push(self.rf_state.set_phase());
        match self.rf_state.set_power() {
            Some(code) => {
                cmds.push(code);
            }
            None => {}
        }
        Some(CommandString::new_calculation(&cmds.join("\n")))
    }
    fn block_initialization(&self) -> CommandString {
        CommandString::new_calculation(
            &vec![
                self.rf_state.init_phase_var(),
                self.rf_state.init_power_var().unwrap_or("".to_string()),
                self.init_list(),
            ].join("\n")
        )
    }
    fn block_constant_initialization(&self) -> Option<CommandString> {
        None
    }
    fn block_header_adjustments(&self) -> Option<Vec<Adjustment>> {
        self.rf_state.header_declaration()
    }
    fn as_reference(&self) -> Box<dyn ExecutionBlock> {
        Box::new(self.clone())
    }

    fn time_to_start(&self) -> i32 {
        TIME_BLOCK + RFSTART_PREDELAY
    }

    fn time_to_end(&self) -> i32 {
        self.time_to_start() + self.pulse_duration_clocks()
    }

    fn time_to_center(&self) -> i32 {
        self.time_to_start() + self.pulse_duration_clocks()/2
    }

    fn label(&self) -> String {
        self.label.clone()
    }

    fn render_normalized(&self, time_step_us:usize) -> WaveformData {
        let phase = render_function_vector(self.rf_frame.phase_function(time_step_us));
        let amplitude_plot = self.rf_frame.render_normalized(time_step_us);
        let t = amplitude_plot.x.clone();
        let phase_plot = PlotTrace::new(t,phase);
        WaveformData::Rf(amplitude_plot,phase_plot)
    }
    fn seq_params(&self,sample_period_us:usize) -> Option<String> {
        let (amp,phase) = self.seq_frame(sample_period_us);
        Some(vec![
            amp.serialize(),
            phase.serialize()
        ].join("\n"))
    }
    fn blocking(&self) -> bool {
        true
    }

    fn render_magnitude(&self, time_step_us: usize, driver_value: u32) -> WaveformData {
        let power = self.rf_state.power();
        match power {
            RfStateType::Static(dac) => {
                let phase = render_function_vector(self.rf_frame.phase_function(time_step_us));
                let amplitude_plot = self.rf_frame.render_magnitude(time_step_us,dac);
                let t = amplitude_plot.x.clone();
                let phase_plot = PlotTrace::new(t,phase);
                WaveformData::Rf(amplitude_plot,phase_plot)
            }
            _=> {
                //todo!(get dac values for driven rf state instead of rendering normalized)
                println!("dynamic rf power rendering not yet implemented!!!");
                let phase = render_function_vector(self.rf_frame.phase_function(time_step_us));
                let amplitude_plot = self.rf_frame.render_normalized(time_step_us);
                let t = amplitude_plot.x.clone();
                let phase_plot = PlotTrace::new(t,phase);
                WaveformData::Rf(amplitude_plot,phase_plot)
            }
        }
    }
}

/*
starttimer();
resync();
phase(excitation_phase);
waittimer(150);
MR3031_RFSTART(2,100,excitation,60,4)
delay32(c_excitation_postdel);
 */

// #[test]
// fn test(){
//
//     let phase_cycling = PhaseCycleStrategy::LUTNinetyTwoSeventy(480,Some(480));
//     let phase_cycle_driver = RfDriver::new("no_complete_views",RfDriverType::PhaseCycle3D(phase_cycling));
//     let phase = RfStateType::Driven(phase_cycle_driver);
//     let power = RfStateType::Adjustable(400);
//     let pulse = CompositeHardpulse::new_180(100E-6);
//     let rfe = RfEvent::new("excite",0,pulse,power,phase);
//
//     //let header = rfe.block_header_statements();
//     let dec = rfe.block_declaration();
//     let consts = rfe.block_constant_initialization();
//     let init = rfe.block_initialization();
//     let calc = rfe.block_calculation();
//     let exec = rfe.block_execution(64).cmd_string();
//     println!("{}",dec.commands);
//     println!("{}",init.commands);
//     println!("{}",calc.unwrap().commands);
//     println!("{}",exec.commands);
// }