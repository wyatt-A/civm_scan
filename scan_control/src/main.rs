use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;

const STATUS_VBS:&str = "status.vbs";
const DIR:&str = "C:/workstation/civm_scan/vb_script";

fn main() {
    scan_status();
}

fn scan_status() -> i32 {
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
    status.parse().expect("unable to parse string")
}