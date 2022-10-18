use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;
use clap::Parser;

const DIR:&str = "C:/workstation/civm_scan/vb_script";
const STATUS_VBS:&str = "status.vbs";
const SET_PPR_VBS:&str = "set_ppr.vbs";
const SETUP_VBS:&str = "setup.vbs";
const ABORT_VBS:&str = "abort.vbs";
const RUN_VBS:&str = "run.vbs";
const UPLOAD_VBS:&str = "load_table.vbs";
const SET_MRD_VBS:&str = "set_mrd.vbs";

/* To see all available methods for a new vbscript, run the following in powershell
    New-Object -ComObject Scan.Application | Get-Member
*/


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    sub_command:String,
    vargs:Vec<String>
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ArgsSetPPR {
    parent_command:String,
    ppr_file:String
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ArgsSetMRD {
    parent_command:String,
    mrd_file:String
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ArgsUpload {
    parent_command:String,
    table_file:String
}

fn main() {

    let args = Args::parse();
    match args.sub_command.as_str() {
        "status" => {
            let stat = scan_status();
            println!("{:?}",stat);
            //todo!(make enum of all status' to report what's happening in english)
        }
        "set_ppr" => {
            let args = ArgsSetPPR::parse();
            let ppr_file = Path::new(&args.ppr_file);
            let stat = set_ppr(&ppr_file);
            match stat {
                false => println!("failed to set ppr"),
                true => {}
            }
        }
        "set_mrd" => {
            let args = ArgsSetMRD::parse();
            let mrd_file = Path::new(&args.mrd_file);
            let stat = set_mrd(&mrd_file);
            match stat {
                false => println!("failed to set mrd"),
                true => {}
            }
        }
        "run_setup" => {
            run_setup()
        }
        "run" => {
            run_acquisition()
        }
        "abort" => {
            abort()
        }
        "upload" => {
            let args:ArgsUpload = ArgsUpload::parse();
            let table_file = Path::new(&args.table_file);
            upload_table(table_file);
        }
        _=> println!("command not recognized")
    }
}

//196095
fn upload_table(path_to_table:&Path){
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

fn set_ppr(path:&Path) -> bool {
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

fn set_mrd(path:&Path) -> bool {
    let script = Path::new(DIR).join(SET_MRD_VBS);
    let mut cmd = Command::new("cscript");
    let out = cmd.args(vec![
        script,
        path.to_owned()
    ]).output().expect("failed to launch cscript");
    true
}

fn run_setup() {
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

fn run_acquisition() {
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


fn abort() {
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
            0 => Idle,
            _=> Unknown
        }
    }
}

fn scan_status() -> Status {
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