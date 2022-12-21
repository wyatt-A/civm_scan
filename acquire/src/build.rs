use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use seq_lib::pulse_sequence::{Build, SequenceParameters, DWSequenceParameters, Initialize, AcqDims, ScoutConfig, AdjustmentParameters, SequenceLoadError};
use headfile::headfile::Headfile;
use dyn_clone::clone_box;
use encoding::all::ISO_8859_1;
use encoding::{DecoderTrap, EncoderTrap, Encoding};
use glob::glob;
use regex::Regex;
use seq_lib::fse_dti::FseDtiParams;
use crate::args::{ApplySetupArgs, NewAdjArgs, NewArgs, NewConfigArgs, NewDiffusionExperimentArgs};
use std::fs::copy;
use seq_lib::mgre::MgreParams;
use seq_lib::multi_echo_2d::Me2DParams;
use seq_lib::one_pulse::OnePulseParams;
use seq_lib::rfcal::RfCalParams;
use seq_lib::scout::ScoutParams;
use seq_lib::se_dti::SeDtiParams;
use utils;
use crate::scout::ScoutViewSettings;

//const SEQUENCE_LIB:&str = r"C:/workstation/civm_scan/sequence_library";
//const SEQUENCE_LIB:&str = "/Users/Wyatt/sequence_library";
//const SEQUENCE_LIB:&str = "/Users/Wyatt/IdeaProjects/test_data/seq_lib";
//const SEQUENCE_LIB:&str = r"C:\Users\waust\OneDrive\Desktop\test_data\seq_lib";
//pub const SEQUENCE_LIB:&str = r"C:\Users\waust\IdeaProjects\civm_scan\test_env\sequence_library";
pub const SEQUENCE_LIB:&str = r"C:\workstation\dev\civm_scan\test_env\sequence_library";

pub const HEADFILE_NAME:&str = "meta";
pub const HEADFILE_EXT:&str = "txt";


pub enum Sequence {
    FseDti,
    SeDti,
    MGRE,
    GRE,
    Scout,
    OnePulse,
    RfCal,
    Me2D,
}

impl Sequence {
    pub fn list() -> String{
        vec![
            Self::decode(&Self::FseDti),
            Self::decode(&Self::SeDti),
            Self::decode(&Self::MGRE),
            Self::decode(&Self::GRE),
            Self::decode(&Self::Scout),
            Self::decode(&Self::OnePulse),
            Self::decode(&Self::RfCal),
            Self::decode(&Self::Me2D),
        ].join("\n")
    }
    pub fn encode(name:&str) -> Self {
        match name {
            "fse_dti" => Self::FseDti,
            "se_dti" => Self::SeDti,
            "mgre" => Self::MGRE,
            "gre" => Self::GRE,
            "scout" => Self::Scout,
            "one_pulse" => Self::OnePulse,
            "rf_cal" => Self::RfCal,
            "multi_echo_2d" => Self::Me2D,
            _=> panic!("name not recognized")
        }
    }
    pub fn decode(&self) -> String {
        match &self {
            Self::FseDti => String::from("fse_dti"),
            Self::SeDti => String::from("se_dti"),
            Self::MGRE => String::from("mgre"),
            Self::GRE => String::from("gre"),
            Self::Scout => String::from("scout"),
            Self::OnePulse => String::from("one_pulse"),
            Self::RfCal => String::from("rf_cal"),
            Self::Me2D => String::from("multi_echo_2d"),
        }
    }
}

#[derive(Clone)]
pub struct ContextParams {
    pub export_dir:PathBuf,
    pub adjustment:Option<PathBuf>,
    pub build:bool,
}

impl ContextParams {
    pub fn from_adjustments(adjustment_file:&Path,export_dir:&Path,to_build:bool) -> Self {
        Self {
            export_dir: export_dir.to_owned(),
            adjustment: Some(adjustment_file.to_owned()),
            build:to_build
        }
    }

    pub fn without_adjustments(export_dir:&Path,to_build:bool) -> Self {
        Self {
            export_dir: export_dir.to_owned(),
            adjustment: None,
            build: to_build
        }
    }
}


pub fn acq_dims(cfg_file:&Path) -> Result<AcqDims,SequenceLoadError> {
    Ok(load_params(cfg_file)?.acq_dims())
}

fn load_params(cfg_file:&Path) -> Result<Box<dyn SequenceParameters>,SequenceLoadError> {
    let cfg_str = read_to_string(cfg_file);
    Ok(match find_seq_name_from_config(&cfg_str) {
        Sequence::FseDti => {
            Box::new(FseDtiParams::load(&cfg_file)?)
        },
        Sequence::SeDti => {
            Box::new(SeDtiParams::load(&cfg_file)?)
        },
        Sequence::Scout => {
            Box::new(ScoutParams::load(&cfg_file)?)
        }
        Sequence::Me2D => {
            Box::new(Me2DParams::load(&cfg_file)?)
        }
        Sequence::MGRE => {
            Box::new(MgreParams::load(&cfg_file)?)
        }
        _=> panic!("not yet implemented")
    })
}


pub fn load_build_params(cfg_file:&Path) -> Result<Box<dyn Build>,SequenceLoadError> {
    let cfg_str = read_to_string(cfg_file);
    Ok(match find_seq_name_from_config(&cfg_str) {
        Sequence::FseDti => {
            FseDtiParams::load(&cfg_file)?.instantiate()
        },
        Sequence::SeDti => {
            SeDtiParams::load(&cfg_file)?.instantiate()
        },
        Sequence::Scout => {
            ScoutParams::load(&cfg_file)?.instantiate()
        }
        Sequence::OnePulse => {
            OnePulseParams::load(&cfg_file)?.instantiate()
        },
        Sequence::RfCal => {
            RfCalParams::load(&cfg_file)?.instantiate()
        },
        Sequence::Me2D => {
            Me2DParams::load(&cfg_file)?.instantiate()
        },
        Sequence::MGRE => {
            MgreParams::load(&cfg_file)?.instantiate()
        }
        _=> panic!("sequence not registered")
    })
}


pub fn load_adj_params(cfg_file:&Path) -> Result<Box<dyn AdjustmentParameters>,SequenceLoadError> {
    let cfg_str = read_to_string(cfg_file);
    Ok(match find_seq_name_from_config(&cfg_str) {
        Sequence::OnePulse => {
            Box::new(OnePulseParams::load(&cfg_file)?)
        },
        Sequence::RfCal => {
            Box::new(RfCalParams::load(&cfg_file)?)
        },
        _ => panic!("not yet implemented")
    })
}


pub fn load_scout_params(cfg_file:&Path) -> Result<Box<dyn ScoutConfig>,SequenceLoadError> {
    let cfg_str = read_to_string(cfg_file);
    Ok(match find_seq_name_from_config(&cfg_str) {
        Sequence::Scout => {
            Box::new(ScoutParams::load(&cfg_file)?)
        }
        _=> panic!("not yet implemented")
    })
}

pub fn validate(cfg_file:&Path) -> (bool,Option<String>) {
    match load_build_params(cfg_file){
        Err(_) => (false,Some(String::from("invalid sequence config file! Is the syntax correct?"))),
        Ok(build) => build.is_valid()
    }
}

pub fn load_dw_params(cfg_file:&Path) -> Result<Box<dyn DWSequenceParameters>,SequenceLoadError> {
    let cfg_str = read_to_string(cfg_file);
    Ok(match find_seq_name_from_config(&cfg_str) {
        Sequence::FseDti => {
            Box::new(FseDtiParams::load(&cfg_file)?)
        },
        Sequence::SeDti => {
            Box::new(SeDtiParams::load(&cfg_file)?)
        },
        _=> panic!("not yet implemented")
    })
}

pub fn new_simulation(args:&NewArgs) {
    let cfg_file = Path::new(SEQUENCE_LIB).join(&args.alias).with_extension("json");
    let mut params = load_params(&cfg_file).expect("cannot load parameters");
    params.configure_simulation();
    build_simulation(params,&args.context_params());
}

pub fn new(args:&NewArgs) {
    let cfg_file = Path::new(SEQUENCE_LIB).join(&args.alias).with_extension("json");
    let params = load_params(&cfg_file).expect("cannot load parameters");
    if !args.destination.exists() {
        create_dir_all(&args.destination).expect(&format!("unable to create directory: {:?}",args.destination));
    }
    build(params,&args.context_params());
}

pub fn new_setup(args:&NewArgs) {
    let cfg_file = Path::new(SEQUENCE_LIB).join(&args.alias).with_extension("json");
    let params = load_params(&cfg_file).expect("cannot load parameters");
    build_setup(params,&args.context_params());
}

pub fn new_adjustment(args:&NewAdjArgs) {
    let cfg_file = Path::new(SEQUENCE_LIB).join(&args.alias).with_extension("json");
    let params = load_adj_params(&cfg_file).expect("cannot load parameters");
    build_adj(params,&args.context_params());
}

pub fn new_config(args:&NewConfigArgs){
    let seq = Sequence::encode(&args.name);
    let path_out = Path::new(SEQUENCE_LIB).join(&args.alias).with_extension("json");
    if path_out.exists(){
        println!("{} already exists. Choose a different alias.",&args.alias);
        return
    }
    match seq {
        Sequence::FseDti => {
            FseDtiParams::write_default(&path_out);
        },
        Sequence::SeDti => {
            SeDtiParams::write_default(&path_out);
        }
        Sequence::Scout => {
            ScoutParams::write_default(&path_out);
        }
        Sequence::OnePulse => {
            OnePulseParams::write_default(&path_out);
        }
        Sequence::RfCal => {
            RfCalParams::write_default(&path_out);
        }
        Sequence::Me2D => {
            Me2DParams::write_default(&path_out);
        }
        Sequence::MGRE => {
            MgreParams::write_default(&path_out);
        }
        _=> panic!("not yet implemented")
    }
}

pub fn new_diffusion_experiment(args: &NewDiffusionExperimentArgs) {
    let cfg_file = Path::new(SEQUENCE_LIB).join(&args.alias).with_extension("json");
    let b_table = Path::new(&args.b_table);
    if !b_table.exists() {
        println!("cannot find specified b-table {:?}",b_table);
        return
    }
    let params = load_dw_params(&cfg_file).expect("cannot load parameters");

    if !args.destination.exists() {
        create_dir_all(&args.destination).expect(&format!("unable to create directory: {:?}",args.destination));
    }
    build_diffusion_experiment(params,&args.context_params(),b_table);
}

pub fn new_scout_experiment(args:&NewArgs) {
    let cfg_file = Path::new(SEQUENCE_LIB).join(&args.alias).with_extension("json");
    let params = load_scout_params(&cfg_file).expect("cannot load parameters");
    build_scout_experiment(params,&args.context_params(),&ScoutViewSettings::default());
}

pub fn apply_setup(args:&ApplySetupArgs) {
    match utils::find_files(&args.children,"ppr",true) {
        Some(entries) => sync_pprs(&args.setup_ppr,&entries),
        None => {
            let none = Vec::<PathBuf>::new();
            sync_pprs(&args.setup_ppr,&none);
        }
    }
}

pub fn find_seq_name_from_config(config_str:&str) -> Sequence {
    let reg_pat = r#""*name"*\s*:\s*"*(.*\w.*)""#;
    let reg = Regex::new(reg_pat).unwrap();
    let caps = reg.captures(&config_str);
    let name:String = caps.expect("name field not found in config!").get(1).map_or("", |m| m.as_str()).to_string();
    Sequence::encode(&name)
}

pub fn find_files(base_dir:&Path, pattern:&str, depth:u16) -> Vec<PathBuf> {
    let pattern_rep = (0..depth).map(|_| r"*\").collect::<String>();
    let pattern = format!("{}*{}",pattern_rep,pattern);
    let pat = base_dir.join(pattern);
    glob(pat.to_str().unwrap()).expect("failed to read glob pattern").flat_map(|m| m).collect()
}

pub fn read_to_string(file_path:&Path) -> String {
    let mut f = File::open(file_path).expect(&format!("cannot open {:?}",file_path));
    let mut s = String::new();
    f.read_to_string(&mut s).expect("cannot read from file");
    s
}

pub fn sync_pprs(ppr_template:&Path,to_sync:&Vec<PathBuf>) {
    let template = read_ppr(ppr_template);
    let map = ppr_var_map(&template).expect("no ppr parameters found!");

    to_sync.iter().for_each(|file| {
        let mut to_modify = read_ppr(file);
        to_modify = update_ppr(&to_modify,&map);
        write_ppr(file,&to_modify);
    });
}

pub fn read_ppr(ppr_file:&Path) -> String {
    let mut f = File::open(ppr_file).expect("cannot open file");
    let mut bytes = Vec::<u8>::new();
    f.read_to_end(&mut bytes).expect("cannot read file");
    ISO_8859_1.decode(&bytes, DecoderTrap::Strict).expect("cannot decode ppr bytes")
}

pub fn write_ppr(ppr_file:&Path,ppr_string:&str) {
    let mut f = File::create(ppr_file).expect("cannot create file");
    let bytes = ISO_8859_1.encode(ppr_string,EncoderTrap::Strict).expect("cannot encode string");
    f.write_all(&bytes).expect("trouble writing to file");
}

pub fn ppr_var_map(ppr_string:&str) -> Option<HashMap<String,String>> {
    let var_reg = Regex::new(r":VAR (.*?), ([-0-9]+)").expect("invalid regex");

    let freq_reg = Regex::new(r":OBSERVE_FREQUENCY").expect("invalid regex");

    let mut map = HashMap::<String,String>::new();
    let str = ppr_string.to_owned();
    let lines:Vec<String> = str.lines().map(|s| s.to_string()).collect();
    lines.iter().for_each(|line| {
        let captures = var_reg.captures(line);
        match captures {
            Some(capture) => {
                let cap1 = capture.get(1).expect("ppr variable not found");
                let cap2 = capture.get(2).expect("ppr value not found");
                let var_name = cap1.as_str().to_string();
                let value = cap2.as_str().to_string();
                map.insert(var_name,value);
            },
            None => {}
        }
        match freq_reg.is_match(line) {
            true => {
                map.insert(String::from("OBSERVE_FREQUENCY"),line.clone());
            }
            _=> {}
        }
    });
    match map.is_empty() {
        true => None,
        false => Some(map)
    }
}

pub fn update_ppr(ppr_string:&str,var_map:&HashMap<String,String>) -> String {
    let mut str = ppr_string.to_owned();
    var_map.iter().for_each(|(key,value)| {
        let mut lines:Vec<String> = str.lines().map(|s| s.to_string()).collect();
        lines.iter_mut().for_each(|line| {
            let u_var = update_ppr_var_line(line, key, &value);
            match u_var {
                Some((new_string,_)) => {
                    *line = new_string;
                }
                None => {}
            }
            match key.as_str() {
                "OBSERVE_FREQUENCY" => {
                    match update_ppr_freq_line(line,value.as_str()) {
                        Some(new_line) => {
                            *line = new_line;
                        }
                        None => {}
                    }
                }
                _=> {}
            }
        });
        str = lines.join("\n")
    });
    str
}

fn update_ppr_var_line(line:&str, var_name:&str, new_value:&str) -> Option<(String,String)> {
    let reg = Regex::new(&format!(":VAR {}, ([-0-9]+)",var_name)).expect("invalid regex");
    let captures = reg.captures(line);
    match captures {
        Some(capture) => {
            let cap = capture.get(1).expect("ppr value not found");
            let old_value = cap.as_str().to_string();
            Some((format!(":VAR {}, {}",var_name,new_value),old_value))
        },
        None => None
    }
}

fn update_ppr_freq_line(line:&str, new_line:&str) -> Option<String> {
    let freq_reg = Regex::new(r":OBSERVE_FREQUENCY").expect("invalid regex");
    match freq_reg.is_match(line) {
        true => {
            Some(new_line.to_string())
        }
        _=> None
    }
}


pub fn build_simulation(sequence_params:Box<dyn SequenceParameters>,ctx:&ContextParams) {
    let params = clone_box(&*sequence_params);
    let mut to_build = params.instantiate();
    create_dir_all(&ctx.export_dir).expect("trouble building directory");
    let label = format!("{}_simulation",params.name());
    to_build.ppl_export(&ctx.export_dir,&label,true,ctx.build).expect("invalid event queue!");
    to_build.param_export(&ctx.export_dir);
}

pub fn build(sequence_params:Box<dyn SequenceParameters>,ctx:&ContextParams) {
    let mut params = clone_box(&*sequence_params);
    if ctx.adjustment.is_some(){
        params.set_adjustment_file(&ctx.adjustment.clone().unwrap());
    }
    match params.is_cs(){
        true =>{
            params.set_cs_table();
            let table = &params.cs_table().unwrap();
            copy(table,ctx.export_dir.join("cs_table")).expect("unable to copy cs table to destination");
        }
        _=> {}
    }
    let mut to_build = params.instantiate();
    create_dir_all(&ctx.export_dir).expect("trouble building directory");
    to_build.ppl_export(&ctx.export_dir,&params.name(),false,ctx.build).expect("invalid event queue!");
    params.mrd_to_kspace_params().to_file(&ctx.export_dir.join("mrd_to_kspace"));
    let h = Headfile::new(&ctx.export_dir.join(HEADFILE_NAME).with_extension(HEADFILE_EXT));
    h.append(&params.acq_params().to_hash());
    to_build.param_export(&ctx.export_dir);
}

pub fn build_adj(adj_params:Box<dyn AdjustmentParameters>,ctx:&ContextParams){
    let mut to_build = adj_params.instantiate();
    create_dir_all(&ctx.export_dir).expect("trouble building directory");
    to_build.ppl_export(&ctx.export_dir,&adj_params.name(),false,ctx.build).expect("invalid event queue!");
    to_build.param_export(&ctx.export_dir);
}

pub fn build_setup(sequence_params:Box<dyn SequenceParameters>,ctx:&ContextParams) {
    let mut setup_params = clone_box(&*sequence_params);
    setup_params.configure_setup();
    if ctx.adjustment.is_some(){
        setup_params.set_adjustment_file(&ctx.adjustment.clone().unwrap());
    }
    let mut to_build = setup_params.instantiate();
    create_dir_all(&ctx.export_dir).expect("trouble building directory");
    let label = format!("{}_setup",setup_params.name());
    to_build.ppl_export(&ctx.export_dir,&label,false,ctx.build).expect("invalid event queue!");
    to_build.param_export(&ctx.export_dir);
    match setup_params.is_cs(){
        true =>{
            let table = &setup_params.cs_table().unwrap();
            copy(table,ctx.export_dir.join("cs_table")).expect("unable to copy cs table to destination");
        }
        _=> {}
    }
}

pub fn build_scout_experiment(sequence_params:Box<dyn ScoutConfig>,ctx:&ContextParams,view_settings:&ScoutViewSettings) {
    let mut s = clone_box(&*sequence_params);

    let orientations = &view_settings.orientations;
    let fovs = &view_settings.fields_of_view;
    let samps = &view_settings.samples;

    orientations.iter().enumerate().for_each(|(index,orient)|{
        s.set_orientation(orient);
        s.set_samples(samps[index].clone());
        s.set_fov(fovs[index].clone());
        let label = utils::m_number(index,3);
        let dir = ctx.export_dir.join(&label);
        create_dir_all(&dir).expect("trouble building directory");
        s.mrd_to_kspace_params().to_file(&dir.join("mrd_to_kspace"));
        if ctx.adjustment.is_some(){
            s.set_adjustment_file(&ctx.adjustment.clone().unwrap());
        }
        let mut to_build = s.instantiate();
        let h = Headfile::new(&dir.join(HEADFILE_NAME).with_extension(HEADFILE_EXT));
        h.append(&s.acq_params().to_hash());
        to_build.ppl_export(&dir,&label,false,ctx.build).expect("invalid event queue!");
        to_build.param_export(&dir);
    });
}

pub fn build_diffusion_experiment(sequence_params:Box<dyn DWSequenceParameters>,ctx:&ContextParams,b_table:&Path) {
    let mut s = clone_box(&*sequence_params);
    let b_val = s.b_value();
    let b_table = read_b_table(b_table);
    let n = b_table.len();
    let w = ((n-1) as f32).log10().floor() as usize + 1;
    let formatter = |index:usize| format!("m{:0width$ }",index,width=w);
    b_table.iter().enumerate().for_each(|(index,exp)| {
        let scale = exp.0;
        let direction = (exp.1,exp.2,exp.3);
        s.set_b_value(b_val*scale);
        s.set_b_vec(direction);
        s.set_cs_table();
        if ctx.adjustment.is_some(){
            s.set_adjustment_file(&ctx.adjustment.clone().unwrap());
        }
        let label = formatter(index);
        let dir = ctx.export_dir.join(&label);
        create_dir_all(&dir).expect("trouble building directory");
        let mut to_build = s.instantiate();
        s.mrd_to_kspace_params().to_file(&dir.join("mrd_to_kspace"));
        let h = Headfile::new(&dir.join(HEADFILE_NAME).with_extension(HEADFILE_EXT));
        h.append(&s.acq_params().to_hash());
        h.append(&s.diffusion_params().to_hash());
        to_build.ppl_export(&dir,&label,false,ctx.build).expect("invalid event queue!");
        to_build.param_export(&dir);
        match s.is_cs() {
            true => {
                let table = &s.cs_table().unwrap();
                copy(table,dir.join("cs_table")).expect("unable to copy cs table to destination");
            }
            false => {}
        }
    })
}

pub fn read_b_table(b_table:&Path) -> Vec<(f32,f32,f32,f32)>{
    let mut f = File::open(b_table).expect("b_vec table not found");
    let mut file_string = String::new();
    f.read_to_string(&mut file_string).expect("trouble reading from file");
    let mut b_table = Vec::<(f32,f32,f32,f32)>::new();
    file_string.lines().for_each(|line| {
        if !line.starts_with("#") && !line.is_empty() {
            let s = line.split(",");
            let values:Vec<f32> = s.map(|elem| elem.trim().parse().expect(&format!("unable to parse {}",elem))).collect();
            if values.len() == 4 {
                b_table.push((values[0],values[1],values[2],values[3]));
            }
        }
    });
    b_table
}