use crate::rf_frame::RfFrame;
use crate::rf_state::{RfState, RfStateType};
use crate::command_string::CommandString;
use crate::_utils;
use crate::execution::{ExecutionBlock, WaveformData, PlotTrace, BlockExecution, EventType};
use crate::ppl_function;
use crate::pulse_function::render_function_vector;
use crate::_utils::us_to_clock;
use crate::hardware_constants::RF_SEQ_FILE_LABEL;
use crate::ppl_header::Adjustment;
use crate::seqframe::SeqFrame;
use crate::timing_constants::{RF_TIME_BLOCK, RFSTART_POSTDELAY, RFSTART_PREDELAY};


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
    pub fn set_rf_phase(&mut self,rf_phase:RfStateType) {
        let power = self.rf_state.power();
        self.rf_state = RfState::new(&self.label,power,rf_phase);
    }
    pub fn pulse_duration_us(&self) -> i32 {
        _utils::sec_to_us(self.rf_frame.duration())
    }
    pub fn pulse_duration_clocks(&self) -> i32 {
        us_to_clock(self.pulse_duration_us())
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
    fn block_duration(&self) -> i32 {
        RF_TIME_BLOCK + RFSTART_PREDELAY + us_to_clock(self.pulse_duration_us()) + RFSTART_POSTDELAY
    }
    fn time_to_start(&self) -> i32 {
        RF_TIME_BLOCK + RFSTART_PREDELAY
    }
    fn time_to_end(&self) -> i32 {
        self.time_to_start() + self.pulse_duration_clocks()
    }
    fn time_to_center(&self) -> i32 {
        self.time_to_start() + self.pulse_duration_clocks()/2
    }
    fn block_execution(&self,post_delay:i32) -> BlockExecution {
        let cmd_str = vec![
            ppl_function::start_timer(),
            ppl_function::resync(),
            ppl_function::set_phase_with_var(&self.rf_state.phase_var()),
            ppl_function::wait_timer(RF_TIME_BLOCK),
            ppl_function::rf_start(self.uid, self.pulse_duration_us() as u16, &self.rf_state.power_var(), _utils::clock_to_us(RFSTART_PREDELAY) as u16),
        ];
        let cmd_str = CommandString::new_hardware_exec(&cmd_str.join("\n"));
        BlockExecution::new(cmd_str,post_delay)
    }
    fn block_header_adjustments(&self) -> Option<Vec<Adjustment>> {
        self.rf_state.header_declaration()
    }
    fn block_constant_initialization(&self) -> Option<CommandString> {
        None
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

    fn as_reference(&self) -> Box<dyn ExecutionBlock> {
        Box::new(self.clone())
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

    fn kind(&self) -> EventType {
        EventType::Rf
    }
    fn blocking(&self) -> bool {
        true
    }
    fn seq_params(&self,sample_period_us:usize) -> Option<String> {
        let (amp,phase) = self.seq_frame(sample_period_us);
        Some(vec![
            amp.serialize(),
            phase.serialize()
        ].join("\n"))
    }

    fn render_magnitude(&self, time_step_us: usize, _driver_value: u32) -> WaveformData {
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
