use std::borrow::BorrowMut;
use std::rc::Rc;
use crate::execution::{ExecutionBlock, WaveformData, EventType};
use crate::ppl_function::MIN_DELAY_CLOCKS;
use serde::{Serialize};
use serde_json;
use crate::command_string::CommandString;
use crate::ppl::{FlatLoopStructure, Adjustment};
use crate::_utils;
use std::cell::RefCell;
use std::collections::HashSet;
use crate::acq_event::SpectralWidth;


// this is where execution blocks inherit an absolute time relative to one another
pub struct Event {
    pub execution:Box<dyn ExecutionBlock>,
    center:i32,
    is_origin:bool,
    label:String,
    unique_label:String, // unique label in case duplicate execution block is run
    post_delay:i32
}

#[derive(Debug,Serialize,Clone)]
pub struct EventGraph {
    pub label:String,// derived from the execution block
    pub block_interval:(f32,f32),
    pub waveform_start:f32,
    pub wave_data:WaveformData
}

impl EventGraph {
    pub fn waveform_start(&self) -> f32 {
        self.waveform_start
    }
    pub fn waveform_end(&self) -> f32 {
        self.waveform_start + self.wave_data.waveform_duration()
    }
}


impl Event {
    pub fn new(exec_block:Box<dyn ExecutionBlock>,placement:EventPlacementType) -> Rc<RefCell<Self>> {
        use EventPlacementType::*;
        let center = match &placement {
            Origin => 0,
            ExactFromOrigin(center) => *center,
            Before(parent,offset) =>{
                let p = parent.borrow();
                //match p.execution.kind() == EventType::Grad && (p.execution.blocking() || exec_block.blocking()) {
                match exec_block.kind() == EventType::Grad && (p.execution.blocking() || exec_block.blocking()) {
                    true => p.block_start() - exec_block.time_to_end() + exec_block.time_to_center() - MIN_DELAY_CLOCKS - *offset as i32,
                    false => p.block_start() - exec_block.block_duration() + exec_block.time_to_center() - MIN_DELAY_CLOCKS - *offset as i32
                }
            }
            After(parent,offset) => {
                let p = parent.borrow();
                match p.execution.kind() == EventType::Grad && (p.execution.blocking() || exec_block.blocking()) {
                    true => p.block_start() + p.execution.time_to_end() + exec_block.time_to_center() + MIN_DELAY_CLOCKS + *offset as i32,
                    false => p.block_end() + exec_block.time_to_center() + MIN_DELAY_CLOCKS + *offset as i32
                }
            }
        };
        println!("event center for {} set to {}", exec_block.label(), _utils::clock_to_sec(center));
        let is_origin = match &placement {
            Origin => true,
            _ => false
        };
        let label = exec_block.label();
        Rc::new(RefCell::new(Self {
            execution:exec_block,
            center,
            is_origin,
            label:label.clone(),
            unique_label:label.clone(),
            post_delay:MIN_DELAY_CLOCKS
        }))
    }
    pub fn block_end(&self) -> i32 {
        self.center + self.execution.block_start() + self.execution.block_duration()
    }
    pub fn block_start(&self) -> i32 {
        self.center + self.execution.block_start()
    }
    pub fn event_graph_normalized(&self, time_step_us:usize) -> EventGraph {
        let start_sec = _utils::clock_to_sec(self.block_start());
        let end_sec = _utils::clock_to_sec(self.block_end());
            EventGraph{
                label:self.unique_label.to_owned(),
                block_interval:(start_sec,end_sec),
                waveform_start:start_sec + _utils::clock_to_sec(self.execution.time_to_start()),
                wave_data:self.execution.render_normalized(time_step_us)
                }
    }
    pub fn event_graph_dynamic(&self, time_step_us:usize, driver_var:u32) -> EventGraph {
        let start_sec = _utils::clock_to_sec(self.block_start());
        let end_sec = _utils::clock_to_sec(self.block_end());
        println!("waveform start for {} = {}",self.label,start_sec + _utils::clock_to_sec(self.execution.time_to_start()));
        //println!("waveform start for {} = {}",self.label,start_sec);
        EventGraph{
            label:self.unique_label.to_owned(),
            block_interval:(start_sec,end_sec),
            waveform_start:start_sec + _utils::clock_to_sec(self.execution.time_to_start()),
            wave_data:self.execution.render_magnitude(time_step_us,driver_var)
        }
    }
    pub fn set_post_delay(&mut self,extension:i32) {
        self.post_delay = extension;
    }
    pub fn center(&self) -> i32 {
        self.center
    }
}

/** Queue of sequence events that will be executed in a loop */
pub struct EventQueue {
    events:Vec<Rc<RefCell<Event>>>
}

impl EventQueue {
    /** Take an unordered vector of event refs and turn them into a valid event queue
     for execution */
    pub fn new(unordered:&Vec<Rc<RefCell<Event>>>) -> Self {
        let mut input = unordered.clone();
        input.sort_by_key(|event| event.borrow().block_start());
        let mut s = Self {
            events:input
        };
        s.set_unique_labels();
        s
    }
    /** Assign labels to events such that they don't collide */
    fn set_unique_labels(&mut self){
        let mut h = HashSet::<String>::new();
        // build collection of names
        self.events.iter().for_each(|event| {
            let label = event.borrow().label.clone();
            /* if hash doesn't contain the label, the name doesn't need to be changed
            and it get added to the hash */
            match h.contains(&label){
                false => {
                    h.insert(label);
                }
                /* If the hash does contain the label, it get modified and then added
                 to the hash*/
                true => {
                    let mut i = 2;
                    loop {
                        let modified = format!("{}_{}",&label,i);
                        match h.contains(&modified) {
                            false => {
                                event.as_ref().borrow_mut().unique_label = modified.clone();
                                h.insert(modified);
                                break
                            }
                            true => i += 1
                        }
                    }
                }
            }
        })
    }
    /** Render out waveforms to a vector of EventGraph structures for writing to a file */
    pub fn graphs(&self,time_step_us:usize) -> Vec<EventGraph> {
        self.events.iter().map(|event| event.borrow().event_graph_normalized(time_step_us)).collect()
    }
    pub fn graphs_dynamic(&self,time_step_us:usize,driver_val:u32) -> Vec<EventGraph> {
        self.events.iter().map(|event| event.borrow().event_graph_dynamic(time_step_us,driver_val)).collect()
    }
    /** Generates a vector of ordered commands that will run the mr hardware */
    pub fn export_exec_blocks(&self) -> Vec<CommandString> {
        // needs to be validated first
        self.events.iter().map(|event|
            event.borrow().execution.block_execution(event.borrow().post_delay).cmd_string()
        ).collect()
    }
    /** Generates a vector of ordered commands for initializing variables every loop iteration */
    pub fn export_calc_blocks(&self) -> Vec<CommandString> {
        self.unique().iter().flat_map(|event| event.borrow().execution.block_calculation()).collect()
    }
    /** Calculates the required delay for the last event to ensure desired rep time is accurate */
    fn set_tr_makeup(&mut self,rep_time:f32,loop_time:i32,calc_time:i32) {
        // set the last event post delay such that tr is accurate
        let first_event_start = self.events[0].borrow().block_start();
        let last_event_idx = self.events.len()-1;
        let last_event_end = self.events[last_event_idx].borrow().block_end();
        let tr = _utils::sec_to_clock(rep_time);
        let makeup = tr + first_event_start - last_event_end - loop_time - calc_time;
        if makeup < 0 {
            let adj = rep_time - _utils::clock_to_sec(makeup);
            panic!("rep time is too short. It needs to be at least {} seconds for loop execution",adj)
        }
        self.events[last_event_idx].as_ref().borrow_mut().set_post_delay(makeup);
    }
    /** Sets the rep time and all other event post-delay properties */
    pub fn set_rep_time(&mut self,rep_time:f32,loop_time:i32,calc_time:i32){
        self.set_post_delay();
        self.set_tr_makeup(rep_time,loop_time,calc_time);
    }
    /** Sets the post-delays for every event except the last, which needs special attention */
    fn set_post_delay(&mut self) {
        for i in 0..self.events.len()-1 {
            let this_end = self.events[i].borrow().block_end();
            let this_label = self.events[i].borrow().unique_label.clone();
            let next_start = self.events[i+1].borrow().block_start();
            let next_label = self.events[i+1].borrow().unique_label.clone();
            println!("end of {}:{} -> start of {}:{}",this_label,this_end,next_label,next_start);
            let difference = next_start - this_end;
            if difference < 0 {panic!("there is something wrong with event placement. Has the queue been solved?")}
            self.events[i].as_ref().borrow_mut().set_post_delay(difference);
        }
    }
    fn unique(&self) -> Vec<Rc<RefCell<Event>>> {
        let mut visited = HashSet::<String>::new();
        let mut unique = Vec::<Rc<RefCell<Event>>>::new();
        for event in self.events.iter(){
            let label = event.borrow().label.clone();
            match visited.contains(&label) {
                false => {
                    unique.push(event.clone());
                    visited.insert(label);
                }
                true => {/* no op */}
            }
        }
        unique
    }
    /** Generates a loop structure that constitutes the majority of the pulse program */
    // todo!( the view loop needs to be incremented according to the number of echos per repetition)
    // if there are 4 acq events, there is assumed to be 4 echos, so the view count needs to be incrementd
    // by 4 at the end of the loop. We also need to check the condition that the total number of view is a multple
    // of the number of echos in the view loop
    pub fn flat_loop_structure(&mut self,repetitions:u32,averages:u16,rep_time:f32,acceleration:u16) -> FlatLoopStructure {
        FlatLoopStructure::new(repetitions,averages,rep_time,self,acceleration)
    }
    pub fn ppl_user_adjustments(&self) -> Option<Vec<Adjustment>> {
        let mut scrollbars = Vec::<Adjustment>::new();
        self.unique().iter().for_each(|event| {
            let adj = event.borrow().execution.block_header_adjustments();
            match adj {
                Some(scroll_bars) => scrollbars.extend(scroll_bars),
                None => {}
            }
        });
        return if scrollbars.len() > 0 {Some(scrollbars)} else {None};
    }
    pub fn ppl_declarations(&self) -> Vec<CommandString> {
        // only want to do this for unique events so things aren't re-declared
        let mut cmd = Vec::<CommandString>::new();
        self.unique().iter().for_each(|event|
                cmd.push(event.borrow().execution.block_declaration())
        );
        cmd
    }
    pub fn ppl_constants(&self) -> Option<Vec<CommandString>> {
        let mut cmds = Vec::<CommandString>::new();
        self.unique().iter().for_each(|event|
            match event.borrow().execution.block_constant_initialization() {
                Some(cmd) => cmds.push(cmd),
                None => {}
            }
        );
        return if cmds.len() > 0 {Some(cmds)} else {None}
    }
    pub fn ppl_initializations(&self) -> Vec<CommandString> {
        let mut cmds = Vec::<CommandString>::new();
        self.unique().iter().for_each(|event|
            cmds.push(event.borrow().execution.block_initialization())
        );
        cmds
    }
    pub fn ppl_acquisition(&self) -> AcquisitionParams {
        // get number of acquires for reporting echos
        let mut acq_events = Vec::<(SpectralWidth,u16,u16)>::new();
        self.events.iter().for_each(|event| {
            match event.borrow().execution.kind() {
                EventType::Acq(sample_rate,n_samples,n_discards) => {
                    acq_events.push((sample_rate,n_samples,n_discards))
                }
                _=> {}
            }
        });
        if acq_events.len() < 1 {panic!("there must be at least 1 acquisition event for the event queue to be valid")}
        // do a consistency check
        for i in 0..acq_events.len()-1 {
            if acq_events[i].0 != acq_events[i+1].0 {panic!("different sample rates detected in single event queue! Only one sample rate is allowed.")}
            if acq_events[i].1 != acq_events[i+1].1 {panic!("different number of samples detected in single event queue! Only one sample count is allowed.")}
            if acq_events[i].2 != acq_events[i+1].2 {panic!("different number of discards detected in single event queue! Only one number of discards is allowed.")}
        }
        AcquisitionParams{
            n_samples:acq_events[0].1.clone(),
            sample_rate:acq_events[0].0.clone(),
            n_discards:acq_events[0].2.clone(),
            n_echos:acq_events.len() as u16
        }
    }
    pub fn ppl_seq_params(&self,sample_period_us:usize) -> (Option<String>,Option<String>) {
        let mut grad_out_str = Vec::<String>::new();
        let mut rf_out_str = Vec::<String>::new();
        // only run this for unique events (based on label)
        let mut grad_h = HashSet::<String>::new();
        let mut rf_h = HashSet::<String>::new();
        self.events.iter().for_each(|event|{
            match event.borrow().execution.kind(){
                EventType::Grad => { // only match for specified type
                    let label = &event.borrow().label; // get the non-unique label
                    match grad_h.contains(label) {
                        false => { // only consider labels that haven't been added to the check list
                            match event.borrow().execution.seq_params(sample_period_us) {
                                Some(param_text) => grad_out_str.push(param_text), // push text to vector
                                None => {/*no op*/}
                            }
                            grad_h.insert(label.clone());
                        }
                        _ => {/*no op*/}
                    }
                }
                EventType::Rf => {
                    let label = &event.borrow().label; // get the non-unique label
                    match rf_h.contains(label) {
                        false => { // only consider labels that haven't been added to the check list
                            match event.borrow().execution.seq_params(sample_period_us) {
                                Some(param_text) => rf_out_str.push(param_text), // push text to vector
                                None => {/*no op*/}
                            }
                            rf_h.insert(label.clone());
                        }
                        _ => {/*no op*/}
                    }
                }
                _=> {}
            }
        });
        let grad_out = if grad_out_str.len() > 0 {Some(grad_out_str.join("\n"))} else {None};
        let rf_out = if rf_out_str.len() > 0 {Some(rf_out_str.join("\n"))} else {None};
        (grad_out,rf_out)
    }
}

pub struct AcquisitionParams {
    pub n_samples:u16,
    pub n_discards:u16,
    pub sample_rate:SpectralWidth,
    pub n_echos:u16
}

#[derive(Clone)]
pub enum GradEventType {
    Blocking,
    NonBlocking
}

pub enum EventPlacementType {
    Origin,
    ExactFromOrigin(i32),
    After(Rc<RefCell<Event>>,u32),
    Before(Rc<RefCell<Event>>,u32),
}