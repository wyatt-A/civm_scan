use crate::command_string::CommandString;
use crate::{_utils, ppl_function};
use crate::ppl_constants::{AVERAGES_LOOP_COUNTER_VAR, AVERAGES_LOOP_NAME, NO_AVERAGES_VAR, NO_VIEWS_VAR, VIEW_LOOP_COUNTER_VAR, VIEW_LOOP_NAME};
use crate::event_block::{EventQueue, EventQueueError};

pub enum Loop {
    Repetition(u32,u16),
    Average(u16)
}

impl Loop {
    pub fn start(&self) -> String {
        match self {
            Loop::Repetition(_,_) =>
                Loop::_start(VIEW_LOOP_NAME),
            Loop::Average(_) =>
                Loop::_start(AVERAGES_LOOP_NAME)
        }
    }
    pub fn end(&self) -> String {
        match self{
            Loop::Repetition(_,step) =>
                Loop::_end(VIEW_LOOP_NAME,VIEW_LOOP_COUNTER_VAR,NO_VIEWS_VAR,*step),
            Loop::Average(_) =>
                Loop::_end(AVERAGES_LOOP_NAME,AVERAGES_LOOP_COUNTER_VAR,NO_AVERAGES_VAR,1),
        }
    }

    pub fn init_counter(&self) -> String {
        match self {
            Loop::Repetition(_,_) =>
                format!("{} = 0;",VIEW_LOOP_COUNTER_VAR),
            Loop::Average(_) =>
                format!("{} = 0;",AVERAGES_LOOP_COUNTER_VAR)
        }
    }

    fn _start(loop_name:&str) -> String {
        format!("{}:",loop_name)
    }
    fn _end(loop_name:&str,counter_name:&str,varname:&str,step:u16) -> String {
        vec![
            format!("{} = {} + {};",counter_name,counter_name,step),
            format!("if ({} < {}*{})",counter_name,step,varname),
            format!("goto {};",loop_name),
        ].join("\n")
    }

}

pub struct FlatLoopStructure {
    outer:Loop,
    inner:Loop,
    calc_block:CalcBlock,
    exec_block:Vec<CommandString>
}

impl FlatLoopStructure {
    pub fn new(repetitions:u32, averages:u16, rep_time:f32, event_queue:&mut EventQueue,acceleration:u16) -> Result<Self,EventQueueError> {
        // lock in events in the event queue so timing is accurate
        let calc_block = CalcBlock::new(event_queue.export_calc_blocks());
        let calc_time = calc_block.duration_clocks();
        let loop_time = FlatLoopStructure::loop_waittimer();
        event_queue.set_rep_time(rep_time,loop_time,calc_time)?;
        Ok(Self {
            outer:Loop::Repetition(repetitions,acceleration),
            inner:Loop::Average(averages),
            calc_block,
            exec_block:event_queue.export_exec_blocks() // rep time must be set before exporting execs
        })
    }
    fn loop_waittimer() -> i32 {
        500
    }

    pub fn print(&self) -> String {

        let exec_string_vec:Vec<String> = self.exec_block.iter().map(|block| block.commands.clone()).collect();
        let exec_string = exec_string_vec.join("\n");

        vec![
            ppl_function::start_timer(),
            self.outer.init_counter(),
            self.outer.start(),
            self.inner.init_counter(),
            self.inner.start(),
            ppl_function::wait_timer(FlatLoopStructure::loop_waittimer()),
            self.calc_block.print(),
            exec_string,
            ppl_function::start_timer(),
            self.inner.end(),
            self.outer.end()
        ].join("\n")
    }
    pub fn n_reps(&self) -> u32 {
        match self.outer {
            Loop::Repetition(n,_) => n,
            _=> panic!("this loop structure must have an outer repetition loop")
        }
    }
    pub fn n_averages(&self) -> u32 {
        match self.outer {
            Loop::Average(n) => n as u32,
            _=> panic!("this loop structure must have an inner averages loop")
        }
    }
}

pub struct CalcBlock {
    body:Vec<CommandString>,
}

impl CalcBlock {
    pub fn new(calc_commands:Vec<CommandString>) -> Self {
        Self {
            body:calc_commands
        }
    }
    fn header(&self) -> Vec<CommandString>{
        vec![
            CommandString::new_calculation(&ppl_function::start_timer()),
            CommandString::new_calculation(&ppl_function::host_request()),
            CommandString::new_calculation(&ppl_function::system_out())
        ]
    }
    pub fn footer(&self) -> Vec<CommandString> {
        vec![CommandString::new_calculation(&ppl_function::wait_timer(self.duration_clocks()))]
    }
    pub fn duration_clocks(&self) -> i32 {
        // todo! estimate required duration from body
        _utils::ms_to_clock(3)
    }
    pub fn print(&self) -> String {
        let h:Vec<String> = self.header().iter().map(|cmds| cmds.commands.clone()).collect();
        let b:Vec<String> = self.body.iter().map(|cmds| cmds.commands.clone()).collect();
        let f:Vec<String> = self.footer().iter().map(|cmds| cmds.commands.clone()).collect();
        vec![
            h.join("\n"),
            b.join("\n"),
            f.join("\n")
        ].join("\n")
    }
}