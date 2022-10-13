use std::borrow::Borrow;
use std::cell::RefCell;
use std::f32::consts::PI;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;
use crate::event_block::{Event, GradEventType};
use crate::event_block::EventPlacementType::{After, Origin, ExactFromOrigin};
use crate::execution::{ExecutionBlock, PlotTrace, WaveformData};
use crate::grad_cal;
use crate::gradient_event::GradEvent;
use crate::gradient_matrix::{DacValues, Matrix};
use crate::pulse::{Pulse, Trapezoid};
use crate::utils::linspace;

#[test]
fn test(){
    println!("diffusion test");

    let diffusion_waveform = Trapezoid::new(500E-6,4E-3);

    let channel_dacs = solve_dac(Rc::new(Box::new(diffusion_waveform)),10E-3,20E-3,30E-3,(1.0,0.0,0.0),3000.0);

    println!("{:?}",channel_dacs);
}

fn solve_dac(diff_pulse:Rc<Box<dyn Pulse>>,pulse1_center:f32,pulse2_center:f32,echo_time:f32,direction:(f32,f32,f32),bvalue:f32) -> (i16,i16,i16) {
    let mut max_dac = 32767;
    let mut min_dac = 0;

    let max_bval = find_bvalue(&diff_pulse.clone(),pulse1_center,pulse2_center,echo_time,direction,max_dac);
    let min_bval = find_bvalue(&diff_pulse.clone(),pulse1_center,pulse2_center,echo_time,direction,min_dac);

    if bvalue > max_bval || bvalue < min_bval {
        panic!("bvalue out of range!");
    }

    while (max_dac - min_dac).abs() > 1 {
        let current_dac = (max_dac + min_dac) / 2;
        let current_bval = find_bvalue(&diff_pulse.clone(), pulse1_center, pulse2_center, echo_time, direction, current_dac);
        if current_bval > bvalue {
            max_dac = current_dac;
        } else if current_bval <= bvalue {
            min_dac = current_dac
        }
    }

    let norm = (direction.0.powi(2) + direction.1.powi(2) + direction.2.powi(2)).sqrt();
    let dir = (direction.0/norm,direction.1/norm,direction.2/norm);
    ((dir.0*min_dac as f32) as i16,(dir.1*min_dac as f32) as i16,(dir.2*min_dac as f32) as i16)
}

fn find_bvalue(diff_pulse:&Box<dyn Pulse>,pulse1_center:f32,pulse2_center:f32,echo_time:f32,direction:(f32,f32,f32),dac:i16) -> f32 {

    let norm = (direction.0.powi(2) + direction.1.powi(2) + direction.2.powi(2)).sqrt();
    let dir = (direction.0/norm,direction.1/norm,direction.2/norm);

    let channel_dacs = ((dir.0*dac as f32),(dir.1*dac as f32),(dir.2*dac as f32));

    let mut r = diff_pulse.render(1);
    let t = linspace(-diff_pulse.duration()/2.0,diff_pulse.duration()/2.0,r.len());

    let t1:Vec<f32> = t.iter().map(|point| *point + pulse1_center).collect();
    let t2:Vec<f32> = t.iter().map(|point| *point + pulse2_center).collect();

    let mut wav = Vec::<f32>::new();

    let mut time = Vec::<f32>::new();

    time.extend(t1);
    time.extend(t2);

    wav.extend(r.clone());
    wav.extend(r);

    let n = 2000;
    let t_resample = linspace(0.0,echo_time,n);
    let dt = t_resample[1] - t_resample[0];

    let wavr = resample(&time,&wav,&t_resample);

    let gread:Vec<f32> = wavr.iter().map(|val| grad_cal::dac_to_tesla_per_meter((*val*channel_dacs.0) as i16)).collect();
    let gphase:Vec<f32> = wavr.iter().map(|val| grad_cal::dac_to_tesla_per_meter((*val*channel_dacs.1) as i16)).collect();
    let gslice:Vec<f32> = wavr.iter().map(|val| grad_cal::dac_to_tesla_per_meter((*val*channel_dacs.2) as i16)).collect();

    // let mut file = File::create(r"/mnt/c/Users/MRS/Desktop/diffusion_test/out.txt").unwrap();
    // let mut s = String::new();
    // for i in 0..gread.len() {
    //     s.push_str(&format!("{},{},{},{}\n",t_resample[i],gread[i],gphase[i],gslice[i]))
    // }
    // file.write_all(s.as_bytes()).unwrap();

    let f_read = cumptrapz(&gread,dt);
    let f_phase = cumptrapz(&gphase,dt);
    let f_slice = cumptrapz(&gslice,dt);

    let mut fsq = Vec::<f32>::with_capacity(f_read.len());
    for i in 0..f_read.len() {
        let v = (f_read[i],f_phase[i],f_slice[i]);
        fsq.push(dot(v,v));
    }

    let term1 = trapz(&fsq,dt);

    let idx = f_read.len()/2;
    let ft_read = f_read[idx];
    let ft_phase = f_phase[idx];
    let ft_slice = f_slice[idx];

    let f_read_h = f_read[idx..n].to_vec();
    let f_phase_h = f_phase[idx..n].to_vec();
    let f_slice_h = f_phase[idx..n].to_vec();

    let v1 = (trapz(&f_read_h,dt),trapz(&f_phase_h,dt),trapz(&f_slice_h,dt));
    let v2 = (4.0*ft_read,4.0*ft_phase,4.0*ft_slice);
    let term2 = dot(v1,v2);

    let term3 = 4.0*dot((ft_read,ft_phase,ft_slice),(ft_read,ft_phase,ft_slice))*(echo_time/2.0);

    (grad_cal::GAMMA*2.0*PI).powi(2)*(term1 - term2 + term3)*1E-6

}


fn resample(t:&Vec<f32>,x:&Vec<f32>,tq:&Vec<f32>) -> Vec<f32> {
    let xq:Vec<f32> = tq.iter().map(|t_val| interp1(&t,&x,*t_val)).collect();
    xq
}


fn interp1(x: &Vec<f32>,y: &Vec<f32>,qy:f32) -> f32 {
    let n = x.len()-1;
    let i = look(&x,qy);
    if i.is_none() {
        return y[0]
    }
    let i = i.unwrap();
    if i >= n {
        return y[n]
    }
    lerp((x[i],x[i+1]),(y[i],y[i+1]),qy)
}


fn lerp(x:(f32,f32),y:(f32,f32),qx:f32) -> f32 {
    // parameterize space between x.0,x.1
    let p = (qx - x.0)/(x.1 - x.0);
    (1.0-p)*y.0 + p*y.1
}

pub fn lookup(a: &Vec<f32>, target_value: f32) -> Option<usize> {
    let mut low:usize = 0;
    let mut high = a.len() - 1;

    while low <= high {
        let mid = ((high - low) / 2) + low;
        let mid_index = mid;
        let val = a[mid_index].clone();
        if val == target_value {
            return Some(mid_index);
        }
        // Search values that are greater than val - to right of current mid_index
        if val < target_value {
            low = mid + 1;
        }
        // Search values that are less than val - to the left of current mid_index
        if val > target_value {
            high = mid - 1;
        }
    }
    None
}

fn dot(a:(f32,f32,f32),b:(f32,f32,f32)) -> f32 {
    a.0*b.0 + a.1*b.1 + a.2*b.2
}

fn cumptrapz(a: &Vec<f32>,dt:f32) -> Vec<f32> {
    let mut atmp = Vec::<f32>::new();
    for i in 0..a.len()-1 {
        atmp.push(dt*(a[i] + a[i+1])/2.0)
    }
    let mut b = cumsum(&atmp);
    b.insert(0,0.0);
    b
}

fn trapz(a :&Vec<f32>,dt:f32) -> f32 {
    let mut sum = 0.0;
    for i in 0..a.len()-1 {
        sum += dt*(a[i] + a[i+1])/2.0;
    }
    sum
}

fn cumsum(a: &Vec<f32>) -> Vec<f32> {
    let mut sum = 0.0;
    let mut o = Vec::<f32>::with_capacity(a.len());
    for i in 0..a.len() {
        sum = sum + a[i];
        o.push(sum);
    }
    o
}


// if this becomes a problem, we should implement a binary search here instead.
// This will take quadratic time with a and array of target values O(len(a)*len(target_val))
fn look(a: &Vec<f32>, target_value: f32) -> Option<usize> {
    // a is assumed to be sorted
    let min = a[0];
    let max = a[a.len()-1];
    if target_value < min {
        return None
    }
    if target_value >= max {
        return Some(a.len()-1)
    }
    for i in 0..a.len()-1 {
        if (a[i] == target_value) || (target_value > a[i] && target_value < a[i+1]){
            return Some(i as usize)
        }
    }
    None
}