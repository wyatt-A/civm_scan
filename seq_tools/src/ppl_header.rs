use crate::acq_event::SpectralWidth;
use crate::ppl_constants::{FREQ_OFFSET_MAX, FREQ_OFFSET_MIN};
use crate::ppl_function;
use serde::{Deserialize, Serialize};
use crate::hardware_constants::RF_MAX_DAC;


pub struct Header {
    pub dsp_routine:DspRoutine,
    pub receiver_mask:u16,
    pub base_frequency:BaseFrequency,
    pub samples:u16,
    pub spectral_width: SpectralWidth,
    pub sample_discards:u16,
    pub repetitions:u32,
    pub echos:u16,
    pub echo_divisor:u16,
    pub averages:u16,
    pub user_adjustments:Option<Vec<Adjustment>>
}

pub enum DspRoutine {
    Dsp
}

impl DspRoutine {
    pub(crate) fn print(&self) -> String {
        match self {
            DspRoutine::Dsp =>
                String::from("DSP_ROUTINE \"dsp\";")
        }
    }
    pub fn print_ppr(&self) -> String {
        match self {
            DspRoutine::Dsp =>
                String::from(":DSP_ROUTINE dsp")
        }
    }
}

#[derive(Clone,Serialize,Deserialize)]
pub struct BaseFrequency {
    base_freq:f32,
    obs_offset:f32
}

impl BaseFrequency {

    pub fn civm9p4t(offset:f32) -> Self {
        Self {
            base_freq:30171576.0,
            obs_offset:offset
        }
    }
    pub(crate) fn print(&self) -> String {
        format!("OBSERVE_FREQUENCY \"9.4T 1H\",{},{},{},MHz, kHz, Hz, rx1MHz;",
                FREQ_OFFSET_MIN,FREQ_OFFSET_MAX,self.obs_offset)
    }
    pub(crate) fn print_ppr(&self) -> String {
        format!(":OBSERVE_FREQUENCY \"9.4T 1H\", {:.1}, MHz, kHz, Hz, rx1MHz"
                ,self.base_freq+self.obs_offset)
    }
    pub fn set_freq_buffer(&self) -> String {
        ppl_function::set_base_freq()
    }
}

pub enum AdjustmentInterface {
    Scrollbar,
    Text,
}

pub struct Adjustment {
    interface:AdjustmentInterface,
    title:String,
    title_hint:String,
    target_var:String,
    min:i16,
    max:i16,
    scale:f32,
    default:i16,
}

impl Adjustment {
    pub fn new_rf_pow_adj(label:&str,target_var:&str,default_val:i16) -> Self {
        Self {
            title:format!("{} dac percent",label),
            title_hint:String::from("%"),
            target_var:String::from(target_var),
            min:0,
            max:RF_MAX_DAC,
            scale:RF_MAX_DAC as f32/100.0,
            default:default_val,
            interface:AdjustmentInterface::Scrollbar
        }
    }
    pub fn new_rf_phase_adj(label:&str,target_var:&str,default_val:i16) -> Self {
        Self {
            title:format!("{} phase adjustment",label),
            title_hint:String::from("400=90deg"),
            target_var:String::from(target_var),
            min:-800,
            max:800,
            scale:1.0,
            default:default_val,
            interface:AdjustmentInterface::Scrollbar
        }
    }
    pub fn new_grad_adj(label:&str,target_var:&str,half_range:i16) -> Self {
        Self {
            title:format!("{}",label),
            title_hint:String::from("dac"),
            target_var:String::from(target_var),
            min:-half_range,
            max:half_range,
            scale:1.0,
            default:0,
            interface:AdjustmentInterface::Text
        }
    }
    pub(crate) fn print(&self) -> String {
        match self.interface {
            AdjustmentInterface::Text => {
                format!("EDITTEXT \"{}\",\"{}\",\"%.2f\",{},{},{},{},{};",
                        self.title,self.title_hint,self.min,self.max,self.default,self.scale,self.target_var,)
            },
            AdjustmentInterface::Scrollbar => {
                format!("SCROLLBAR \"{}\",\"{}\",\"%.2f\",{},{},{},{},{};",
                        self.title,self.title_hint,self.min,self.max,self.default,self.scale,self.target_var,)
            }
        }

    }
    pub(crate) fn print_ppr(&self) -> String {
        format!(":VAR {}, {}",self.target_var,self.default)
    }
}