use crate::pulse::{Pulse,Trapezoid, Hardpulse, CompositeHardpulse};
use crate::seqframe::{self, SeqFrame, FrameType, SeqFrameExpression};
use crate::utils;
use crate::pulse_function::{Function,FunctionParams};

pub const RF_MAX_DAC:i16 = 2047;

// this is like inheritance, forcing whatever implements GradFrame to also implement Pulse
pub trait RfFrame:Pulse {
    fn amplitude_function(&self,sample_period_us:usize) -> Vec<Function>{
        self.function(sample_period_us)
    }
    fn phase_function(&self,sample_period_us:usize) -> Vec<Function>{
        let n_samples = self.n_samples(sample_period_us);
        vec![
            Function::Plateau(FunctionParams::new(n_samples,0.0))
        ]
    }
    fn amplitude_expression(&self, sample_period_us:usize) -> Vec<seqframe::Expression> {
        self.function(sample_period_us).iter().map(|func| func.expression(RF_MAX_DAC)).collect()
    }
    fn phase_expression(&self, sample_period_us:usize) -> Vec<seqframe::Expression>{
        self.phase_function(sample_period_us).iter().map(|func| func.expression(0)).collect()
    }
    fn rf_seq_frame(&self,label:&str,sample_period_us:usize) -> (SeqFrame,SeqFrame){
        let amplitude = self.amplitude_expression(sample_period_us);
        let phase = self.phase_expression(sample_period_us);
        (
            SeqFrame::from_expressions(amplitude,label,sample_period_us,FrameType::Rf),
            SeqFrame::from_expressions(phase,label,sample_period_us,FrameType::RfPhase)
        )
    }
}

// the default behavior is adequate for a trapezoid
impl RfFrame for Trapezoid {}

impl RfFrame for Hardpulse {}

impl RfFrame for CompositeHardpulse {
    fn phase_function(&self,sample_period_us:usize) -> Vec<Function>{
        let n_samples = self.n_samples(sample_period_us);
        let n_per_phase_div = n_samples/self.phase_divisions.len();
        let remainder = n_samples - self.phase_divisions.len()*n_per_phase_div;
        let outer = FunctionParams::new(n_per_phase_div,0.0);
        let inner = FunctionParams::new(2*n_per_phase_div+remainder,1.0);
        vec![
            Function::Plateau(outer),
            Function::Plateau(inner),
            Function::Plateau(outer)
        ]
    }
    fn phase_expression(&self, sample_period_us:usize) -> Vec<seqframe::Expression>{
        let dac_val = 90;// degrees
        self.phase_function(sample_period_us).iter().map(|func| func.expression(dac_val)).collect()
    }
}


#[test]
fn test(){
    println!("rf frame test ...");
    let t = Trapezoid::new(100E-6,2E-3);
    let s = t.rf_seq_frame("rfpulse",2);
    let h = CompositeHardpulse::new_180(100E-6);
    let hs = h.rf_seq_frame("hardpulsecomp",2);
    println!("{}",s.0.serialize());
    println!("{}",s.1.serialize());
    println!("{}",hs.0.serialize());
    println!("{}",hs.1.serialize());
}