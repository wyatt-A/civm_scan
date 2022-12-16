use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{thread,time};
use std::time::SystemTime;
use regex::Regex;
use glob::glob;
use chrono::{DateTime,Local};
use utils;

use crate::args::*;

const DIR:&str = "C:/workstation/civm_scan/vb_script";
const STATUS_VBS:&str = "status.vbs";
const SET_PPR_VBS:&str = "set_ppr.vbs";
const SETUP_VBS:&str = "setup.vbs";
const ABORT_VBS:&str = "abort.vbs";
const RUN_VBS:&str = "run.vbs";
const UPLOAD_VBS:&str = "load_table.vbs";
const SET_MRD_VBS:&str = "set_mrd.vbs";

pub fn setup_ppr(args:RunDirectoryArgs) {
    let ppr = args.path.to_owned();

    if !set_ppr(&ppr){
        panic!("ppr not set. Cannot continue.");
    }
    match &args.cs_table {
        Some(table_pat) => {
            let pat = ppr.with_file_name(format!("*{}*",table_pat));
            let paths:Vec<PathBuf> = glob(pat.to_str().unwrap()).expect("failed to read glob pattern").flat_map(|m| m).collect();
            if paths.len() < 1 {
                println!("no cs table found that matches pattern! table will not be uploaded");
            }
            else{
                upload_table(&paths[0]);
                println!("cs table uploaded");
            }
        }
        None => {}
    };
    run_setup();
}



pub fn acquire_ppr(args:RunDirectoryArgs) {
    let ppr = args.path.to_owned();
    let mrd = ppr.with_extension("mrd");
    set_ppr(&ppr);
    match &args.cs_table {
        Some(table_pat) => {
            let pat = ppr.with_file_name(format!("*{}*",table_pat));
            let paths:Vec<PathBuf> = glob(pat.to_str().unwrap()).expect("failed to read glob pattern").flat_map(|m| m).collect();
            if paths.len() < 1 {
                println!("no cs table found that matches pattern! table will not be uploaded");
            }
            else{
                upload_table(&paths[0]);
                println!("cs table uploaded");
            }
        }
        None => {}
    };
    set_mrd(&mrd);
    run_acquisition();
}

pub enum ScanControlError {
    PPRNotFound,
    ScanBusy,
    ScanStoppedUnexpectedly,
}


pub fn run_directory(args:RunDirectoryArgs) -> Result<(),ScanControlError>{

    let ppr_files = utils::find_files(&args.path,"ppr",true).ok_or(ScanControlError::PPRNotFound)?;

    let cs_table_pattern = args.cs_table.unwrap_or(String::from("cs_table"));
    let n_pprs = ppr_files.len();

    // check to make sure we are not already running something before we start
    if scan_busy() {
        println!("scan is currently active. Cannot continue");
        return Err(ScanControlError::ScanBusy);
    }

    // loop thru all pprs and run them in acq mode
    ppr_files.iter().enumerate().for_each(|(index,ppr)| {
        // upload a cs_table if it exists
        match utils::get_first_match(&ppr.parent().unwrap(), &cs_table_pattern) {
            Some(cs_table) => upload_table(&cs_table),
            None => println!("no cs table found that matches {}. No table will be uploaded",cs_table_pattern),
        }
        println!("running ppr {} of {} ...",index+1,n_pprs);
        set_ppr(&ppr);
        set_mrd(&ppr.with_extension("mrd"));
        run_acquisition();
        thread::sleep(time::Duration::from_secs(2));

        let err = loop {

            if !scan_busy() && scan_complete() {
                utils::write_to_file(&ppr,"ac",&format!("completion_date={}", utils::time_stamp()));
                break Ok(());
            }else if !scan_busy() && !scan_complete() {
                break Err(ScanControlError::ScanStoppedUnexpectedly)
            }
            else if scan_busy() {
                thread::sleep(time::Duration::from_secs(2))
            }
            else {
                Err()
            }
        };

    println!("acquisition complete");
    }
err
}

pub fn scan_busy() -> bool {
    match scan_status(){
        Status::AcquisitionInProgress | Status::SetupInProgress | Status::Running => true,
        _=> false
    }
}

pub fn scan_complete() -> bool {
    match scan_status(){
        Status::AcquisitionComplete => true,
        _=> false
    }
}


//196095
pub fn upload_table(path_to_table:&Path){
    let script = Path::new(DIR).join(UPLOAD_VBS);
    let mut cmd = Command::new("cscript");
    if !path_to_table.exists(){
        panic!("cannot find table: {:?}",path_to_table);
    }
    let mut table_string = String::new();
    let mut f = File::open(path_to_table).expect("cannot open table");
    f.read_to_string(&mut table_string).expect("cannot read table");
    let lines = table_string.lines();
    let v:Vec<i32> = lines.flat_map(|line| line.parse()).collect();
    for x in v.iter() {
        if *x > i16::MAX as i32 {
            panic!("detected value larger than max int16: {}",*x);
        }
    }
    let v2:Vec<i16> = v.iter().map(|entry| *entry as i16).collect();
    if v2.len() > 196095 {
        panic!("not enough memory for table");
    }
    let out = cmd.args(vec![
        script,
        path_to_table.to_owned()
    ]).output().expect("failed to launch cscript");
    //let s = String::from_utf8(out.stdout).unwrap();
    //println!("{}",s);
}

pub fn set_ppr(path:&Path) -> bool {
    let script = Path::new(DIR).join(SET_PPR_VBS);
    let mut cmd = Command::new("cscript");
    if !path.exists(){
        println!("cannot find ppr file: {:?}",path);
        return false
    }
    let out = cmd.args(vec![
        script,
        path.to_owned()
    ]).output().expect("failed to launch cscript");
    true
}

pub fn set_mrd(path:&Path) -> bool {
    let script = Path::new(DIR).join(SET_MRD_VBS);
    let mut cmd = Command::new("cscript");
    let out = cmd.args(vec![
        script,
        path.to_owned()
    ]).output().expect("failed to launch cscript");
    true
}

pub fn run_setup() {
    let stat = scan_status();
    match stat {
        Status::AcquisitionInProgress => println!("acquisition is already in progress. You must abort the scan first."),
        Status::SetupInProgress => println!("setup is already in progress. You must abort the scan first."),
        _=> {
            let script = Path::new(DIR).join(SETUP_VBS);
            let mut cmd = Command::new("cscript");
            let out = cmd.args(vec![
                script
            ]).output().expect("failed to launch cscript");
        }
    }
}

pub fn run_acquisition() {
    let stat = scan_status();
    match stat {
        Status::AcquisitionInProgress => println!("acquisition is already in progress. You must abort the scan first."),
        Status::SetupInProgress => println!("setup is in progress. You must abort the current scan first."),
        _=> {
            let script = Path::new(DIR).join(RUN_VBS);
            let mut cmd = Command::new("cscript");
            let out = cmd.args(vec![
                script
            ]).output().expect("failed to launch cscript");
        }
    }
}


pub fn abort() {
    let script = Path::new(DIR).join(ABORT_VBS);
    let mut cmd = Command::new("cscript");
    let out = cmd.args(vec![
        script
    ]).output().expect("failed to launch cscript");
}

#[derive(Debug)]
pub enum Status {
    Running,
    SetupInProgress,
    AcquisitionInProgress,
    AcquisitionComplete,
    Aborted,
    Idle,
    Unknown
}

impl Status {
    pub fn from_id(id:i32) -> Self {
        use Status::*;
        match id {
            5 => Aborted,
            2 => SetupInProgress,
            3 => AcquisitionInProgress,
            4 => AcquisitionComplete,
            0 => Idle,
            _=> Unknown
        }
    }
}

pub fn scan_status() -> Status {
    let script = Path::new(DIR).join(STATUS_VBS);
    let mut cmd = Command::new("cscript");
    let out = cmd.args(vec![
        script
    ]).output().expect("failed to launch cscript");
    let stdout = String::from_utf8(out.stdout).expect("failed to parse bytes");
    let lines = stdout.lines();
    let reg = Regex::new(r"status_id:([0-9])").unwrap();
    let mut status = String::new();
    lines.for_each(|line|{
        //println!("{}",line);
        let caps = reg.captures(line);
        if caps.is_some(){
            let stat:String = caps.unwrap().get(1).map_or("", |m| m.as_str()).to_string();
            if !stat.is_empty(){
                status = stat;
            }
        }
    });
    if status.is_empty(){
        panic!("status not found!");
    }
    let id = status.parse().expect("unable to parse string");
    Status::from_id(id)
}