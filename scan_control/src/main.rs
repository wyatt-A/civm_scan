use std::path::PathBuf;
use std::process::Command;

const STATUS_VBS:&str = "status.vbs";
const DIR:PathBuf = PathBuf::new("D:\\dev\\scan_control");

fn main() {
    println!("Hello, world!");
}

pub enum ScanCommand {
    Maximize,
    SetPPR(PathBuf),
    SetMrd(PathBuf),
    Status,
}

fn scan_status() -> (i32,String) {
    let args = vec![
        "status.vbs"
    ];
    let mut cmd = Command::new("cscript");
    cmd.args(args);
}