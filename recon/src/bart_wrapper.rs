use serde::{Deserialize, Serialize};
use std::io::{Write, Read};
use std::path::{Path,PathBuf};
use std::fs::File;
use std::process::{Command, CommandArgs};
use toml;
use crate::utils::{self, vec_to_string};
use crate::cfl;
//use crate::mrd::{mrd_to_cfl};

#[derive(Debug,Deserialize,Serialize)]
pub struct BartPicsSettings{
    pub bart_binary:String,
    pub max_iter:u32,
    pub algorithm:String,
    pub respect_scaling:bool,
    pub regularization:f32,
    pub debug:bool,
    pub coil_sensitivity:String,
    pub image_scale_histo_percent:f64
}

impl BartPicsSettings{
    pub fn default() -> BartPicsSettings{
        return BartPicsSettings{
            bart_binary:"bart".to_string(), // assume that it is on the path
            max_iter:36,
            algorithm:"l1".to_string(),
            respect_scaling:true,
            regularization:0.005,
            debug:true,
            coil_sensitivity:"".to_string(),
            image_scale_histo_percent:0.9995
        }
    }

    pub fn quick() -> BartPicsSettings{
        let mut s = BartPicsSettings::default();
        s.max_iter = 2;
        return s
    }

    pub fn to_file(&self,dest_path:&str){
        let s = toml::to_string(&self).expect("trouble serializing data struct");
        utils::write_to_file(dest_path,"toml",&s);
    }

    pub fn write_default(dest_path:&str){
        let d = BartPicsSettings::default();
        d.to_file(dest_path);
    }

    pub fn from_file(src_path:&str) -> BartPicsSettings{
        let s = utils::read_to_string(src_path,"toml").expect("cannot open file");
        return toml::from_str(&s).expect("cannot deserialize file");
    }

    pub fn cmd_stub(&self) -> String {
        let reg = format!("-{}",self.regularization);
        let algo = format!("-{}",self.algorithm);
        let scale = if self.respect_scaling { "-S" } else { "" };
        let debug = if self.debug {"-d5"} else {""};
        let iter = format!("-i{}",self.max_iter);
        return format!("{} pics {} {} {} {} {}",self.bart_binary,algo,reg,iter,scale,debug);
    }

    pub fn cmd_full(&self,cfl_in:&str,cfl_out:&str){
        let mut pin = Path::new(cfl_in).to_owned().with_extension("");
        let mut pout = Path::new(cfl_out).to_owned().with_extension("");
        // create kspace sensitivity data
        let mut ksens = pin.clone();
        let mut fname = ksens.file_name().expect("cannot extract file name from path").to_str().unwrap().to_string();
        fname.push_str("_sens"); // in the future we will likely only need one of these, not one for every volume
        ksens = ksens.with_file_name(fname);
    }

    pub fn set_unit_coil_sens(&mut self,sens_cfl:&str,dims:Vec<usize>){
        self.coil_sensitivity = sens_cfl.to_string();
        println!("writing unit sens");
        let mut cmd = Command::new(&self.bart_binary);
        cmd.arg("ones")
        .arg(dims.len().to_string());
        for d in dims{
            cmd.arg(d.to_string());
        }
        cmd.arg(sens_cfl);
        println!("{:?}",cmd);
        let proc = cmd.spawn().expect("failed to start bart ones");
        let result = proc.wait_with_output().expect("failed to wait for output");
        if !result.status.success(){
            println!("command failed!");
        }
    }

    pub fn set_bart_binary(&mut self,binary_path:&str){
        self.bart_binary = binary_path.to_string();
        let p = Path::new(binary_path);
        //if !p.exists(){panic!("bart binary not found at {}",binary_path)}
    }

    pub fn set_unit_sens_from_cfl(&mut self,cfl:&str){
        let cfl_path = Path::new(cfl);
        let p_in = Path::new(cfl).with_extension("").to_owned();
        let mut coil_sens = p_in.clone();
        let mut name = coil_sens.file_name().unwrap().to_str().unwrap().to_string();
        name.push_str("_sens");
        coil_sens = coil_sens.with_file_name(name);
        let dims = cfl::get_dims(&cfl_path);
        self.set_unit_coil_sens(coil_sens.to_str().unwrap(),dims);
    }
}

pub fn bart_pics(kspace_cfl:&str,img_cfl:&str,settings:&mut BartPicsSettings){

    let kspace_cfl_path = Path::new(&kspace_cfl);
    
    let sens_path = Path::new(&settings.coil_sensitivity);
    // reference coil sensitivity data if exist and is correct, make fresh if otherwise
    if settings.coil_sensitivity.is_empty(){
        settings.set_unit_sens_from_cfl(kspace_cfl);
    }else {
        let sens_dims = cfl::get_dims(sens_path);
        let kspace_dims = cfl::get_dims(kspace_cfl_path);
        if sens_dims != kspace_dims{
            settings.set_unit_sens_from_cfl(kspace_cfl);
        }
    }
    //settings.to_file(bart_pics_settings);
    let mut cmd = Command::new(&settings.bart_binary);
    let scale = if settings.respect_scaling { "-S" } else { "" };
    let debug = if settings.debug {"-d5"} else {""};
    cmd.arg("pics");
    cmd.arg(format!("-{}",settings.algorithm));
    cmd.arg(format!("-r{}",settings.regularization));
    cmd.arg(format!("-i{}",settings.max_iter));
    cmd.arg(scale);
    cmd.arg(debug);
    cmd.arg(kspace_cfl).arg(&settings.coil_sensitivity).arg(img_cfl);

    // cmd.arg("normalize");
    // cmd.arg(kspace_cfl).arg(&settings.coil_sensitivity).arg(img_cfl);

    println!("{:?}",cmd);
    let proc = cmd.spawn().expect("failed to launch bart pics");
    let results = proc.wait_with_output().expect("failed to wait on output");
    if !results.status.success(){panic!("bart pics failed!");}    
}

// #[test]
// fn test(){
//     // configure BartPicsSettings
//     let p = "./def_recon";
//     let mut s = BartPicsSettings::quick();
//     s.to_file(p);
//     let mut r_settings = BartPicsSettings::from_file(p);
//     r_settings.set_bart_binary("/home/wyatt/bart-0.7.00/bart");
//     r_settings.to_file(p);
//
//     mrd_to_cfl("/home/wyatt/testdata/220521T03_m00.mrd",
//     "0", "/home/wyatt/petableCS_stream/stream_CS480_8x_pa18_pb54",
//     "480", "./test_cfl");
//
//     bart_pics("./test_cfl","./img_cfl",&mut s);
//
// }