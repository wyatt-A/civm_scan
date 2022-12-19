use std::path::{Path, PathBuf};
use std::process::Command;
use std::{thread,time};
use regex::Regex;
use utils;
use cs_table::cs_table::CSTable;
use cs_table::cs_table::MAX_TABLE_ELEMENTS;
use crate::args::*;



const VBS_DIR:&str = "C:/workstation/civm_scan/vb_script";

enum VBScript {
    Status,
    SetPPr,
    SetMrd,
    Setup,
    Abort,
    Run,
    UploadTable,
}

impl VBScript {
    fn file_name(&self) -> &str {
        use VBScript::*;
        match &self {
            Status => "status.vbs",
            SetPPr => "set_ppr.vbs",
            Setup => "setup.vbs",
            Abort => "abort.vbs",
            Run => "run.vbs",
            UploadTable => "load_table.vbs",
            SetMrd => "set_mrd.vbs",
        }
    }
    pub fn path(&self) -> PathBuf {
        Path::new(VBS_DIR).join(self.file_name())
    }
    pub fn run(&self,arg:Option<&Path>) -> Result<String,ScanControlError> {
        let mut cmd = Command::new("cscript");
        cmd.arg(self.path());
        match arg {
            Some(argument) => {
                cmd.arg(argument);
            }
            None => {}
        }
        let out = cmd.output().or(Err(ScanControlError::CScriptError))?;
        Ok(String::from_utf8(out.stdout).unwrap())
    }
}


//let script = Path::new(DIR).join(SET_MRD_VBS);
//     let mut cmd = Command::new("cscript");
//     let out = cmd.args(vec![
//         script,
//         path.to_owned()
//     ]).output().expect("failed to launch cscript");




// pub fn setup_ppr(args:RunDirectoryArgs) {
//     let ppr = args.path.to_owned();
//
//     if !set_ppr(&ppr){
//         panic!("ppr not set. Cannot continue.");
//     }
//     match &args.cs_table {
//         Some(table_pat) => {
//             let pat = ppr.with_file_name(format!("*{}*",table_pat));
//             let paths:Vec<PathBuf> = glob(pat.to_str().unwrap()).expect("failed to read glob pattern").flat_map(|m| m).collect();
//             if paths.len() < 1 {
//                 println!("no cs table found that matches pattern! table will not be uploaded");
//             }
//             else{
//                 upload_table(&paths[0]);
//                 println!("cs table uploaded");
//             }
//         }
//         None => {}
//     };
//     run_setup();
// }



// pub fn acquire_ppr(args:RunDirectoryArgs) -> Result<(),ScanControlError> {
//     let ppr = args.path.to_owned();
//     if !ppr.exists(){
//         return Err(ScanControlError::PPRNotFound);
//     }
//     let mrd = ppr.with_extension("mrd");
//     set_ppr(&ppr);
//     set_mrd(&mrd);
//     let cs_table_pattern = args.cs_table.unwrap_or(String::from("cs_table"));
//     match utils::get_first_match(&ppr.parent().unwrap(), &cs_table_pattern) {
//         Some(cs_table) => upload_table(&cs_table),
//         None => println!("no cs table found that matches {}. No table will be uploaded",cs_table_pattern),
//     }
//     run_acquisition()?;
//     Ok(())
// }


pub fn acquire_ppr(args:RunDirectoryArgs) -> Result<(),ScanControlError> {
    // check to make sure we are not already running something before we start
    if scan_busy()? {
        return Err(ScanControlError::ScanBusy);
    }
    let cs_table_pattern = args.cs_table.unwrap_or(String::from("cs_table"));
    let parent_dir = match args.path.is_dir(){
        true => &args.path,
        false => args.path.parent().ok_or(ScanControlError::InvalidPath)?
    };
    // upload a cs_table if it exists
    match utils::get_first_match(parent_dir, &cs_table_pattern) {
        Some(cs_table) => upload_table(&cs_table)?,
        None => println!("no cs table found that matches {}. No table will be uploaded",cs_table_pattern),
    }
    let ppr = utils::get_first_match(parent_dir,"*.ppr").ok_or(ScanControlError::PPRNotFound)?;
    let mrd = ppr.with_extension("mrd");
    set_ppr(&ppr)?;
    set_mrd(&mrd)?;
    VBScript::Run.run(None)?;
    Ok(())
}



#[derive(Debug)]
pub enum ScanControlError {
    PPRNotFound,
    ScanBusy,
    ScanStoppedUnexpectedly,
    CScriptError,
    StatusNotFound,
    UnknownStatus,
    InvalidPath,
    CSTableNotFound,
    CSTableTooLarge,
}


pub fn run_directory(args:RunDirectoryArgs) -> Result<(),ScanControlError>{

    // find all pprs recursively from base path
    let ppr_files = utils::find_files(&args.path,"ppr",true).ok_or(ScanControlError::PPRNotFound)?;
    let n_pprs = ppr_files.len();

    // this is the pattern used to search for a cs table in the same dir as the ppr
    let cs_table_pattern = args.cs_table.unwrap_or(String::from("cs_table"));

    // check to make sure we are not already running something before we start
    if scan_busy()? {
        println!("scan is currently active. Cannot continue");
        return Err(ScanControlError::ScanBusy);
    }

    for file_idx in 0..n_pprs {
        let ppr = &ppr_files[file_idx];

        // upload a cs_table if it exists
        match utils::get_first_match(&ppr.parent().unwrap(), &cs_table_pattern) {
            Some(cs_table) => upload_table(&cs_table)?,
            None => println!("no cs table found that matches {}. No table will be uploaded",cs_table_pattern),
        }

        // launch the ppr in acquisition mode
        println!("running ppr {} of {} ...",file_idx+1,n_pprs);
        set_ppr(&ppr)?;
        set_mrd(&ppr.with_extension("mrd"))?;
        VBScript::Run.run(None)?;
        thread::sleep(time::Duration::from_secs(2));

        loop {
            if scan_complete()? {
                // scan is complete so we write an ac file and break
                utils::write_to_file(&ppr,"ac",&format!("completion_date={}", utils::time_stamp()));
                break
            }else if !scan_busy()? {
                // if the scan isn't complete and not busy, something unexpected happened, so we will return an error
                return Err(ScanControlError::ScanStoppedUnexpectedly);
            }
            else {
                // here the scanner must be busy so we'll wait 2 seconds and check again
                thread::sleep(time::Duration::from_secs(2))
            }
        };
    }

    // if we get here, the scan job is done and everything is presumed okay
    println!("acquisition complete");
    Ok(())
}

pub fn scan_busy() -> Result<bool,ScanControlError> {
    match scan_status()?{
        Status::AcquisitionInProgress | Status::SetupInProgress | Status::Running => Ok(true),
        _=> Ok(false)
    }
}

pub fn scan_complete() -> Result<bool,ScanControlError> {
    match scan_status()?{
        Status::AcquisitionComplete => Ok(true),
        _=> Ok(false)
    }
}

pub fn upload_table(path_to_table:&Path) -> Result<(),ScanControlError>{
    if !path_to_table.exists(){
        return Err(ScanControlError::CSTableNotFound)
    }
    let table = CSTable::open(path_to_table);
    if table.n_elements() > MAX_TABLE_ELEMENTS {
        return Err(ScanControlError::CSTableTooLarge)
    }
    VBScript::UploadTable.run(Some(path_to_table))?;
    Ok(())
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
    pub fn from_id(id:i32) -> Result<Self,ScanControlError> {
        use Status::*;
        match id {
            5 => Ok(Aborted),
            2 => Ok(SetupInProgress),
            3 => Ok(AcquisitionInProgress),
            4 => Ok(AcquisitionComplete),
            0 => Ok(Idle),
            _=> Err(ScanControlError::UnknownStatus)
        }
    }
}

pub fn scan_status() -> Result<Status,ScanControlError> {
    let stdout = VBScript::Status.run(None)?;
    let reg = Regex::new(r"status_id:([0-9])").expect("invalid regex");
    let caps = reg.captures(&stdout).ok_or(ScanControlError::StatusNotFound)?;
    let stat_string = caps.get(1).map_or("", |m| m.as_str()).to_string();
    let stat_id:i32 = stat_string.parse().or(Err(ScanControlError::StatusNotFound))?;
    Ok(Status::from_id(stat_id)?)
}

pub fn abort() -> Result<(),ScanControlError> {
    VBScript::Abort.run(None)?;
    Ok(())
}

pub fn set_mrd(filepath:&Path) -> Result<(),ScanControlError> {
    // the file path's parent should exist, otherwise scan will not know what to do
    // also, the path needs to be absolute
    let filepath = utils::absolute_path(filepath);
    let mrd_dir = filepath.parent().ok_or(ScanControlError::InvalidPath)?;
    if !mrd_dir.exists() {
        return Err(ScanControlError::InvalidPath);
    }
    let mrd = filepath.with_extension("mrd");
    VBScript::SetMrd.run(Some(&mrd))?;
    Ok(())
}

pub fn set_ppr(filepath:&Path) -> Result<(),ScanControlError> {
    let filepath = utils::absolute_path(filepath).with_extension("ppr");
    if !filepath.exists(){
        return Err(ScanControlError::InvalidPath);
    }
    VBScript::SetPPr.run(Some(&filepath))?;
    Ok(())
}


pub fn setup_ppr(args:RunDirectoryArgs) -> Result<(),ScanControlError> {

    // check to make sure we are not already running something before we start
    if scan_busy()? {
        return Err(ScanControlError::ScanBusy);
    }
    let cs_table_pattern = args.cs_table.unwrap_or(String::from("cs_table"));
    let parent_dir = match args.path.is_dir(){
        true => &args.path,
        false => args.path.parent().ok_or(ScanControlError::InvalidPath)?
    };
    // upload a cs_table if it exists
    match utils::get_first_match(parent_dir, &cs_table_pattern) {
        Some(cs_table) => upload_table(&cs_table)?,
        None => println!("no cs table found that matches {}. No table will be uploaded",cs_table_pattern),
    }

    let ppr = utils::get_first_match(parent_dir,"*.ppr").ok_or(ScanControlError::PPRNotFound)?;

    set_ppr(&ppr)?;
    VBScript::Setup.run(None)?;
    Ok(())
}



// pub fn scan_status() -> Status {
//     let script = Path::new(VBS_DIR).join(STATUS_VBS);
//     let mut cmd = Command::new("cscript");
//     let out = cmd.args(vec![
//         script
//     ]).output().expect("failed to launch cscript");
//     let stdout = String::from_utf8(out.stdout).expect("failed to parse bytes");
//     let lines = stdout.lines();
//     let reg = Regex::new(r"status_id:([0-9])").unwrap();
//     let mut status = String::new();
//     lines.for_each(|line|{
//         //println!("{}",line);
//         let caps = reg.captures(line);
//         if caps.is_some(){
//             let stat:String = caps.unwrap().get(1).map_or("", |m| m.as_str()).to_string();
//             if !stat.is_empty(){
//                 status = stat;
//             }
//         }
//     });
//     if status.is_empty(){
//         panic!("status not found!");
//     }
//     let id = status.parse().expect("unable to parse string");
//     Status::from_id(id)
// }