use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::format;
use serde::{Deserialize, Serialize};
use std::fs::{File,create_dir_all};
use std::io::{Read,Write};
use std::path::{Path, PathBuf};
use whoami;
use serde_json;
use crate::bart_wrapper::{bart_pics};
use crate::slurm::{self,BatchScript, JobState};
use std::process::{Command, exit};
use seq_lib::pulse_sequence::MrdToKspaceParams;
//use crate::config::{ProjectSettings, Recon};
use mr_data::mrd::{fse_raw_to_cfl, cs_mrd_to_kspace};
use headfile::headfile::{ReconHeadfile, Headfile};
use acquire::build::{HEADFILE_NAME,HEADFILE_EXT};
use clap::Parser;
use serde_json::to_string;
use mr_data::cfl::{self, ImageScale, write_u16_scale};
use crate::recon_config::{ConfigFile, RemoteSystem, VolumeManagerConfig};

pub const SCALE_FILENAME:&str = "volume_scale_info";
pub const DEFAULT_HIST_PERCENT:f32 = 0.9995;
pub const DEFAULT_IMAGE_CODE:&str = "t9";
pub const DEFAULT_IMAGE_TAG:&str = "imx";


#[derive(Debug,Serialize,Deserialize)]
pub struct VolumeManager{
    config:PathBuf,
    state:VolumeManagerState,
    kspace_data:Option<PathBuf>,
    image_data:Option<PathBuf>,
    image_output:Option<PathBuf>,
    image_scale:Option<f32>,
    slurm_job_id:Option<u32>,
    resources:Option<VolumeManagerResources>,
}

#[derive(Clone,Debug,Serialize,Deserialize)]
struct VolumeManagerResources {
    cs_table:PathBuf,
    raw_mrd:PathBuf,
    acq_complete:PathBuf,
    kspace_config:PathBuf,
    meta:Option<PathBuf>,
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub enum ResourceError {
    CsTableNotFound,
    MrdNotFound,
    MrdNotComplete,
    KspaceConfigNotFound,
    FetchError,
    Unknown,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum VolumeManagerState {
    Idle,
    NeedsResources(ResourceError),
    FormattingKspace,
    Reconstructing,
    Filtering,
    Scaling,
    WritingImageData,
    WritingHeadfile,
    CleaningUp,
    SendingToArchiveEngine,
    Done,
}


impl VolumeManager {

    pub fn read(config:&Path) -> Option<Self> {
        let state_file = config.with_extension(Self::file_ext());
        match state_file.exists() {
            false => None,
            true => {
                let t = utils::read_to_string(config,&Self::file_ext());
                Some(toml::from_str(&t).expect("volume manager state file is corrupt. What happened?"))
            }
        }
    }

    pub fn open(config:&Path) -> Self {
        let state_file = config.with_extension(Self::file_ext());
        match state_file.exists() {
            true => {
                println!("state file already exists. will not overwrite ...");
                let t = utils::read_to_string(config,&Self::file_ext());
                toml::from_str(&t).expect("volume manager state file is corrupt. What happened?")
            }
            false => {
                println!("state file not found. creating a new one ...");
                Self::new(config)
            }
        }
    }
    pub fn config(&self) -> VolumeManagerConfig {
        VolumeManagerConfig::from_file(&self.config)
    }
    fn to_file(&self) {
        let t = toml::to_string(&self).unwrap();
        utils::write_to_file(&self.config,&Self::file_ext(),&t);
    }
    fn file_ext() -> String {
        String::from("vol_man")
    }
    fn new(config:&Path) -> Self {
        Self {
            config: config.to_owned(),
            state: VolumeManagerState::Idle,
            resources: None,
            kspace_data: None,
            image_data: None,
            image_output: None,
            image_scale: None,
            slurm_job_id: None,
        }
    }
}


impl VolumeManagerResources {

    pub fn open(config:&Path) -> Result<Self,ResourceError> {
        match Self::fetch(config) {
            Some(res_dir) => {
                let cs_table = utils::get_first_match(&res_dir, "*cs_table").ok_or(ResourceError::CsTableNotFound)?;
                let raw_mrd = utils::get_first_match(&res_dir, "*.mrd").ok_or(ResourceError::MrdNotFound)?;
                let acq_complete = utils::get_first_match(&res_dir, "*.ac").ok_or(ResourceError::MrdNotComplete)?;
                let kspace_config = utils::get_first_match(&res_dir, "*.mtk").ok_or(ResourceError::KspaceConfigNotFound)?;
                let meta = utils::get_first_match(&res_dir, "meta.txt");
                Ok(Self {
                    cs_table,
                    raw_mrd,
                    acq_complete,
                    kspace_config,
                    meta,
                })
            }
            None => Err(ResourceError::FetchError)
        }
    }
    fn resource_dir(config:&Path) -> PathBuf {
        let local_dir = config.with_file_name("resource");
        if !local_dir.exists() {
            println!("creating local dir: {:?}",local_dir);
            create_dir_all(&local_dir).expect("cannot create local resource directory");
        }else {
            println!("{:?} already exists",local_dir);
        }
        local_dir.to_owned()
    }

    fn fetch(config:&Path) -> Option<PathBuf> {
        /*
            get settings from config (remote user,host, and data directory)
            if local resource dir doesn't exist, create one
            use scp to transfer remote files to resource dir
            if scp fails, the resource dir is removed
            return the resource directory if success, None if failed
         */
        let settings = VolumeManagerConfig::from_file(config);
        let user = &settings.project_settings.scanner_settings.remote_user;
        let host = &settings.project_settings.scanner_settings.remote_host;
        let dir = &settings.vm_settings.resource_dir.join("*");

        let dir_str = dir.clone().into_os_string();
        let dir_str = dir_str.to_str().unwrap();

        let remote_system = format!("{}@{}",user,host);
        // maybe test remote system connection here

        let local_dir = Self::resource_dir(config);
        let local_dir_str = local_dir.clone().into_os_string();
        let local_dir_str = local_dir_str.to_str().unwrap();

        let mut scp_command = Command::new("scp");
        scp_command.args(vec![
            &format!("{}:{}",remote_system,dir_str),
            &format!("{}/",local_dir_str)
        ]);

        println!("attempting to run {:?}",scp_command);

        let o = scp_command.output().expect(&format!("failed to launch {:?}",scp_command));

        match o.status.success() {
            true => {
                println!("scp successful");
                Some(local_dir)
            },
            false => {
                println!("scp failed with error:\n {}",String::from_utf8(o.stderr.clone()).unwrap_or(String::from("unknown")));
                println!("removing resource directory ...");
                //std::fs::remove_dir_all(&local_dir).expect("unable to clean up resource directory");
                None
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

    pub fn m_number(&self) -> String {
        let settings = self.config();
        settings.m_number()
    }

    pub fn work_dir(&self) -> &Path {
        self.config.parent().expect(&format!("volume manager config has no parent {:?}",self.config))
    }

    pub fn scale_file(&self) -> PathBuf {
        self.work_dir().parent().expect("volume manager work dir has no parent").join(SCALE_FILENAME)
    }

    pub fn name(&self) -> String {
        let settings = self.config();
        settings.name()
    }

    fn kspace_file(&self) -> PathBuf {
        self.work_dir().join(format!("{}_kspace",self.name()))
    }

    fn image_space_file(&self) -> PathBuf {
        self.work_dir().join(format!("{}_imspace",self.name()))
    }

    fn image_dir(&self) -> PathBuf {
        self.work_dir().join(format!("{}images",self.name()))
    }

    pub fn launch_with_slurm_later(config:&Path,seconds_later:u32) -> u32 {
        let mut vm = VolumeManager::open(config);
        let mut bs = BatchScript::new(&vm.name(),&vec![Self::launch_cmd(config)]);
        bs.options.partition = String::from("reconstruction");
        bs.options.output = config.with_file_name("slurm-%j").with_extension("out").into_os_string().to_str().unwrap().to_string();
        let jid = bs.submit_later(vm.work_dir(),seconds_later);
        vm.slurm_job_id = Some(jid);
        vm.to_file();
        jid
    }

    pub fn launch_with_slurm_now(config:&Path) -> u32 {
        let mut vm = VolumeManager::open(config);
        let mut bs = BatchScript::new(&vm.name(),&vec![Self::launch_cmd(config)]);
        bs.options.partition = String::from("reconstruction");
        bs.options.output = config.with_file_name("slurm-%j").with_extension("out").into_os_string().to_str().unwrap().to_string();
        let jid = bs.submit_now(vm.work_dir());
        vm.slurm_job_id = Some(jid);
        vm.to_file();
        jid
    }

    fn launch_cmd(config:&Path) -> Command {
        let this_exe = std::env::current_exe().expect("couldn't determine the current executable");
        let mut cmd = Command::new(this_exe);
        cmd.args(
            vec![
                "volume-manager",
                "launch",
                config.to_str().unwrap()
            ]
        );
        cmd
    }

    pub fn slurm_status(&self) -> Option<JobState> {
        match self.slurm_job_id {
            Some(jid) => Some(slurm::get_job_state(jid,60)),
            None => None
        }
    }

    pub fn state_string(&self) -> String {
        format!("{:?}",self.state)
    }

    pub fn launch_with_srun(config:&Path) {

        let this_exe = std::env::current_exe().expect("couldn't determine the current executable");
        let this_exe = this_exe.into_os_string();
        let this_exe = this_exe.to_str().unwrap();

        let mut cmd = Command::new("srun");
        cmd.args(
            vec![
                this_exe,
                "--mem=30G",
                "volume-manager",
                "launch",
                config.to_str().unwrap()
            ]
        );
        println!("running {:?}",cmd);

        let o = cmd.output().expect("failed to launch srun");
        println!("{}",String::from_utf8(o.stdout).unwrap());
    }

    pub fn no_cluster_scheduling() -> bool {
        // check to see if we are running sequentially or in parallel on the cluster
        match std::env::var("SLURM_DISABLE").unwrap_or(String::from("no")).as_str() {
            "yes" | "y" | "true" | "1" => {
                true
            },
            _=> false
        }
    }

    pub fn launch(config:&Path) {

        let mut vm = VolumeManager::open(config);

        println!("loaded volume manager state: {:?}",vm);

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
                    VolumeManager::launch_with_slurm_later(config,120);
                    break
                }
                TerminalFailure => {
                    println!("volume manager cannot continue. Will not reschedule.");
                    if vm.config().is_slurm_disabled() {
                        panic!("cannot continue with any remaining volume managers");
                    }
                    break
                },
                AllWorkDone => {
                    println!("all work is complete.");
                    break
                }
            }
        }
    }

    fn advance_state(&mut self) -> StateAdvance {

        let settings = self.config();

        use VolumeManagerState::*;
        match &self.state {
            Idle | NeedsResources(_) => {
                println!("gathering and checking resources ...");
                match VolumeManagerResources::open(&self.config) {
                    Ok(resources) => {
                        self.resources = Some(resources);
                        self.state = FormattingKspace;
                        StateAdvance::Succeeded
                    },
                    Err(e) => {
                        self.state = NeedsResources(e);
                        match settings.is_slurm_disabled() {
                            false => StateAdvance::TryingAgainLater,
                            true => StateAdvance::TerminalFailure
                        }
                    }
                }
            }
            FormattingKspace => {
                println!("formatting kspace ...");
                match &self.resources {
                    Some(res) => {
                        let mtk = MrdToKspaceParams::from_file(&res.kspace_config);
                        cs_mrd_to_kspace(&res.raw_mrd, &res.cs_table, &self.kspace_file(), &mtk);
                        self.kspace_data = Some(self.kspace_file());
                        self.state = Reconstructing;
                        StateAdvance::Succeeded
                    }
                    None => {
                        println!("resources not available");
                        StateAdvance::TerminalFailure
                    }
                }
            }
            //todo!(think about how to use the cluster's temp directory that lives in memory. "Needs special cleanup care")
            Reconstructing => {
                println!("reconstructing kspace ...");
                match &self.kspace_data {
                    Some(kspace) => {
                        let image_space = self.image_space_file();
                        bart_pics(kspace,&image_space,&settings.project_settings.recon_settings);
                        self.image_data = Some(image_space);
                        self.state = Filtering;
                        StateAdvance::Succeeded
                    }
                    None => {
                        //self.state = FormattingKspace;
                        println!("kspace data is not available to reconstruct!");
                        StateAdvance::TerminalFailure
                    }
                }
            }
            Filtering => {
                match &self.image_data {
                    Some(image) => {
                        let w1 = settings.project_settings.recon_settings.fermi_filter_w1;
                        let w2 = settings.project_settings.recon_settings.fermi_filter_w2;
                        cfl::fermi_filter_image(&image,&image,w1,w2);
                        self.state = Scaling;
                        StateAdvance::Succeeded
                    }
                    None => {
                        println!("image data is not set. Cannot continue!");
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
                let scale_histo = settings.project_settings.recon_settings.image_scale_hist_percent as f64;
                match settings.vm_settings.is_scale_setter {
                    true => {
                        write_u16_scale(&self.image_data.clone().unwrap(),scale_histo, &self.scale_file());
                        let scale = ImageScale::from_file(&self.scale_file());
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
                        match settings.vm_settings.is_scale_dependent {
                            false => {
                                let scale = cfl::find_u16_scale(image,scale_histo);
                                self.image_scale = Some(scale);
                                self.state = WritingImageData;
                                StateAdvance::Succeeded
                            }
                            true => {
                                match &self.scale_file().exists() {
                                    true => {
                                        let scale = ImageScale::from_file(&self.scale_file());
                                        self.image_scale = Some(scale.scale_factor);
                                        self.state = WritingImageData;
                                        StateAdvance::Succeeded
                                    }
                                    false => {
                                        // schedule to run again later
                                        println!("scale file not found yet. Expecting it to be {:?}",self.scale_file());
                                        match settings.is_slurm_disabled() {
                                            false => StateAdvance::TryingAgainLater,
                                            true => StateAdvance::TerminalFailure
                                        }
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
                let image_dir = self.image_dir();
                let name = self.name();
                match self.image_scale {
                    Some(scale) => {
                        match image_dir.exists() {
                            false => create_dir_all(&image_dir).expect(&format!("cannot create dir: {:?}",image_dir)),
                            true => {
                                std::fs::remove_dir_all(&self.image_dir()).expect(&format!("cannot clean up image directory {:?}",image_dir));
                                create_dir_all(&self.image_dir()).expect(&format!("cannot create dir: {:?}",image_dir));
                            }
                        }
                        match &self.image_data {
                            Some(image) => {
                                let code = &settings.project_settings.scanner_settings.image_code;
                                let tag = &settings.project_settings.scanner_settings.image_tag;
                                let raw_prefix = format!("{}{}",code,tag);
                                let axis_inversion = (true,true,true); // flip all axes
                                cfl::to_civm_raw_u16(&image, &image_dir, &name, &raw_prefix, scale, axis_inversion);
                            }
                            None => {
                                panic!("where did the image data go!?")
                            }
                        }
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
                let recon_headfile = ReconHeadfile{
                    spec_id: settings.run_settings.spec_id.clone(),
                    civmid: settings.run_settings.civm_id.clone(),
                    project_code:settings.project_settings.project_code.clone(),
                    dti_vols: settings.project_settings.dti_vols.clone(),
                    scanner_vendor:settings.project_settings.scanner_settings.scanner_vendor.clone(),
                    run_number:settings.run_settings.run_number.clone(),
                    m_number:settings.m_number(),
                    image_code:settings.project_settings.scanner_settings.image_code.clone(),
                    image_tag:settings.project_settings.scanner_settings.image_tag.clone(),
                    engine_work_dir: settings.vm_settings.engine_work_dir.clone(),
                    more_archive_info: settings.project_settings.archive_info.clone()
                };
                let image_dir = self.image_dir();

                let headfile_name = image_dir.join(self.name()).with_extension("headfile");
                match &self.resources.clone().unwrap().meta {
                    Some(meta) => {
                        std::fs::copy(&meta, &meta.with_file_name("temp")).expect("cannot copy headfile to temp");
                        Headfile::open(&meta.with_file_name("temp")).append(&recon_headfile.to_hash());
                        std::fs::rename(&meta.with_file_name("temp"),&headfile_name).expect("cannot move headfile");
                    }
                    None => {
                        let h = Headfile::new(&headfile_name);
                        h.append(&recon_headfile.to_hash());
                    }
                }
                self.state = SendingToArchiveEngine;
                StateAdvance::Succeeded
            }

            SendingToArchiveEngine => {
                println!("sending to archive engine ...");
                // connect to system and make the volume directory
                let mut mkdir = Command::new("ssh");
                let u = settings.project_settings.archive_engine_settings.user();
                let h = settings.project_settings.archive_engine_settings.hostname();
                let p = settings.project_settings.archive_engine_settings.base_dir();
                let runno_dir = p.join(self.name());
                let runno_dir = runno_dir.to_str().unwrap();
                mkdir.arg(format!("{}@{}",u,h));
                mkdir.arg(format!("mkdir -p {}",runno_dir));

                let mut scp = Command::new("scp");
                scp.arg("-r");
                scp.arg(self.image_dir().to_str().unwrap());
                scp.arg(&format!("{}@{}:{}",u,h,runno_dir));

                let mkdir_o = mkdir.output().expect("cannot launch ssh");
                match mkdir_o.status.success() {
                    true => {
                        let o = scp.output().expect("failed to launch scp");
                        match o.status.success() {
                            true => {
                                println!("scp successful");
                                self.state = VolumeManagerState::Done;
                                StateAdvance::Succeeded
                            }
                            false => {
                                println!("unable to transfer files with {:?}",scp);
                                StateAdvance::TerminalFailure
                            }
                        }
                    }
                    false => {
                        println!("cannot create directory on archive engine to transfer images. The command was {:?}",mkdir);
                        StateAdvance::TerminalFailure
                    }
                }
            }
            Done => {
                println!("all work is complete.");
                StateAdvance::AllWorkDone
            }
            _=> {
                panic!("not yet implemented")
            }
        }
    }
}