use std::f32::consts::PI;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use seq_tools::event_block::{EventQueue, EventQueueError};
use seq_tools::seqframe::SeqFrame;
use build_sequence::build_directory::{Config,build_directory};
use seq_tools::ppl::{BaseFrequency, GradClock, Orientation, PhaseUnit, PPL};
use serde_json;
use serde::{Serialize,Deserialize};
use seq_tools::grad_cal::{GAMMA, tesla_per_mm_to_dac};
use dyn_clone::DynClone;
use headfile::headfile::{AcqHeadfile,DWHeadfile};
use seq_tools::pulse::Pulse;
use utils;

#[derive(Serialize,Deserialize)]
pub struct AdjustmentResults {
    pub obs_freq_offset:f32,
    pub rf_dac_seconds:f32,
    pub freq_spectrum:Vec<[f64;2]>,
    pub rf_cal_spin_vs_stim:Vec<[f64;2]>,
}

impl AdjustmentResults {
    pub fn to_file(&self,filename:&Path) {
        let s = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        utils::write_to_file(filename,"json",&s);
    }
    pub fn from_file(file_path:&Path) -> Self {
        let s = utils::read_to_string(file_path,"json");
        serde_json::from_str(&s).expect("unable to parse json")
    }
}

#[derive(Clone,Serialize,Deserialize)]
pub struct PPLBaseParams {
    pub n_averages:u16,
    pub n_repetitions:u32,
    pub rep_time:f32,
    pub base_frequency:BaseFrequency,
    pub orientation:Orientation,
    pub grad_clock:GradClock,
    pub phase_unit:PhaseUnit,
    pub view_acceleration:u16,
    pub waveform_sample_period_us:usize,
}

pub enum DiffusionPulseShape {
    HalfSin,
}

pub trait DiffusionWeighted {
    fn b_value(&self) -> f32;
    fn set_b_value(&mut self,b_value:f32);
    fn b_vec(&self) -> (f32,f32,f32);
    fn set_b_vec(&mut self,b_vec:(f32,f32,f32));
    fn pulse_shape(&self) -> DiffusionPulseShape;
    fn pulse_separation(&self) -> f32;
    fn pulse_duration(&self) -> f32;
}

pub trait CompressedSense{
    fn is_cs(&self) -> bool {
        false
    }
    fn set_cs_table(&mut self){
    }
    fn cs_table(&self) -> Option<PathBuf> {
        None
    }
}

impl PPLBaseParams {
    pub fn to_file(&self,file_path:&Path) {
        let mut f = File::create(file_path).expect(&format!("cannot create file {:?}",file_path));
        let str = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        f.write_all(str.as_bytes()).expect("trouble writing to file");
    }
    pub fn from_file(file_path:&Path) -> Self {
        let mut f = File::open(file_path).expect(&format!("cannot open file {:?}",file_path));
        let mut in_str = String::new();
        f.read_to_string(&mut in_str).expect("trouble reading file");
        serde_json::from_str(&in_str).expect("cannot deserialize struct")
    }
}

#[derive(Debug)]
pub enum SequenceLoadError {
    InvalidFormat
}


pub trait Initialize {
    fn default() -> Self;
    fn load(params_file:&Path) -> Result<Self,SequenceLoadError> where Self: Sized;
    fn write_default(params_file: &Path);
}

pub trait DWSequenceParameters:SequenceParameters + DiffusionWeighted + DynClone + DWHeadfile {}
pub trait SequenceParameters:
CompressedSense+Simulate+AcqDimensions+DynClone+MrdToKspace+Setup+AcqHeadfile+UseAdjustments {
    fn name(&self) -> String;
    fn write(&self,params_file:&Path);
    fn instantiate(&self) -> Box<dyn Build>;
}

pub trait UseAdjustments {
    fn set_adjustment_file(&mut self,adj_file:&Path);

    fn adjustment_file(&self) -> Option<PathBuf>;

    fn obs_freq_offset(&self) -> Option<f32> {
        match self.adjustment_file() {
            Some(file) => {
                let results = AdjustmentResults::from_file(&file);
                Some(results.obs_freq_offset)
            }
            None => None
        }
    }

    fn rf_dac(&self,flip_angle_degrees:f32,pulse:Box<dyn Pulse>) -> Option<i16> {
        let flip_multiplier = flip_angle_degrees/90.0;
        match self.adjustment_file() {
            Some(file) => {
                let results = AdjustmentResults::from_file(&file);
                Some((flip_multiplier*results.rf_dac_seconds/pulse.power_net(1.0)) as i16)
            }
            None => None
        }
    }
}

pub trait AdjustmentParameters {
    fn set_freq_offset(&mut self,offset_hertz:f32);
    fn name(&self) -> String;
    fn write(&self,params_file:&Path);
    fn instantiate(&self) -> Box<dyn Build>;
}

pub trait ScoutConfig:SequenceParameters {
    fn set_orientation(&mut self,orient:&Orientation);
    fn set_fov(&mut self,fov:(f32,f32));
    fn set_samples(&mut self,samples:(u16,u16));
}

pub trait Setup {
    fn set_mode(&mut self);
    fn set_repetitions(&mut self);
    fn configure_setup(&mut self){
        self.set_mode();
        self.set_repetitions();
    }
}

pub trait Simulate {
    fn set_sim_repetitions(&mut self);
    fn configure_simulation(&mut self) {
        self.set_sim_repetitions();
    }
}

#[derive(Debug)]
pub struct AcqDims {
    pub n_read:i32,
    pub n_phase1:i32,
    pub n_phase2:i32,
    pub n_slices:i32,
    pub n_echos:i32,
    pub n_experiments:i32,
}

#[derive(Serialize,Deserialize)]
pub enum MrdFormat {
    FseCSVol, // 3-D accelerated compressed sensing
    StandardCSVol, // 3-D compressed sensing (single or multi-echo)
    StandardVol,// 3-D standard imaging (single or multi-echo)
    StandardSlice // 2-D imaging (single or multi-echo)
}

// this is an attempt to provide info to unify the reconstruction process for any
// raw mrd file. This will likely grow as we need more fields
#[derive(Serialize,Deserialize)]
pub struct MrdToKspaceParams {
    pub mrd_format:MrdFormat,
    pub n_read:usize,
    pub n_phase1:usize,
    pub n_phase2:usize,
    pub n_views:usize,
    pub view_acceleration:usize,
    pub dummy_excitations:usize,
    pub n_objects:usize // for MGRE or any multi-echo data
}

impl MrdToKspaceParams {
    pub fn from_file(file_path:&Path) -> Self{
        let mut f = File::open(file_path).expect("cannot open file");
        let mut textstr = String::new();
        f.read_to_string(&mut textstr).expect("cannot read from file");
        serde_json::from_str(&textstr).expect("cannot deserialize json")
    }
    pub fn to_file(&self,file_path:&Path) {
        let ext = "mtk";
        let full_name = file_path.with_extension(ext);
        let mut f = File::create(full_name).expect("unable to create file");
        let out_str = serde_json::to_string_pretty(&self).expect("cannot serialize");
        f.write_all(out_str.as_bytes()).expect("trouble writing to file");
    }
}

pub trait MrdToKspace {
    fn mrd_to_kspace_params(&self) -> MrdToKspaceParams;
}

pub trait AcqDimensions {
    fn acq_dims(&self) -> AcqDims;
}


pub trait Build {
    fn place_events(&self) -> EventQueue;
    fn base_params(&self) -> PPLBaseParams;
    fn ppl(&self,ppl_file_path:&Path,sim_mode:bool,build:bool) -> Result<PPL,EventQueueError> {
        let seq_path_strs = self.seq_path_strs(ppl_file_path,build);
        let base_params = self.base_params();
        match PPL::new(
            &mut self.place_events(),
            base_params.n_repetitions,
            base_params.n_averages,
            base_params.rep_time,
            base_params.base_frequency.clone(),
            &seq_path_strs.0,
            &seq_path_strs.1,
            base_params.orientation.clone(),
            base_params.grad_clock.clone(),
            base_params.phase_unit.clone(),
            base_params.view_acceleration,
            sim_mode
        ) {
            Err(e) => Err(e),
            Ok(ppl) => Ok(ppl)
        }
    }

    fn is_valid(&self) -> (bool,Option<String>) {
        match self.ppl(Path::new("dummy_path"),false,false) {
            Ok(ppl) => (true,None),
            Err(e) => {
                (false,Some(format!("event queue error! {:?}",e)))
            }
        }
    }
    fn seq_file_export(&self,sample_period_us:usize,filepath:&str) {
        let q = self.place_events();
        let (grad_params,rf_params) = q.ppl_seq_params(sample_period_us);
        let path = Path::new(filepath);
        let config = Config::load();
        let grad_param_path = path.join(config.grad_param_filename());
        let rf_param_path = path.join(config.rf_param_filename());
        let mut rf_seq_file = File::create(rf_param_path).expect("cannot create file");
        rf_seq_file.write_all(&SeqFrame::format_as_bytes(&rf_params.unwrap())).expect("trouble writing to file");
        match grad_params {
            Some(params) => {
                let mut grad_seq_file = File::create(grad_param_path).expect("cannot create file");
                grad_seq_file.write_all(&SeqFrame::format_as_bytes(&params)).expect("trouble writing to file");
            }
            None => {
                // we still need to create the grad seq file,despite it being empty
                File::create(grad_param_path).expect("cannot create file");
            }
        }
    }


    fn seq_path_strs(&self,ppl_filepath:&Path,build:bool) -> (String,String){
        let base_params = self.base_params();
        let seq_path_strs = match build {
            true => {
                let config = Config::load();
                let grad_seq_path = Path::new(ppl_filepath).join(config.grad_seq()).to_owned();
                let grad_path_str = grad_seq_path.into_os_string().to_str().unwrap().to_owned();
                let rf_seq_path = Path::new(ppl_filepath).join(config.rf_seq()).to_owned();
                let rf_path_str = rf_seq_path.into_os_string().to_str().unwrap().to_owned();
                self.seq_file_export(base_params.waveform_sample_period_us, ppl_filepath.as_os_str().to_str().unwrap());
                (
                    grad_path_str,
                    rf_path_str
                )
            }
            false => {
                (String::from(""),String::from(""))
            }
        };
        seq_path_strs
    }

    fn ppl_export(&mut self,filepath:&Path,ppr_name:&str,sim_mode:bool,build:bool) -> Result<(),EventQueueError> {
        let name = Path::new(ppr_name).with_extension("ppl");
        //let base_params = self.base_params();
        //let seq_path_strs = self.seq_path_strs(filepath);


        let ppl = self.ppl(filepath,sim_mode,build)?;

        // let ppl = PPL::new(
        //     &mut self.place_events(),
        //     base_params.n_repetitions,
        //     base_params.n_averages,
        //     base_params.rep_time,
        //     base_params.base_frequency.clone(),
        //     &seq_path_strs.0,
        //     &seq_path_strs.1,
        //     base_params.orientation.clone(),
        //     base_params.grad_clock.clone(),
        //     base_params.phase_unit.clone(),
        //     base_params.view_acceleration,
        //     sim_mode
        // )?;
        let filename = filepath.join(name);
        let ppr_filename = filepath.join(ppr_name).with_extension("ppr");
        let ppr_str = ppl.print_ppr(&filename);
        let mut out_ppr = File::create(&ppr_filename).expect("cannot create file");
        out_ppr.write_all(ppr_str.as_bytes()).expect("cannot write to file");
        let mut outfile = File::create(&filename).expect("cannot create file");
        outfile.write_all(ppl.print().as_bytes()).expect("cannot write to file");
        if build {
            build_directory(filepath);
        }
        Ok(())
    }
    fn param_export(&self,filepath:&Path);
}

// s/mm^2 -> dac
pub fn b_val_to_dac(pulse: DiffusionPulseShape, b_val:f32, delta:f32, Delta:f32, direction:(f32, f32, f32)) -> (i16, i16, i16) {
    let g = b_val_to_grad(pulse,b_val,delta,Delta);
    let grad_vec = grad_to_grad_vec(g,direction); // T/mm
    (tesla_per_mm_to_dac(grad_vec.0),tesla_per_mm_to_dac(grad_vec.1),tesla_per_mm_to_dac(grad_vec.2))
}

// s/mm^2 -> T/mm
pub fn b_val_to_grad(pulse: DiffusionPulseShape, b_val:f32, delta:f32, Delta:f32) -> f32 {
    //gp = sqrt(bval*pi^2*delta^(-2)*gamma^(-2)*(4*Delta - delta)^(-1))
    //let gamma:f32 = 267.52218744E6;
    match pulse {
        DiffusionPulseShape::HalfSin => {
            (b_val*PI.powi(2)*delta.powi(-2)*GAMMA.powi(-2)*(4.0*Delta - delta).powi(-1)).sqrt()
        }
    }
}

pub fn grad_to_grad_vec(gradient_strength:f32,direction:(f32,f32,f32)) -> (f32,f32,f32) {
    let mag = (direction.0.powi(2) + direction.1.powi(2) + direction.2.powi(2)).sqrt();
    let direction_norm = (direction.0/mag, direction.1/mag, direction.2/mag);
    (direction_norm.0*gradient_strength, direction_norm.1*gradient_strength, direction_norm.2*gradient_strength)
}