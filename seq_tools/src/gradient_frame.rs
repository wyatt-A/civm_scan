use crate::pulse::{Pulse,Trapezoid};
use crate::pulse_function::Function;
use crate::seqframe::{self, SeqFrame, FrameType, SeqFrameExpression};
use crate::utils;

const GRAD_MAX_DAC:i16 = 32767;

// this is like inheritance, forcing whatever implements GradFrame to also implement Pulse
pub trait GradFrame:Pulse {
    fn amplitude_function(&self,sample_period_us:usize) -> Vec<Function>{
        self.function(sample_period_us)
    }
    fn amplitude_expression(&self, sample_period_us:usize) -> Vec<seqframe::Expression> {
        self.function(sample_period_us).iter().map(|func| func.expression(GRAD_MAX_DAC)).collect()
    }
    fn grad_seq_frame(&self,label:&str,sample_period_us:usize) -> SeqFrame{
        let expressions = self.amplitude_expression(sample_period_us);
        return SeqFrame::from_expressions(expressions,label,sample_period_us,FrameType::Grad);
    }
}

impl GradFrame for Trapezoid {}

#[test]
fn test(){
    println!("gradient seqframe test ...");
    let t = Trapezoid::new(100E-6,2E-3);
    let s = t.grad_seq_frame("trap",2);
    println!("{}",s.serialize());
}