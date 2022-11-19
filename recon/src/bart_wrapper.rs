use serde::{Deserialize, Serialize};
use std::io::{Write, Read};
use std::path::{Path,PathBuf};
use std::fs::File;
use std::process::{Command, CommandArgs};
use toml;
use utils::{read_to_string, vec_to_string};
use mr_data::cfl;
use crate::recon_config::ReconSettings;
//use crate::mrd::{mrd_to_cfl};

pub fn write_unit_sens(template_cfl_base:&Path,output_base:&Path,settings:&ReconSettings) {
    let dims = cfl::get_dims(template_cfl_base);
    let mut cmd = Command::new(&settings.bart_binary);
    cmd.arg("ones")
        .arg(dims.len().to_string());
    for d in dims{
        cmd.arg(d.to_string());
    }
    cmd.arg(output_base);
    println!("{:?}",cmd);
    let proc = cmd.spawn().expect("failed to start bart ones");
    let result = proc.wait_with_output().expect("failed to wait for output");
    if !result.status.success(){
        println!("command failed!");
    }
}

pub fn bart_pics(kspace_cfl:&Path,img_cfl:&Path,settings:&ReconSettings){

    let name = format!("{}_sens",kspace_cfl.file_name().unwrap().to_str().unwrap());
    let sens = kspace_cfl.with_file_name(name);

    write_unit_sens(kspace_cfl,&sens,settings);

    //settings.to_file(bart_pics_settings);
    let mut cmd = Command::new(&settings.bart_binary);
    let scale = if settings.respect_scaling { "-S" } else { "" };
    let debug = "-d5";
    cmd.arg("pics");
    cmd.arg(format!("-{}",settings.algorithm.print()));
    cmd.arg(format!("-r{}",settings.regularization));
    cmd.arg(format!("-i{}",settings.max_iter));
    cmd.arg(scale);
    cmd.arg(debug);
    cmd.arg(kspace_cfl).arg(&sens).arg(img_cfl);
    println!("{:?}",cmd);
    let proc = cmd.spawn().expect("failed to launch bart pics");
    let results = proc.wait_with_output().expect("failed to wait on output");
    if !results.status.success(){panic!("bart pics failed!");}

    std::fs::remove_file(sens.with_extension("cfl")).expect("cannot clean up sens file!");
    std::fs::remove_file(sens.with_extension("hdr")).expect("cannot clean up sens header!");
}