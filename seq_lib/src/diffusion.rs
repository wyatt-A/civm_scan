use std::f32::consts::PI;
use std::fs::{create_dir_all, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use seq_tools::grad_cal::{GAMMA, tesla_per_mm_to_dac};
use crate::compressed_sensing::{CompressedSensing, CSTable};
use crate::pulse_sequence::{Build, PulseSequence};

pub fn generate_experiment<T>(sequence_params:&T, b_table:&Path) -> Vec<T>
    where T: DiffusionWeighted + Clone {
    let mut s = sequence_params.clone();
    let b_val = s.b_value();
    let b_table = read_b_table(b_table);
    b_table.iter().map(|exp| {
        let scale = exp.0;
        let direction = (exp.1,exp.2,exp.3);
        s.set_b_value(b_val*scale);
        s.set_b_vec(direction);
        s.clone()
    }).collect()
}

pub trait DiffusionWeighted {
    fn b_value(&self) -> f32;
    fn set_b_value(&mut self,b_value:f32);
    fn b_vec(&self) -> (f32,f32,f32);
    fn set_b_vec(&mut self,b_vec:(f32,f32,f32));
    fn pulse_shape(&self) -> PulseShape;
    fn pulse_separation(&self) -> f32;
    fn pulse_duration(&self) -> f32;
}

fn read_b_table(b_table:&Path) -> Vec<(f32,f32,f32,f32)>{
    let mut f = File::open(b_table).expect("b_vec table not found");
    let mut file_string = String::new();
    f.read_to_string(&mut file_string).expect("trouble reading from file");
    let mut b_table = Vec::<(f32,f32,f32,f32)>::new();
    file_string.lines().for_each(|line| {
        if !line.starts_with("#") && !line.is_empty() {
            let s = line.split(", ");
            let values:Vec<f32> = s.map(|elem| elem.parse().expect(&format!("unable to parse {}",elem))).collect();
            if values.len() == 4 {
                b_table.push((values[0],values[1],values[2],values[3]));
            }
        }
    });
    b_table
}

// pub fn build_dw_cs_experiment<T>(sequence:&T, b_table:&Path, work_dir:&Path) -> Vec<PathBuf>
// where T:Clone + DiffusionWeighted + CompressedSensing + Build {
//     let mut diff_experiment = generate_experiment(sequence,b_table);
//     let n = diff_experiment.len();
//     let w = ((n-1) as f32).log10().floor() as usize + 1;
//     let formatter = |index:usize| format!("m{:0width$ }",index,width=w);
//     let mut dirs = Vec::<PathBuf>::new();
//     diff_experiment.iter_mut().enumerate().for_each(|(index, item)| {
//         let label = formatter(index);
//         let d = work_dir.join(&label);
//         dirs.push(d.clone());
//         create_dir_all(&d).expect("trouble building directory");
//         item.ppl_export(&d,&label,false,true);
//         item.cs_table().copy_to(&d,"cs_table");
//         //item.to_file(&d);
//     });
//     dirs
// }

pub fn build_cs_experiment<T>(sequence_array:&mut Vec<T>,work_dir:&Path) -> Vec<PathBuf>
where T:CompressedSensing + Build
{
    let n = sequence_array.len();
    let w = ((n-1) as f32).log10().floor() as usize + 1;
    let formatter = |index:usize| format!("m{:0width$ }",index,width=w);
    let mut dirs = Vec::<PathBuf>::new();
    sequence_array.iter_mut().enumerate().for_each(|(index, item)| {
        let label = formatter(index);
        let d = work_dir.join(&label);
        dirs.push(d.clone());
        create_dir_all(&d).expect("trouble building directory");
        item.set_cs_table();
        item.ppl_export(&d,&label,false,true);
        CSTable::open(&item.cs_table()).copy_to(&d,"cs_table");
        //item.to_file(&d);
    });
    dirs
}




pub enum PulseShape {
    HalfSin,
}

// s/mm^2 -> dac
pub fn b_val_to_dac(pulse:PulseShape,b_val:f32,delta:f32,Delta:f32,direction:(f32,f32,f32)) -> (i16,i16,i16) {
    let g = b_val_to_grad(pulse,b_val,delta,Delta);
    let grad_vec = grad_to_grad_vec(g,direction); // T/mm
    (tesla_per_mm_to_dac(grad_vec.0),tesla_per_mm_to_dac(grad_vec.1),tesla_per_mm_to_dac(grad_vec.2))
}

// s/mm^2 -> T/mm
pub fn b_val_to_grad(pulse:PulseShape,b_val:f32,delta:f32,Delta:f32) -> f32 {
    //gp = sqrt(bval*pi^2*delta^(-2)*gamma^(-2)*(4*Delta - delta)^(-1))
    //let gamma:f32 = 267.52218744E6;
    match pulse {
        PulseShape::HalfSin => {
            (b_val*PI.powi(2)*delta.powi(-2)*GAMMA.powi(-2)*(4.0*Delta - delta).powi(-1)).sqrt()
        }
    }
}

pub fn grad_to_grad_vec(gradient_strength:f32,direction:(f32,f32,f32)) -> (f32,f32,f32) {
    let mag = (direction.0.powi(2) + direction.1.powi(2) + direction.2.powi(2)).sqrt();
    let direction_norm = (direction.0/mag, direction.1/mag, direction.2/mag);
    (direction_norm.0*gradient_strength, direction_norm.1*gradient_strength, direction_norm.2*gradient_strength)
}