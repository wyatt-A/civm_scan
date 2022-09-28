use std::cell::RefCell;
use crate::acq_event::{AcqEvent, SpectralWidth};
use crate::pulse::{CompositeHardpulse, Hardpulse, Trapezoid};
use crate::rf_state::{RfDriver, RfDriverType, RfStateType};
use crate::rf_event::RfEvent;
use crate::command_string::CommandString;
use crate::gradient_event::GradEvent;
use crate::gradient_matrix::{DacValues, EncodeStrategy, LinTransform, Matrix, MatrixDriver, MatrixDriverType};
use crate::rf_state::PhaseCycleStrategy::LUTNinetyTwoSeventy;
use crate::{ppl_function, utils};
use serde::{Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;
use crate::event_block::{Event, EventPlacementType, GradEventType, EventQueue};
use crate::ppl::{FlatLoopStructure, CalcBlock, Header, DspRoutine, BaseFrequency, ScrollBar, PPL, Orientation, GradClock, VIEW_LOOP_COUNTER_VAR, PhaseUnit};
use crate::ppl_function::acquire;
use crate::seqframe::SeqFrame;

#[derive(PartialEq)]
pub enum EventType {
    Rf,
    Grad,
    Acq(SpectralWidth,u16,u16)
}

#[derive(Clone,Debug,Serialize)]
pub enum WaveformData {
    Rf(PlotTrace,PlotTrace),
    Grad(Option<PlotTrace>,Option<PlotTrace>,Option<PlotTrace>),
    Acq(PlotTrace),
}

#[derive(Debug,Clone,Serialize)]
pub struct PlotTrace {
    pub x:Vec<f32>,
    pub y:Vec<f32>
}

impl PlotTrace {
    pub fn new(x: Vec<f32>, y: Vec<f32>) -> Self {
        if x.len() != y.len() {
            panic!("vectors must be the same length");
        }
        Self {
            x,
            y
        }
    }
}

pub struct BlockExecution {
    body:CommandString,
    post_delay_clocks:i32
}

impl BlockExecution {
    pub fn new(cmd_string:CommandString,post_delay_clocks:i32) -> Self {
        Self {
            body:cmd_string,
            post_delay_clocks
        }
    }
    pub fn cmd_string(&self) -> CommandString {
        CommandString::new_hardware_exec(&vec![
            self.body.commands.clone(),
            ppl_function::delay(self.post_delay_clocks)
        ].join("\n"))
    }
}

pub trait ExecutionBlock {
    // time needed to launch an event. blocks cannot overlap
    fn block_duration(&self) -> i32;
    // time to event start
    fn time_to_start(&self) -> i32;
    // time to event end. This can be shorter than block duration (such as for acq event)
    fn time_to_end(&self) -> i32;
    // time to event center (half of the event duration)
    fn time_to_center(&self) -> i32;
    // start of the block with respect to event center
    fn block_start(&self) -> i32 {
        -self.time_to_center()
    }
    // code the executes sequence events on the hardware
    fn block_execution(&self,post_delay:i32) -> BlockExecution;
    // Optional header fields for special variables
    fn block_header_adjustments(&self) -> Option<Vec<ScrollBar>>;
    // constant declarations (have to be done ahead of variable inits)
    fn block_constant_initialization(&self) -> Option<CommandString>;
    // variable initializations for execution block
    fn block_initialization(&self) -> CommandString;
    // variable initializations required by execution block
    fn block_declaration(&self) -> CommandString;
    // code to be run in a calculation block before execution
    fn block_calculation(&self) -> Option<CommandString>;
    // return a reference to self stored on heap
    fn as_reference(&self) -> Box<dyn ExecutionBlock>;
    // label for the block
    fn label(&self) -> String;
    // render the label to waveform data
    fn render_normalized(&self, time_step_us:usize) -> WaveformData;
    fn kind(&self) -> EventType;
    fn blocking(&self) -> bool;
    fn seq_params(&self,sample_period_us:usize) -> Option<String>;
    fn render_magnitude(&self,time_step_us:usize,driver_value:u32) -> WaveformData;
}