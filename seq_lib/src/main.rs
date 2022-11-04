use std::fs::{create_dir_all, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use clap::{Parser, Arg, Subcommand};
use glob::glob;
use seq_lib::fse_dti::{FseDti, FseDtiParams, Protocol};
use seq_lib::pulse_sequence::{Build, Setup};
use regex::Regex;
use seq_lib::diffusion;
use seq_lib::compressed_sensing::{CompressedSensing, CSTable};
use seq_tools::ppl::sync_pprs;


const SEQUENCE_LIB:&str = r"C:\\workstation\\civm_scan\\sequence_library";


#[derive(clap::Parser,Debug)]
struct SeqLibArgs {
    #[command(subcommand)]
    action: Action,
}

#[derive(clap::Subcommand,Debug)]
pub enum Action {
    NewConfig(NewConfigArgs),
    New(NewArgs),
    NewDiffusionExperiment(NewDiffusionExperimentArgs),
    NewSetup(NewArgs),
    ApplySetup(ApplySetupArgs),
    ListSequences
}

#[derive(clap::Args,Debug)]
pub struct NewDiffusionExperimentArgs {
    alias:String,
    destination:PathBuf,
    b_table:PathBuf
}

#[derive(clap::Args,Debug)]
pub struct NewArgs {
    alias:String,
    destination:PathBuf
}

#[derive(clap::Args,Debug)]
pub struct NewConfigArgs {
    name:String,
    alias:String
}

#[derive(clap::Args,Debug)]
pub struct ApplySetupArgs {
    setup_ppr:PathBuf,
    children:PathBuf,
    #[clap(short, long)]
    recursive:Option<u16>
}

// #[derive(clap::Args,Debug,Clone)]
// pub struct ConfiguredSequence {
//     name:String,
//     location:PathBuf,
//     #[clap(short, long)]
//     destination:Option<PathBuf>,
// }


enum Sequence {
    FSE_DTI,
    SE_DTI,
    MGRE,
    GRE,
}

impl Sequence {
    fn list() -> String{
        vec![
            Self::decode(&Self::FSE_DTI),
            Self::decode(&Self::SE_DTI),
            Self::decode(&Self::MGRE),
            Self::decode(&Self::GRE),
        ].join("\n")
    }
    fn encode(name:&str) -> Self {
        match name {
            "fse_dti" => Self::FSE_DTI,
            "se_dti" => Self::SE_DTI,
            "mgre" => Self::MGRE,
            "gre" => Self::GRE,
            _=> panic!("name not recognized")
        }
    }
    fn decode(&self) -> String {
        match &self {
            Self::FSE_DTI => String::from("fse_dti"),
            Self::SE_DTI => String::from("se_dti"),
            Self::MGRE => String::from("mgre"),
            Self::GRE => String::from("gre"),
        }
    }
}


fn main(){
    let args = SeqLibArgs::parse();
    use Action::*;
    match &args.action {

        ListSequences => {
            println!("{}",Sequence::list());
        },
        NewConfig(args) => {
            let seq = Sequence::encode(&args.name);
            let path_out = Path::new(SEQUENCE_LIB).join(&args.alias).with_extension("json");
            if path_out.exists(){
                println!("{} already exists. Choose a different alias.",&args.alias);
                return
            }
            match seq {
                Sequence::FSE_DTI => {
                    FseDtiParams::write_default(&path_out)
                }
                _=> panic!("not yet implemented")
            }
        }
        New(args) => {
            let cfg_file = Path::new(SEQUENCE_LIB).join(&args.alias).with_extension("json");
            match File::open(&cfg_file) {
                Err(_) => println!("alias {} not found!",args.alias),
                Ok(mut f) => {
                    let mut cfg_str = String::new();
                    f.read_to_string(&mut cfg_str).expect("trouble reading file");
                    let seq = find_seq_name_from_config(&cfg_str);
                    match seq {
                        Sequence::FSE_DTI => {
                            let params = FseDtiParams::load(&cfg_file);
                            let mut s = FseDti::new(params);
                            s.ppl_export(&args.destination,"fse_dti",false,true);
                            CSTable::open(&s.cs_table()).copy_to(&args.destination,"cs_table");
                        }
                        _=> panic!("not yet implemented")
                    }
                }
            }
        }
        NewDiffusionExperiment(args) => {
            let cfg_file = Path::new(SEQUENCE_LIB).join(&args.alias).with_extension("json");
            let b_table = Path::new(&args.b_table);
            if !b_table.exists() {
                println!("cannot find specified b-table {:?}",b_table);
                return
            }
            match File::open(&cfg_file) {
                Err(_) => println!("alias {} not found!",args.alias),
                Ok(mut f) => {
                    let mut cfg_str = String::new();
                    f.read_to_string(&mut cfg_str).expect("trouble reading file");
                    let seq = find_seq_name_from_config(&cfg_str);
                    match seq {
                        Sequence::FSE_DTI => {
                            let seq_params = FseDtiParams::load(&cfg_file);
                            let mut exp_params = diffusion::generate_experiment(&seq_params,b_table);
                            let mut seqs:Vec<FseDti> = exp_params.iter().map(|params| FseDti::new(params.clone())).collect();
                            diffusion::build_cs_experiment(&mut seqs,&args.destination);
                        }
                        _=> panic!("not yet implemented")
                    }
                }
            }
        }
        NewSetup(args) => {
            let cfg_file = Path::new(SEQUENCE_LIB).join(&args.alias).with_extension("json");
            match File::open(&cfg_file) {
                Err(_) => println!("alias {} not found!",args.alias),
                Ok(mut f) => {
                    let mut cfg_str = String::new();
                    f.read_to_string(&mut cfg_str).expect("trouble reading file");
                    let seq = find_seq_name_from_config(&cfg_str);
                    match seq {
                        Sequence::FSE_DTI => {
                            let mut params = FseDtiParams::load(&cfg_file);
                            params.set_mode();
                            params.set_repetitions(2000);
                            let mut s = FseDti::new(params);
                            create_dir_all(&args.destination).expect("unable to create directory");
                            s.ppl_export(&args.destination,"setup",false,true);
                        }
                        _=> panic!("not yet implemented")
                    }
                }
            }
        }
        ApplySetup(args) => {
            if args.children.is_file() {
                sync_pprs(&args.setup_ppr,&vec![args.children.clone()]);
                if args.recursive.is_some(){
                    let r = args.recursive.unwrap();
                    let entries = find_files(&args.children, ".ppr", r);
                    sync_pprs(&args.setup_ppr,&entries);
                }
            }
            else {
                let r = args.recursive.unwrap_or(0);
                let entries = find_files(&args.children, ".ppr", r);
                sync_pprs(&args.setup_ppr,&entries);
                println!("updating {} ppr files",entries.len());
            }
        }
        _=> {}
    }
}


fn find_seq_name_from_config(config_str:&str) -> Sequence {
    let reg_pat = r#""*name"*\s*:\s*"*(.*\w.*)""#;
    let reg = Regex::new(reg_pat).unwrap();
    let caps = reg.captures(&config_str);
    let name:String = caps.expect("name field not found in config!").get(1).map_or("", |m| m.as_str()).to_string();
    Sequence::encode(&name)
}


fn find_files(base_dir:&Path, pattern:&str, depth:u16) -> Vec<PathBuf> {
    let pattern_rep = (0..depth).map(|_| r"*\").collect::<String>();
    let pattern = format!("{}*{}",pattern_rep,pattern);
    let pat = base_dir.join(pattern);
    glob(pat.to_str().unwrap()).expect("failed to read glob pattern").flat_map(|m| m).collect()
}

#[test]
fn test(){

}