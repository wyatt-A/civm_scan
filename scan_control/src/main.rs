use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{thread,time};
use std::time::SystemTime;
use regex::Regex;
use clap::{Parser,Arg,Subcommand};
use glob::glob;
use chrono::{DateTime,Local};

const DIR:&str = "C:/workstation/civm_scan/vb_script";
const STATUS_VBS:&str = "status.vbs";
const SET_PPR_VBS:&str = "set_ppr.vbs";
const SETUP_VBS:&str = "setup.vbs";
const ABORT_VBS:&str = "abort.vbs";
const RUN_VBS:&str = "run.vbs";
const UPLOAD_VBS:&str = "load_table.vbs";
const SET_MRD_VBS:&str = "set_mrd.vbs";



#[derive(clap::Parser,Debug)]
struct ScanControlArgs {
    #[command(subcommand)]
    action: Action,
}

#[derive(clap::Subcommand,Debug)]
pub enum Action {
    /// upload a lookup table for compressed sensing
    UploadTable(PathArgs),
    /// set the ppr for the scan
    SetPPR(PathArgs),
    /// set the output mrd for data collection
    SetMRD(PathArgs),
    /// get the status of the scan supervisor
    Status,
    /// run in continuous setup mode with no data collection
    RunSetup,
    /// run the current ppr to generate a mrd file
    RunScan,
    /// finds all pprs nested in the parent directory and runs them
    RunDirectory(RunDirectoryArgs),
    /// abort the scan
    Abort,
    /// Run a ppr in setup mode
    SetupPPR(RunDirectoryArgs),
    /// Acquire data for PPR
    AcquirePPR(RunDirectoryArgs),
}

#[derive(clap::Args,Debug)]
pub struct PathArgs {
    path:PathBuf,
}

#[derive(clap::Args,Debug)]
pub struct RunDirectoryArgs {
    path:PathBuf,
    #[clap(short, long)]
    cs_table:Option<String>,
    #[clap(short, long)]
    depth_to_search:Option<u8>
}


//
// #[derive(Debug,Subcommand)]
// pub enum SubCommand {
//     SetPPR(SetPPRArgs),
//     SetMRD(SetMrdArgs)
//
// }
//
// #[derive(Debug,Arg)]
// pub struct SetPPRArgs {
//     #[clap(subcommand)]
//     ppr_path:String
// }
//
// #[derive(Debug,Arg)]
// pub struct SetMrdArgs {
//     #[clap(subcommand)]
//     mrd_path:String
// }
//
// #[derive(Debug,Arg)]
// pub struct TableUploadArgs {
//     #[clap(subcommand)]
//     table_path:String
// }



#[test]
fn test(){
    let base_dir = Path::new(r"D:\dev\221101\acq");

    let depth = 2;

    let pattern = (0..depth).map(|_| r"*\").collect::<String>();
    let pattern = format!("{}*.ppr",pattern);

    let pat = base_dir.join(pattern);
    let paths:Vec<PathBuf> = glob(pat.to_str().unwrap()).expect("failed to read glob pattern").flat_map(|m| m).collect();

    let pairs:Vec<(PathBuf,PathBuf)> = paths.iter().map(|ppr| (ppr.clone(),ppr.with_extension("mrd"))).collect();

    pairs.iter().for_each(|pair| {
        set_ppr(&pair.0);
        set_mrd(&pair.1);
    });
}


fn main(){
    let args = ScanControlArgs::parse();

    match args.action {
        Action::UploadTable(path_str) => {
            upload_table(Path::new(&path_str.path))
        }
        Action::SetPPR(path_str) => {
            println!("Setting ppr ...");
            set_ppr(Path::new(&path_str.path));
        }
        Action::SetMRD(path_str) => {
            println!("Setting mrd ...");
            set_mrd(Path::new(&path_str.path));
        }
        Action::Status => {
            let stat = scan_status();
            println!("scan_status: {:?}",stat);
        }
        Action::RunSetup => {
            run_setup()
        }
        Action::RunScan => {
            run_acquisition()
        }
        Action::Abort => {
            abort()
        }
        Action::RunDirectory(args) => {
            run_directory(args)
        }
        Action::SetupPPR(args) => {
            setup_ppr(args);
        }
        Action::AcquirePPR(args) => {
            acquire_ppr(args)
        }
    }
}


fn setup_ppr(args:RunDirectoryArgs) {
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



fn acquire_ppr(args:RunDirectoryArgs) {
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


fn run_directory(args:RunDirectoryArgs){
    let base_dir = Path::new(&args.path);

    let depth = args.depth_to_search.unwrap_or(1);

    let pattern = (0..depth).map(|_| r"*\").collect::<String>();
    let pattern = format!("{}*.ppr",pattern);

    let pat = base_dir.join(pattern);
    let paths:Vec<PathBuf> = glob(pat.to_str().unwrap()).expect("failed to read glob pattern").flat_map(|m| m).collect();
    let pairs:Vec<(PathBuf,PathBuf)> = paths.iter().map(|ppr| (ppr.clone(),ppr.with_extension("mrd"))).collect();

    // check to make sure we are not already running something before we start
    match scan_status() {
        Status::AcquisitionInProgress | Status::SetupInProgress | Status::Running => {
            println!("cannot launch new scan jobs while scan is currently running. Use abort to kill the current scan and try again");
            return
        }
        _=> println!("attempting to run {} ppr(s)",pairs.len())
    }

    // loop thru all pprs and run them in acq mode
    pairs.iter().enumerate().for_each(|(index,pair)| {
        match &args.cs_table {
            Some(table_pat) => {
                let pat = pair.0.with_file_name(format!("*{}*",table_pat));
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
        println!("running acquisition {} of {} ...",index+1,pairs.len());
        set_ppr(&pair.0);
        set_mrd(&pair.1);
        run_acquisition();
        thread::sleep(time::Duration::from_secs(2));
        loop { // run_acquisition() doesn't block so we have to do it manually with a loop
            match scan_status() {
                // block until scanning is done
                Status::AcquisitionInProgress | Status::SetupInProgress | Status::Running => {
                    thread::sleep(time::Duration::from_secs(2));
                }
                // when acq is complete, write to an ac (acq complete) file and record the date and time
                Status::AcquisitionComplete => {
                    let datetime: DateTime<Local> = SystemTime::now().into();
                    let s = format!("completion_date={}", datetime.format("%Y%m%d:%T"));
                    let mut f = File::create(pair.0.with_extension("ac")).expect("unable to create file");
                    f.write_all(s.as_bytes()).expect("cannot write to file");
                    break
                }
                _=> break
            }
        }
    });
    println!("acquisition complete");
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