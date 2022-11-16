use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::format;
use serde::{Deserialize, Serialize};
use std::fs::{File,create_dir_all};
use std::io::{Read,Write};
use std::path::{Path, PathBuf};
use whoami;
use serde_json;
use crate::bart_wrapper::{bart_pics, BartPicsSettings};
//use crate::volume_manager::{VmState,VolumeManager,launch_volume_manager,launch_volume_manager_job,re_launch_volume_manager_job};
use crate::slurm::{self,BatchScript, JobState};
use std::process::{Command, exit};
use seq_lib::pulse_sequence::MrdToKspaceParams;
//use crate::config::{ProjectSettings, Recon};
use mr_data::mrd::{fse_raw_to_cfl, cs_mrd_to_kspace};
use headfile::headfile::{ReconHeadfile, Headfile};
use acquire::build::{HEADFILE_NAME,HEADFILE_EXT};
use crate::{cfl, utils};
use glob::glob;
use clap::Parser;
use serde_json::to_string;
use crate::cfl::{ImageScale, write_u16_scale};

pub const SCALE_FILENAME:&str = "volume_scale_info";
pub const DEFAULT_HIST_PERCENT:f32 = 0.9995;
pub const DEFAULT_IMAGE_CODE:&str = "t9";
pub const DEFAULT_IMAGE_TAG:&str = "imx";

pub fn test_updated() {
    let work_dir = Path::new("/privateShares/wa41/N60tacos.work/m01");
    let vmc_file = work_dir.join("vm_config");

    let mut vmc  = VolumeManagerConfig::default();
    vmc.remote_host = Some("stejskal".to_string());
    vmc.remote_user = Some("mrs".to_string());
    vmc.resource_dir = Some(PathBuf::from("/d/dev/221111/acq/m01"));
    vmc.is_scale_dependent = Some(true);
    vmc.is_scale_setter = Some(false);
    vmc.to_file(&work_dir);

    let vma = VolumeManagerArgs::new(work_dir,&vmc_file);
    let vma_file = vma.to_file();

    
    VolumeManager::launch(&vma_file);
    
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct VolumeManagerArgs {
    work_dir:PathBuf,
    config:PathBuf
}

#[derive(Debug,Serialize,Deserialize)]
pub struct VolumeManager{
    args:VolumeManagerArgs,
    config:VolumeManagerConfig,
    resources:Option<VolumeManagerResources>,
    kspace_data:Option<PathBuf>,
    image_data:Option<PathBuf>,
    image_output:Option<PathBuf>,
    image_scale:Option<f32>,
    state:VolumeManagerState,
}



#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct VolumeManagerConfig {
    resource_dir:Option<PathBuf>,
    remote_user:Option<String>,
    remote_host:Option<String>,
    recon_headfile:Option<ReconHeadfile>,
    recon_settings:Option<BartPicsSettings>,
    is_scale_dependent:Option<bool>,
    is_scale_setter:Option<bool>,
    scale_hist_percent:Option<f32>,
}

#[derive(Clone,Debug,Serialize,Deserialize)]
struct VolumeManagerResources {
    cs_table:PathBuf,
    raw_mrd:PathBuf,
    acq_complete:PathBuf,
    kspace_config:PathBuf,
    meta:Option<PathBuf>,
    scaling_info:Option<PathBuf>,
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub enum ResourceError {
    CsTableNotFound,
    MrdNotFound,
    MrdNotComplete,
    KspaceConfigNotFound,
    Unknown,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum VolumeManagerState {
    Idle,
    NeedsResources(ResourceError),
    FormattingKspace,
    Reconstructing,
    Scaling,
    WritingImageData,
    WritingHeadfile,
    Done,
}

impl VolumeManagerConfig {
    pub fn from_file(file_name:&Path) -> Self {
        let mut f = File::open(file_name).expect(&format!("unable to open {:?}",Self::file_name(work_dir)));
        let mut s = String::new();
        f.read_to_string(&mut s).expect("cannot read file");
        serde_json::from_str(&s).expect("cannot deserialize args")
    }

    pub fn to_file(&self,file_name:&Path) {
        let parent = file_name.parent().unwrap();
        if !parent.exists() {
            create_dir_all(&parent).expect(&format!("cannot create {:?}",parent));
        }
        let mut f = File::create(&file_name).expect(&format!("unable to create file: {:?}", file_name));
        let s = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        f.write_all(s.as_bytes()).expect("cannot write to file");
    }

    pub fn default() -> Self {
        Self {
            resource_dir: None,
            remote_user: None,
            remote_host: None,
            recon_headfile: Some(ReconHeadfile::default()),
            recon_settings: Some(BartPicsSettings::default()),
            is_scale_dependent: None,
            is_scale_setter: None,
            scale_hist_percent: None
        }
    }

    pub fn file_name(work_dir:&Path) -> PathBuf {
        work_dir.join("volume_manager_config")
    }
}

impl VolumeManagerArgs {
    pub fn file_name(work_dir:&Path) -> PathBuf {
        let vm_args_filename = "vm_args";
        work_dir.join(vm_args_filename)
    }

    pub fn to_file(&self) -> PathBuf {
        let filename = Self::file_name(&self.work_dir);
        let s = serde_json::to_string_pretty(&self).unwrap();
        let mut f = File::create(&filename).unwrap();
        f.write_all(s.as_bytes()).unwrap();
        filename.to_owned()
    }

    pub fn from_file(config_file:&Path) -> Self {
        let mut f = File::open(config_file).unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        serde_json::from_str(&s).unwrap()
    }
}

impl VolumeManagerArgs {
    pub fn new(work_dir:&Path,config_file:&Path) -> Self {
        Self {
            work_dir:work_dir.to_owned(),
            config:config_file.to_owned()
        }
    }
}


impl VolumeManagerResources {

    // pub fn find_cs_table(work_dir:&Path) -> Option<PathBuf> {
    //     get_first_match(work_dir,"cs_table")
    // }

    pub fn from_dir(work_dir:&Path) -> Result<Self,ResourceError> {
        let work_dir = &Self::resource_dir(work_dir);
        let cs_table = get_first_match(work_dir,"*cs_table").ok_or(ResourceError::CsTableNotFound)?;
        let raw_mrd = get_first_match(work_dir,"*.mrd").ok_or(ResourceError::MrdNotFound)?;
        let acq_complete = get_first_match(work_dir,"*.ac").ok_or(ResourceError::MrdNotComplete)?;
        let kspace_config = get_first_match(work_dir,"*.mtk").ok_or(ResourceError::KspaceConfigNotFound)?;
        let scaling_info = get_first_match(work_dir.parent().expect("directory has no parent"),SCALE_FILENAME);
        let meta = get_first_match(work_dir,"meta.txt");
        Ok(Self {
            cs_table,
            raw_mrd,
            acq_complete,
            kspace_config,
            meta,
            scaling_info,
        })
    }
    pub fn resource_dir(work_dir:&Path) -> PathBuf {
        let dir = work_dir.join("resources");
        if !dir.exists() {
            create_dir_all(&dir).expect("cannot create local resource directory");
        }
        dir.to_owned()
    }
    pub fn exist(work_dir:&Path) -> bool {
        Self::from_dir(work_dir).is_ok()
    }

    pub fn fetch(work_dir:&Path,vmc:&VolumeManagerConfig) {

        let vmc = vmc.clone();

        let local = Self::resource_dir(work_dir);
        let local = local.to_str().unwrap();

        match &vmc.remote_host {
            Some(host) => {
                let user = vmc.remote_user.unwrap();
                let resource_dir = vmc.resource_dir.unwrap();
                let resource_dir = resource_dir.to_str().unwrap();

                let mut cmd = Command::new("scp");
                cmd.args(vec![
                    format!("{}@{}:{}/*",user,host,resource_dir).as_str(),
                    local,
                ]);
                let o = cmd.output().expect("failed to launch scp");
                if !o.status.success() {
                    println!("scp failed... we should clean up");
                    println!("we tried to run:{:?}",cmd);
                    println!("{}",String::from_utf8(o.stdout.clone()).unwrap());
                }
                else {
                    println!("directory successfully transferred");
                }
            }
            None => {
                println!("remote hist not specified. Not fetching data");
            }
        }
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
enum StateAdvance {
    Succeeded,
    TerminalFailure,
    TryingAgainLater,
    AllWorkDone,
}

impl VolumeManager {
    fn image_vol(&self) -> PathBuf {
        self.args.work_dir.join("image_vol")
    }
    fn kspace_vol_name(&self) -> PathBuf {
        self.args.work_dir.join("kspace_vol")
    }
    fn file_name(work_dir: &Path) -> PathBuf {
        work_dir.join("volume_manager")
    }
    pub fn to_file(&self) {
        let mut f = File::create(Self::file_name(&self.args.work_dir)).expect("cannot create file");
        let s = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        f.write_all(s.as_bytes()).expect("cannot write to file");
    }
    pub fn new(vma:&VolumeManagerArgs) -> Self {
        let config = VolumeManagerConfig::from_file(&vma.work_dir);
        let vm = Self {
            args: vma.to_owned(),
            config: config,
            resources: None,
            kspace_data: None,
            image_data: None,
            image_output: None,
            image_scale: None,
            state: VolumeManagerState::Idle
        };
        vm.to_file();
        vm
    }
    pub fn from_file(work_dir: &Path) -> Option<Self> {
        let f = File::open(Self::file_name(work_dir));
        match f {
            Ok(mut f) => {
                let mut s = String::new();
                f.read_to_string(&mut s).expect("cannot read from file");
                Some(serde_json::from_str(&s).expect("cannot deserialize struct"))
            }
            Err(_) => None
        }
    }
    pub fn open(args:&VolumeManagerArgs) -> Self {
        match Self::from_file(&args.work_dir) {
            Some(vm) => {
                println!("loading previous volume manager");
                vm
            }
            None => {
                println!("initializing new volume manager");
                Self::new(args)
            }
        }
    }
    pub fn launch(args:&Path) {

        let vma = VolumeManagerArgs::from_file(args);
        let mut vm = Self::open(&vma);
        println!("vol_man = {:?}",vm);

        // attempt to advance state and return a success/failure code
        // advancement either succeeds, fails, or needs to try again later
        use StateAdvance::*;

        loop {
            let status = vm.advance_state();
            println!("state advance returned with code {:?}",status);
            println!("current state is {:?}",vm.state);
            vm.to_file();
            match status {
                Succeeded => continue,
                TryingAgainLater => {
                    let this_exe = std::env::current_exe().expect("couldn't determine the current executable");
                    let mut cmd = Command::new(this_exe);
                    cmd.args(
                        vec![
                            "volume-manager",
                            "launch",
                            args.to_str().unwrap()
                        ]
                    );
                    let mut b = BatchScript::new("volume_manager");
                    b.commands.push(format!("{:?}",cmd));
                    let jid = b.submit_later(&vm.args.work_dir,2*60);
                    break
                }
                // don't reschedule
                // maybe write to a log or send an email
                TerminalFailure | AllWorkDone => break
            }
        }
    }

    fn advance_state(&mut self) -> StateAdvance {
        use VolumeManagerState::*;
        match &self.state {
            Idle | NeedsResources(_) => {
                println!("gathering and checking resources ...");
                // sync resources here
                VolumeManagerResources::fetch(&self.args.work_dir,&self.config);
                match VolumeManagerResources::from_dir(&self.args.work_dir) {
                    Ok(resources) => {
                        self.resources = Some(resources);
                        self.state = FormattingKspace;
                        StateAdvance::Succeeded
                    },
                    Err(e) => {
                        self.state = NeedsResources(e);
                        StateAdvance::TryingAgainLater
                    }
                }
            }
            FormattingKspace => {
                println!("formatting kspace ...");
                match &self.resources {
                    Some(res) => {
                        let mtk = MrdToKspaceParams::from_file(&res.kspace_config);
                        cs_mrd_to_kspace(&res.raw_mrd, &res.cs_table, &self.kspace_vol_name(), &mtk);
                        self.kspace_data = Some(self.kspace_vol_name());
                        self.state = Reconstructing;
                        StateAdvance::Succeeded
                    }
                    None => {
                        println!("resources not available");
                        StateAdvance::TerminalFailure
                    }
                }
            }
            Reconstructing => {
                println!("reconstructing kspace ...");
                match &self.kspace_data {
                    Some(kspace) => {
                        let mut recon_settings = self.config.recon_settings.unwrap_or(BartPicsSettings::default());
                        bart_pics(kspace, &self.image_vol(), &mut recon_settings);
                        self.image_data = Some(self.image_vol());
                        self.state = Scaling;
                        StateAdvance::Succeeded
                    }
                    None => {
                        //self.state = FormattingKspace;
                        println!("kspace data is not available to reconstruct!");
                        StateAdvance::TerminalFailure
                    }
                }
            }
            Scaling => {
                println!("determining image scale ...");
                /*
                If this volume is determining the scale of the other volumes, it will find the proper
                scale and write it to a file in the parent directory
                 */
                match self.config.is_scale_setter.unwrap_or(false) {
                    true => {
                        let scale_file = self.args.work_dir.parent().expect("path has no parent").join(SCALE_FILENAME);
                        write_u16_scale(&self.image_data.clone().unwrap(), self.config.scale_hist_percent.unwrap_or(0.9995) as f64, &scale_file);
                        let scale = ImageScale::from_file(&scale_file);
                        self.image_scale = Some(scale.scale_factor);
                        self.state = WritingImageData;
                        return StateAdvance::Succeeded
                    }
                    _ => { /*no op*/}
                }

                /*
                If the volume is scale-dependent, it will look for a scale file in the parent directory and use it for scaling
                 */
                match &self.image_data {
                    Some(image) => {
                        match self.config.is_scale_dependent.unwrap_or(false) {
                            false => {
                                let scale = cfl::find_u16_scale(image, self.config.scale_hist_percent.unwrap_or(0.9995) as f64);
                                self.image_scale = Some(scale);
                                self.state = WritingImageData;
                                StateAdvance::Succeeded
                            }
                            true => {
                                let scale_file = get_first_match(&self.args.work_dir.parent().unwrap(),SCALE_FILENAME);
                                match &scale_file {
                                    Some(scale_file) => {
                                        let scale = ImageScale::from_file(scale_file);
                                        self.image_scale = Some(scale.scale_factor);
                                        self.state = WritingImageData;
                                        StateAdvance::Succeeded
                                    }
                                    None => {
                                        // schedule to run again later
                                        println!("scale file not yet found!. Trying again later.");
                                        StateAdvance::TryingAgainLater
                                    }
                                }
                            }
                        }
                    },
                    None => {
                        println!("image data is not available to scale!");
                        StateAdvance::TerminalFailure
                    }
                }
            }
            WritingImageData => {
                println!("writing image data ...");
                match self.image_scale {
                    Some(scale) => {
                        let image_dir = self.args.work_dir.join("images");
                        if !image_dir.exists() {
                            create_dir_all(&image_dir).expect(&format!("cannot create dir: {:?}",image_dir));
                        }
                        self.image_output = Some(image_dir.clone());
                        let image = self.image_data.clone().expect("where did the image data go!?");
                        let image_code = match &self.config.recon_headfile.clone() {
                            Some(rh) => rh.image_code.clone(),
                            None => DEFAULT_IMAGE_CODE.to_string()
                        };
                        let image_tag = match &self.config.recon_headfile.clone() {
                            Some(rh) => rh.image_tag.clone(),
                            None => DEFAULT_IMAGE_TAG.to_string()
                        };
                        let raw_prefix = format!("{}{}", image_code, image_tag);
                        let vname = self.args.work_dir.file_name().unwrap().to_str().unwrap();
                        cfl::to_civm_raw_u16(&image, &image_dir, vname, &raw_prefix, scale);
                        self.state = WritingHeadfile;
                        StateAdvance::Succeeded
                    }
                    None => {
                        println!("image scale is undetermined!");
                        StateAdvance::TerminalFailure
                    }
                }
            }
            WritingHeadfile => {
                println!("writing headfile ...");
                match &self.resources.clone().unwrap().meta {
                    Some(meta) => {
                        std::fs::copy(&meta, &meta.with_file_name("temp")).expect("cannot copy headfile to temp");
                        match &self.config.recon_headfile {
                            Some(recon_headfile) => {
                                println!("writing to {:?}",meta.with_file_name("temp"));
                                Headfile::open(&meta.with_file_name("temp")).append(&recon_headfile.to_hash());
                            }
                            None => println!("recon headfile not specified. Head file will be incomplete")
                        }
                        let img_dir = self.image_output.clone().expect("image directory not defined!");
                        let vname = self.args.work_dir.file_name().unwrap().to_str().unwrap();
                        std::fs::rename(&meta.with_file_name("temp"),img_dir.join(vname).with_extension("headfile")).expect("cannot move headfile");
                    }
                    None => println!("meta data not found. No headfile will be written")
                }
                self.state = Done;
                StateAdvance::Succeeded
            }
            Done => {
                println!("all work is complete.");
                StateAdvance::AllWorkDone
            }
        }
    }
}

fn get_first_match(dir:&Path,pattern:&str) -> Option<PathBuf>  {
    let pat = dir.join(pattern);
    let pat = pat.to_str().expect("cannot coerce to str");
    let matches:Vec<PathBuf> = glob(pat).expect("Failed to read glob pattern").flat_map(|m| m).collect();
    match matches.is_empty() {
        true => None,
        false => Some(matches[0].clone())
    }
}