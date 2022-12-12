#[derive(Copy,Clone)]
pub struct FunctionParams {
    pub n_samples:usize,
    pub max_value:f32
}

impl FunctionParams{
    pub fn new(n_samples:usize,max_value:f32) -> Self {
        let params = Self {
            n_samples,
            max_value
        };
        params
    }
}

#[derive(Clone)]
pub enum Function {
    RampUp(FunctionParams),
    RampDown(FunctionParams),
    HalfSin(FunctionParams),
    Plateau(FunctionParams),
    Sinc(u16,FunctionParams),
    RampDownTo(f32,FunctionParams),
    RampUpFrom(f32,FunctionParams)
}

impl Function {
    pub fn waveform_data(&self) -> Vec<f32>{
        match self {
            Function::RampUp(p) => {
                ramp(0.0, p.max_value, p.n_samples)
            }
            Function::RampDown(p) => {
                ramp(p.max_value, 0.0, p.n_samples)
            }
            Function::Plateau(p) => {
                ramp(p.max_value, p.max_value, p.n_samples)
            }
            Function::HalfSin(p) => {
                half_sin(p.max_value, p.n_samples)
            }
            Function::Sinc(n_lobes,p) => {
                sinc(p.max_value, *n_lobes, p.n_samples)
            }
            Function::RampDownTo(intermediate,p) => ramp(p.max_value,*intermediate,p.n_samples),
            Function::RampUpFrom(intermediate,p) => ramp(*intermediate,p.max_value,p.n_samples),
        }
    }
    pub fn n_samples(&self) -> usize {
        self.waveform_data().len()
    }
}

pub fn render_function_vector(functions:Vec<Function>) -> Vec<f32> {
    let mut waveform = Vec::<f32>::new();
    for f in functions.iter(){
        waveform.extend(f.waveform_data());
    }
    waveform
}

fn ramp(start:f32,end:f32,n_samples:usize) -> Vec<f32>{
    let step = (end-start)/(n_samples-1) as f32;
    let mut v:Vec<f32> = (0..n_samples).map(|i| start+((i as f32)*step)).collect();
    v.pop();
    v.push(end);
    v
}

fn half_sin(amplitude:f32,n_samples:usize) -> Vec<f32> {
    //"{}*sin(PI*(Ñ/({}-1)))"
    let pi = std::f32::consts::PI;
    let nm1 = (n_samples - 1) as f32;
    (0..n_samples).map(|i|((pi*(i as f32)/nm1).sin()*amplitude)).collect()
}

fn sinc(amplitude:f32,n_lobes:u16,n_samples:usize) -> Vec<f32> {
    //"{}*sinc(PI*{}*((Ñ-({}/2))/({}/2)))",p.max_dac,lobe_val,p.n_samples,p.n_samples)
    let lobes = if n_lobes%2 == 0 {n_lobes+1} else {n_lobes};
    let lobe_val = ((lobes + 1)/2) as f32;
    let pi = std::f32::consts::PI;
    let nsamp_over_2 = n_samples as f32/2.0;
    let n:Vec<f32> = (0..n_samples).map(|i| i as f32).collect();
    let args:Vec<f32> = n.iter().map(|i| pi*lobe_val*((i-nsamp_over_2)/nsamp_over_2)).collect();
    args.iter().map(|i|
        match i {
            0.0 => amplitude,
            _=> amplitude*(i.sin()/i)
        }
    ).collect()
}