/*
 A pulse (in this module) is defined as a waveform that starts with a magnitude of 0
 and ends with a magnitude of 0. The waveform can have a positive, negative, or both polarities
 between its start and end points.

 All pulses are normalized to have a maximum unit magnitude. This allows for the calculation of
 normalized power to calculate things like flip angle and k-space traversal.

 All pulses should default to have a max amplitude of 1.0 except for some special cases. This is
 important because these pulses get scaled later to get run on hardware.

 */

use std::f32::consts::PI;
use crate::execution::PlotTrace;
use crate::pulse_function::{Function, FunctionParams};
use crate::_utils;
use utils;

pub trait Pulse {
    fn duration(&self) -> f32;
    fn function(&self,time_step_us:usize) -> Vec<Function>;
    fn n_samples(&self,time_step:usize) -> usize {
        self.function(time_step).iter().map(|f| f.n_samples()).sum()
    }
    fn power_net(&self,magnitude:f32) -> f32;
    fn magnitude_net(&self,power_net:f32) -> f32;
    fn power_abs(&self,magnitude:f32) -> f32;
    fn render(&self,time_step_us:usize) -> Vec<f32> {
        let mut waveform = Vec::<f32>::with_capacity(self.n_samples(time_step_us));
        for f in self.function(time_step_us).iter(){
            waveform.extend(f.waveform_data());
        }
        waveform
    }
    fn render_normalized(&self, time_step_us:usize) -> PlotTrace {
        let y = self.render(time_step_us);
        let x = (0..self.n_samples(time_step_us)).map(|i| _utils::us_to_sec((i*time_step_us) as i32)).collect();
        PlotTrace::new(x,y)
    }
    fn render_magnitude(&self,time_step:usize,dac:i16) -> PlotTrace {
        let mut pt = self.render_normalized(time_step);
        pt.y.iter_mut().for_each(|value| *value *= dac as f32);
        pt
    }
}

pub trait SliceSelective:Pulse {
    fn bandwidth(&self) -> f32 {
        let w = self.render(2);
        let c = utils::real_to_complex(&w);
        utils::bandwidth(&c,2.0E-6)
    }
    fn slice_thickness_mm(&self,grad_strength_hzpmm:f32) -> f32 {
        self.bandwidth()/grad_strength_hzpmm
    }
    fn grad_strength_hzpmm(&self,slice_thickness_mm:f32) -> f32 {
        self.bandwidth()/slice_thickness_mm
    }
}


/*
    Basic trapezoid pulse typically for gradient activity
 */

#[derive(Clone,Copy)]
pub struct Trapezoid {
    pub ramp_time:f32,
    pub plateau_time:f32,
}

impl Trapezoid {
    pub fn new(ramp_time:f32,plateau_time:f32) -> Trapezoid {
        assert!(ramp_time > 0.0,"ramp time must be positive");
        assert!(plateau_time >= 0.0,"plateau time must be positive or 0");
        Trapezoid{ramp_time,plateau_time}
    }
}

impl Pulse for Trapezoid {
    fn duration(&self) -> f32 {
        2.0*self.ramp_time + self.plateau_time
    }
    fn function(&self,time_step_us:usize) -> Vec<Function>{
        let n_ramp_samples = _utils::sec_to_samples(self.ramp_time, time_step_us);
        let n_plateau_samples = _utils::sec_to_samples(self.plateau_time, time_step_us);
        let ramp_params = FunctionParams::new(n_ramp_samples,1.0);
        let plat_params = FunctionParams::new(n_plateau_samples,1.0);
        vec![
            Function::RampUp(ramp_params),
            Function::Plateau(plat_params),
            Function::RampDown(ramp_params)
        ]
    }
    fn power_net(&self,magnitude:f32) -> f32 {
        magnitude*(self.ramp_time + self.plateau_time)
    }
    fn magnitude_net(&self,power:f32) -> f32 {
        power/(self.ramp_time + self.plateau_time)
    }
    fn power_abs(&self,magnitude:f32) -> f32 {
        self.power_net(magnitude).abs()
    }
}


/*
    Half-Sin pulse representing sin(x) for 0 < x < pi.
    This is commonly used for our diffusion gradient lobes
 */

#[derive(Clone,Copy)]
pub struct HalfSin {
    pub duration:f32,
}

impl HalfSin {
    pub fn new(duration:f32) -> HalfSin {
        assert!(duration > 0.0,"duration must be positive");
        HalfSin{duration}
    }
}

impl Pulse for HalfSin {
    fn duration(&self) -> f32 {
        self.duration
    }
    fn function(&self, time_step_us: usize) -> Vec<Function> {
        let params = FunctionParams::new(_utils::sec_to_samples(self.duration, time_step_us), 1.0);
        vec![Function::HalfSin(params)]
    }
    fn power_net(&self, magnitude: f32) -> f32 {
        2.0*magnitude*self.duration/PI
    }
    fn magnitude_net(&self, power_net: f32) -> f32 {
        power_net*PI/(2.0*self.duration)
    }
    fn power_abs(&self, magnitude: f32) -> f32 {
        2.0*magnitude*self.duration/PI
    }
}

/*
    Basic rectangular pulse used for full-volume RF excitation
 */

#[derive(Clone,Copy)]
pub struct Hardpulse {
    duration:f32
}

impl Hardpulse {
    pub fn new(duration:f32) -> Hardpulse {
        Hardpulse{duration}
    }
    pub fn bandwidth_hz(&self) -> f32 {
        1.0/(4.0*self.duration)
    }
}

impl SliceSelective for Hardpulse{}
impl Pulse for Hardpulse {
    fn duration(&self) -> f32 {
        self.duration
    }
    fn function(&self,time_step_us:usize) -> Vec<Function>{
        let n_central_samples = _utils::sec_to_us(self.duration()) as usize/time_step_us;
        let central_pulse = FunctionParams::new(n_central_samples,1.0);
        let end_point = FunctionParams::new(1,0.0);
        vec![
            Function::Plateau(end_point),
            Function::Plateau(central_pulse),
            Function::Plateau(end_point)
        ]
    }
    fn power_net(&self,magnitude:f32) -> f32 {
        magnitude*self.duration
    }
    fn magnitude_net(&self, power_net: f32) -> f32 {
        power_net/self.duration
    }
    fn power_abs(&self,magnitude:f32) -> f32 {
        self.power_net(magnitude).abs()
    }
}


/*
    Composite hard pulse exclusively used for 180 degree flip angles
    This is implemented as a hard pulse with phase divisions
 */

#[derive(Clone)]
pub struct CompositeHardpulse {
    duration:f32,
    pub phase_divisions:Vec<f32>
}

impl CompositeHardpulse {
    pub fn new_180(duration:f32) -> CompositeHardpulse{
        CompositeHardpulse{
            duration,
            phase_divisions:vec![0.0,90.0,90.0,0.0]
        }
    }
}

impl SliceSelective for CompositeHardpulse{}
impl Pulse for CompositeHardpulse {
    fn duration(&self) -> f32 {
        self.duration
    }
    fn function(&self,time_step_us:usize) -> Vec<Function>{
        let n_central_samples = _utils::sec_to_us(self.duration()) as usize/time_step_us;
        let central_pulse = FunctionParams::new(n_central_samples,1.0);
        let end_point = FunctionParams::new(1,0.0);
        vec![
            Function::Plateau(end_point),
            Function::Plateau(central_pulse),
            Function::Plateau(end_point)
        ]
    }
    fn power_net(&self,magnitude:f32) -> f32 {
        magnitude*self.duration
    }
    fn magnitude_net(&self, power_net: f32) -> f32 {
        power_net/self.duration
    }
    fn power_abs(&self,magnitude:f32) -> f32 {
        self.power_net(magnitude).abs()
    }
}


/*
    Generic sinc pulse for slice-selective rf excitation and refocusing
 */

pub struct SincPulse {
    duration:f32,
    n_lobes:u16,
}

impl SincPulse {
    pub fn new(duration:f32,lobes:u16) -> Self{
        let lobes = if lobes%2 == 0 {lobes+1} else {lobes};
        Self {
            duration,
            n_lobes: lobes
        }
    }
}

impl SliceSelective for SincPulse{}
impl Pulse for SincPulse {
    fn duration(&self) -> f32 {
        self.duration
    }

    fn function(&self, time_step_us: usize) -> Vec<Function> {
        let n = (self.duration/ _utils::us_to_sec(time_step_us as i32) as f32).floor() as usize;
        let p = FunctionParams::new(n,1.0);
        vec![
            Function::Sinc(self.n_lobes,p),
        ]
    }

    fn power_net(&self, magnitude: f32) -> f32 {
        let a = utils::abs(&self.render(2));
        let p = utils::trapz(&a,Some(2.0E-6));
        p*magnitude
    }

    fn magnitude_net(&self, power_net: f32) -> f32 {
        let a = utils::abs(&self.render(2));
        let p = utils::trapz(&a,Some(2.0E-6));
        power_net/p
    }

    fn power_abs(&self, magnitude: f32) -> f32 {
        self.power_net(magnitude)
    }
}



/*
    A special gradient pulse shape the simplifies 2-d slice selective refocusing.
    There is a crusher built-in to either side of a slice-select gradient.
    The crush ratio is the relative amplitude of the first and last lobe to the central plateau
           ____             ____
          /    \           /    \
         /      \_________/      \
        /                         \
    ___/                           \___
 */

pub struct SliceSelectiveCrusher {
    crush_ratio:f32,
    selection_duration:f32,
    crush_duration:f32,
    ramp_time:f32,
}

impl SliceSelectiveCrusher {
    pub fn new(slice_select_duration:f32,crush_duration:f32,ramp_time:f32) -> Self {
        Self {
            crush_ratio: 2.0,
            selection_duration: slice_select_duration,
            crush_duration,
            ramp_time,
        }
    }

    fn normalized_power(&self) -> f32 {
        let trapezoid_1 = self.crush_ratio*(self.ramp_time + 2.0*self.crush_duration + self.selection_duration);
        let trapezoid_2 = (self.crush_ratio - 1.0)*(self.crush_duration + self.ramp_time);
        trapezoid_1 - trapezoid_2
    }
}

impl Pulse for SliceSelectiveCrusher {
    fn duration(&self) -> f32 {
        self.selection_duration + 2.0*self.crush_duration + 4.0*self.ramp_time
    }

    fn function(&self, time_step_us: usize) -> Vec<Function> {
        let n_ramp_samples = _utils::sec_to_samples(self.ramp_time, time_step_us);
        let n_crush_plateau_samples = _utils::sec_to_samples(self.crush_duration, time_step_us);
        let n_select_plateau_samples = _utils::sec_to_samples(self.selection_duration, time_step_us);

        let ramp_up1 = Function::RampUp(FunctionParams::new(n_ramp_samples,self.crush_ratio));
        let ramp_up2 = Function::RampUpFrom(1.0,FunctionParams::new(n_ramp_samples,self.crush_ratio));

        let ramp_down1 = Function::RampDownTo(1.0,FunctionParams::new(n_ramp_samples,self.crush_ratio));
        let ramp_down2 = Function::RampDown(FunctionParams::new(n_ramp_samples,self.crush_ratio));

        let crush_plat = Function::Plateau(FunctionParams::new(n_crush_plateau_samples,self.crush_ratio));
        let select_plat = Function::Plateau(FunctionParams::new(n_select_plateau_samples,1.0));

        vec![
            ramp_up1,
            crush_plat.clone(),
            ramp_down1,
            select_plat,
            ramp_up2,
            crush_plat,
            ramp_down2
        ]
    }

    fn power_net(&self, magnitude: f32) -> f32 {
        magnitude*self.normalized_power()
    }

    fn magnitude_net(&self, power_net: f32) -> f32 {
        power_net/self.normalized_power()
    }

    fn power_abs(&self, magnitude: f32) -> f32 {
        self.power_net(magnitude)
    }
}