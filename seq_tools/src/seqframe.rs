use encoding::{Encoding, EncoderTrap};
use encoding::all::ISO_8859_1;
use std::path::{Path,PathBuf};
use crate::utils;
use std::io::Write;
use std::fs::File;
use crate::pulse_function::{Function, FunctionParams};

const CLOCK_PERIOD_NS:usize = 100;
const GRAD_CLOCK_MULTIPLIER:usize = 20; // this means that the gradient clock period is 2 us
const RF_CLOCK_MULTIPLIER:usize = 1; // this means that the min rf clock period is 100 ns
pub const GRAD_SEQ_FILE_LABEL:&str = "civm_grad";
pub const RF_SEQ_FILE_LABEL:&str = "civm_rf";

pub enum FrameType{
    Grad,
    Rf,
    RfPhase
}

pub enum FrameChannel{
    Grad,
    RfAmplitude,
    RfPhase
}

impl FrameType {

    pub fn rf_max_dac() -> i16{
        2047
    }
    pub fn rf_min_dac() -> i16{
        -2048
    }
    pub fn rf_phase_max_dac() -> i16{
        360
    }
    pub fn rf_phase_min_dac() -> i16{
        0
    }
    pub fn grad_max_dac() -> i16{
        32767
    }
    pub fn grad_min_dac() -> i16{
        -32768
    }

    pub fn max_dac(&self) -> i16{
        return match self {
            FrameType::Grad => FrameType::grad_max_dac(),
            FrameType::Rf => FrameType::rf_max_dac(),
            FrameType::RfPhase => FrameType::rf_phase_max_dac()
        }
    }

    pub fn min_dac(&self) -> i16{
        return match self {
            FrameType::Grad => FrameType::grad_min_dac(),
            FrameType::Rf => FrameType::rf_min_dac(),
            FrameType::RfPhase => FrameType::rf_phase_min_dac()
        }
    }

    pub fn cycles_per_sample(&self) -> usize {
        return match self {
            FrameType::Grad => 1, // this is misleading because this is actually a multiplier for the clock() command
            FrameType::Rf => 20, // this is 20 clock cycles per sample (each clock cycle takes 100 ns)
            FrameType::RfPhase => 20
        }
    }
}

pub struct Expression {
    n_samples:usize,
    text:String
}

pub trait SeqFrameExpression {
    fn expression(&self,dac_scale:i16) -> Expression;
}

impl SeqFrameExpression for Function {
    fn expression(&self,dac_scale:i16) -> Expression {

        match self {
            Function::RampUp(p) => {
                let dac = ((dac_scale as f32)*p.max_value) as i16;
                Expression {
                    n_samples:p.n_samples,
                    text: format!("ramp(0,{})",dac)
                }
            }
            Function::RampDown(p) => {
                let dac = ((dac_scale as f32)*p.max_value) as i16;
                Expression {
                    n_samples:p.n_samples,
                    text: format!("ramp({},0)",dac)
                }
            }
            Function::HalfSin(p) => {
                let dac = ((dac_scale as f32)*p.max_value) as i16;
                Expression {
                    n_samples:p.n_samples,
                    text: format!("{}*sin(PI*(Ñ/({}-1)))", dac, p.n_samples)
                }
            }
            Function::Plateau(p) => {
                let dac = ((dac_scale as f32)*p.max_value) as i16;
                Expression {
                    n_samples:p.n_samples,
                    text: format!("{}",dac)
                }
            }
            Function::Sinc(n_lobes,p) => {
                let dac = ((dac_scale as f32)*p.max_value) as i16;
                let lobes = if n_lobes%2 == 0 {n_lobes+1} else {*n_lobes};
                let lobe_val = (lobes + 1)/2;
                Expression {
                    n_samples:p.n_samples,
                    text: format!("{}*sinc(PI*{}*((Ñ-({}/2))/({}/2)))", dac, lobe_val, p.n_samples, p.n_samples)
                }
            }
        }
    }
}

// #[derive(Copy,Clone)]
// pub struct FunctionParams {
//     n_samples:usize,
//     max_dac:i16
// }
//
// impl FunctionParams{
//     pub fn new(n_samples:usize,max_dac:i16) -> Self {
//         Self {
//             n_samples,
//             max_dac
//         }
//     }
// }
//
// pub enum Function {
//     RampUp(FunctionParams),
//     RampDown(FunctionParams),
//     HalfSin(FunctionParams),
//     Plateau(FunctionParams),
//     Sinc(u16,FunctionParams)
// }






// impl Expression {
//
//     pub fn ramp_up(n_samples:usize,max_dac:i16) -> Expression {
//         Expression {
//             n_samples,
//             text: format!("ramp(0,{})",max_dac)
//         }
//     }
//
//     pub fn ramp_down(n_samples:usize,max_dac:i16) -> Expression {
//         Expression {
//             n_samples,
//             text: format!("ramp({},0)",max_dac)
//         }
//     }
//
//     pub fn half_sin(n_samples:usize,max_dac:i16) -> Expression{
//         Expression {
//             n_samples,
//             text: format!("{}*sin(PI*(Ñ/({}-1)))",max_dac,n_samples)
//         }
//     }
//
//     pub fn plateau(n_samples:usize,max_dac:i16) -> Expression{
//         Expression {
//             n_samples,
//             text: format!("{}",max_dac)
//         }
//     }
//
//     pub fn sinc(n_lobes:u8,n_samples:usize,max_dac:i16) -> Expression{
//         let lobes = if n_lobes%2 == 0 {n_lobes+1} else {n_lobes};
//         let lobe_val = (lobes + 1)/2;
//         Expression {
//             n_samples,
//             text: format!("{}*sinc(PI*{}*((Ñ-({}/2))/({}/2)))",max_dac,lobe_val,n_samples,n_samples)
//         }
//     }
// }

#[derive(Debug)]
pub struct SeqFrame {
    pub n_samples:usize,
    pub cycles_per_sample:usize,
    pub channel:u8,
    pub overwrite:bool,
    pub label:String,
    pub function:String
}

impl SeqFrame {

    pub fn from_expressions(expressions:Vec<Expression>,label:&str,sample_period_us:usize,target:FrameType) -> SeqFrame {

        let cycles_per_sample = match target {
            FrameType::Grad => 1000*sample_period_us/(GRAD_CLOCK_MULTIPLIER*CLOCK_PERIOD_NS),
            FrameType::Rf | FrameType::RfPhase => 1000*sample_period_us/(RF_CLOCK_MULTIPLIER*CLOCK_PERIOD_NS)
        };

        let n_samples = expressions.iter().fold(0,|total,exp|total+exp.n_samples);
        // create options from frame target
        let opts:(u8,Option<&str>) = match target {
            FrameType::Grad => (0,None),
            FrameType::Rf => (0,None),
            FrameType::RfPhase => (1,Some("P"))
        };
        // get sample suffix from target options
        let suffix = match opts.1 {
            Some(suffix) => suffix,
            None => ""
        };
        // build function string
        let line:Vec<String> = expressions.iter().map(|exp|
            format!("{}{},{};",exp.n_samples,suffix,exp.text)
        ).collect();
        let function:String = line.join("");
        SeqFrame{
            n_samples,
            cycles_per_sample,
            channel:opts.0,
            overwrite:true,
            label:label.to_owned(),
            function
        }
    }

    pub fn write(&self,filename:&Path){
        let bytes = self.serialize_as_bytes();
        let mut f = File::create(filename).expect("cannot create file");
        f.write_all(&bytes).expect("trouble writing to file");
    }

    pub fn serialize(&self) -> String {
        let s:Vec<String> = vec![
            self.n_samples.to_string(),
            self.cycles_per_sample.to_string(),
            self.channel.to_string(),
            self.label.to_owned(),
            (self.overwrite as u8).to_string(),
            self.function.clone()
            ];
        return s.join("\t");
    }

    pub fn serialize_as_bytes(&self) -> Vec<u8>{
        return ISO_8859_1.encode(&self.serialize(),EncoderTrap::Strict).expect("cannot encode string");
    }

    pub fn format_as_bytes(text:&str) -> Vec<u8> {
        return ISO_8859_1.encode(text,EncoderTrap::Strict).expect("cannot encode string");
    }
}