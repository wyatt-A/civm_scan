use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use glob::glob;
use glob::GlobResult;
use serde::{Serialize,Deserialize};
use serde_json;

// this should be replaced with an environemnt variable once I know what im doing
const CONFIG_FILE:&str = "c:/workstation/civm_scan/config/build_sequence.json";


#[derive(Serialize,Deserialize)]
struct Config {
    seq_template_dir:PathBuf,
    grad_template:String,
    rf_template:String,
    rf_seq:String,
    grad_seq:String,
    rf_params:String,
    grad_params:String,
}

impl Config {
    pub fn default() -> Self {
        let cfg = Self {
            seq_template_dir:Path::new("c:/workstation/data/seq_file").to_path_buf(),
            grad_template:String::from("civm_grad_template.seq"),
            rf_template:String::from("civm_rf_template.seq"),
            rf_seq:String::from("civm_rf.seq"),
            grad_seq:String::from("civm_grad.seq"),
            rf_params:String::from("civm_rf_params.txt"),
            grad_params:String::from("civm_grad_params.txt"),
        };
        let str:String = serde_json::to_string_pretty(&cfg).expect("cannot serialize config");
        let mut f = File::create(CONFIG_FILE).expect("cannot create file");
        f.write_all(str.as_bytes()).expect("encountered error writing to file");
        cfg
    }
}

pub fn build_directory(wd:&Path) {

    let config = match Path::new(CONFIG_FILE).exists() {
        true => {
            let mut f = File::open(CONFIG_FILE).expect("trouble opening file");
            let mut str = String::new();
            f.read_to_string(&mut str).expect("encountered problem reading file");
            serde_json::from_str(&str).expect("cannot deserialize config. Is it corrupt?")
        },
        false => {
            Config::default()
        }
    };

    let pattern = wd.join(&config.rf_params);
    let entries:Vec<PathBuf> = glob(pattern.to_str().unwrap()).expect("Failed to read glob pattern").flat_map(|m| m).collect();
    match entries.len() < 1 {
        true => println!("no rf param file found!"),
        false => {
            let output = entries[0].with_file_name(&config.rf_seq);
            let rf_template = Path::new(&config.seq_template_dir).join(&config.rf_template);
            let mut cmd = Command::new("seq_gen");
            cmd.args(vec![rf_template,output,entries[0].clone()]);
            //println!("{:?}",cmd);
            let out = cmd.output().expect("failed to launch seq_gen command");
            if !out.status.success() {
                println!("failed to generate rf seq frame");
                let std_err = String::from_utf8(out.stderr).expect("cannot parse standard error");
                println!("{}",std_err);
            }
        }
    }

    let pattern = wd.join(&config.grad_params);
    let entries:Vec<PathBuf> = glob(pattern.to_str().unwrap()).expect("Failed to read glob pattern").flat_map(|m| m).collect();
    match entries.len() < 1 {
        true => println!("no grad param file found!"),
        false => {
            let output = entries[0].with_file_name(&config.grad_seq);
            let rf_template = Path::new(&config.seq_template_dir).join(&config.grad_template);
            let mut cmd = Command::new("seq_gen");
            cmd.args(vec![rf_template,output,entries[0].clone()]);
            //println!("{:?}",cmd);
            let out = cmd.output().expect("failed to launch seq_gen command");
            if !out.status.success() {
                println!("failed to generate gradient seq frame");
                let std_err = String::from_utf8(out.stderr).expect("cannot parse standard error");
                println!("{}",std_err);
            }
        }
    }

    // find all ppl files and compile them
    let pattern = wd.join("*.ppl");
    let entries:Vec<PathBuf> = glob(pattern.to_str().unwrap()).expect("Failed to read glob pattern").flat_map(|m| m).collect();
    entries.iter().for_each(|entry| {
        let out = Command::new("p2f").args(vec!["-b",entry.as_os_str().to_str().unwrap()]).output().expect("failed to launch p2f");
        let std_err = String::from_utf8(out.stderr).expect("cannot parse standard error");
        let std_out = String::from_utf8(out.stdout).expect("cannot parse standard error");
        println!("{}",std_out);
        println!("{}",std_err);
    });
}