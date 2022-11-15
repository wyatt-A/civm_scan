//use crate::{cfl, resource::*, utils};
//use crate::volume_index::VolumeIndex;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use serde::{Deserialize, Serialize};
use std::fs::{File,create_dir_all};
use std::io::{Read,Write};
use std::path::{Path, PathBuf};
use whoami;
use serde_json;
use crate::bart_wrapper::{bart_pics, BartPicsSettings};
//use crate::volume_manager::{VmState,VolumeManager,launch_volume_manager,launch_volume_manager_job,re_launch_volume_manager_job};
use crate::slurm::{self,BatchScript, JobState};
use std::process::Command;
use seq_lib::pulse_sequence::MrdToKspaceParams;
//use crate::config::{ProjectSettings, Recon};
use mr_data::mrd::{fse_raw_to_cfl, cs_mrd_to_kspace};
use headfile::headfile::{ReconHeadfile, Headfile};
use acquire::build::{HEADFILE_NAME,HEADFILE_EXT};
use crate::{cfl, utils};
use glob::glob;
use clap::Parser;
use crate::cfl::write_u16_scale;

/*
    headfile=mrs_meta_data(mrd);
    headfile.dti_vols = n_volumes;
    headfile.U_code = project_code;
    headfile.U_civmid = civm_userid;
    headfile.U_specid = specimen_id;
    headfile.scanner_vendor = scanner_vendor;
    headfile.U_runno = strcat(run_number,'_',mnum);
    headfile.dim_X = vol_size(1);
    headfile.dim_Y = vol_size(2);
    headfile.dim_Z = vol_size(3);
    headfile.civm_image_code = 't9';
    headfile.civm_image_source_tag = 'imx';
    headfile.engine_work_directory = pwd;
*/

// pub fn main_test_cluster(){
//     return;
//     //let mut bart_settings = BartPicsSettings::quick();
//     //bart_settings.set_bart_binary("bart");
//     //let bart_settings_file = "/privateShares/wa41/cs_recon_test/reco_settings";
//     //let scanner = Host::new("mrs","stejskal");
//     let ptab = "/home/wa41/cs_recon_test/stream_CS256_8x_pa18_pb54";
//     let vpath = "/d/smis/recon_test_data/_01_46_3b0/volume_index.txt";
//     //let mrd_meta_suffix = "_meta.txt";
//
//     let mut recon = Recon::new("grumpy","testrunno0001",vpath,"5xfad","dummyspec");
//
//     let cwd = recon.engine_work_dir.join(format!("{}.work",&recon.run_number));
//     if !cwd.exists(){ create_dir_all(&cwd).expect("unable to create specified working directory")}
//
//     let volman_jobs_file = cwd.join("volume-manager-jobs").with_extension("toml");
//
//     let mrd_vol_offset = 0;
//
//     //bart_settings.to_file(bart_settings_file);
//
//     let raw_base_path = Path::new(vpath).parent().unwrap();
//     let local_raw_path = Path::new(&cwd).join("raw");
//     if !local_raw_path.exists(){create_dir_all(&local_raw_path).expect("issue creating directory");}
//
//     let local_vpath = VolumeIndex::fetch_from(vpath,&recon.scanner.host(),cwd.to_str().unwrap());
//     let ready_mrds = VolumeIndex::read_ready(&local_vpath);
//     let all_mrds = VolumeIndex::read_all(&local_vpath);
//
//     recon.n_volumes = Some(all_mrds.len());
//
//     let mut r = ResourceList::open(local_raw_path.to_str().unwrap());
//     r.set_host(&recon.scanner.host());
//     ready_mrds.iter().for_each(|(mrd,_)| {
//         let mrdname = Path::new(mrd).file_stem().unwrap().to_str().unwrap();
//         let mrd_srcpath = Path::new(raw_base_path).join(mrd);
//         let meta_srcpath = Path::new(raw_base_path).join(format!("{}{}",mrdname,&recon.scanner.vol_meta_suffix));
//         r.try_add(Resource::new(mrd_srcpath.to_str().unwrap(),""));
//         r.try_add(Resource::new(meta_srcpath.to_str().unwrap(),""));
//     });
//     r.start_transfer();
//
//     /*
//         This builds a hashmap of volume managers and their slurm
//         job ids that will updated and saved every time this runs
//     */
//     let mut vol_man_jobs:HashMap<PathBuf,u32>;
//     println!("looking for {:?} ...",volman_jobs_file);
//     if volman_jobs_file.exists(){
//         println!("loading jobs ...");
//         let s = utils::read_to_string(volman_jobs_file.to_str().unwrap(),"toml").expect("cannot open file");
//         vol_man_jobs = toml::from_str(&s).expect("cannot deserialize hash");
//     }
//     else{
//         println!("creating new jobs file ...");
//         vol_man_jobs = HashMap::<PathBuf,u32>::new();
//     }
//     println!("{:?}",vol_man_jobs);
//
//     /*
//         We are assuming a one-to-one mapping a mrd file to a volume manager
//         in "volume_index" mode. If a mrd file is available, and a volume manager
//         hasn't already been launched, a new volume manager will be instantiated
//     */
//     all_mrds.iter().for_each(|(index,mrd)| {
//         let voldir = cwd.join(index);
//         if !voldir.exists(){create_dir_all(&voldir).expect("issue creating directory");}
//         if !VolumeManager::exists(voldir.to_str().unwrap()) && mrd.is_some(){
//             println!("vol man doesn't exist and mrd is available... submitting new job");
//             let mrd_path = local_raw_path.join(mrd.clone().unwrap());
//             let job_id = launch_volume_manager_job(voldir.to_str().unwrap(),mrd_path.to_str().unwrap(),&ptab,mrd_vol_offset,&recon.path());
//             vol_man_jobs.insert(voldir.clone(),job_id);
//         }
//     });
//
//     /*
//         For every volume manager that has been launched, we find the job state
//         from slurm. Note that this is not the volume managers state. This just tells
//         us the state of the slurm job (pending,running,completed,failed ... ect)
//     */
//     let mut job_states = HashMap::<PathBuf,slurm::JobState>::new();
//     vol_man_jobs.iter().for_each(|(vol,job)|{
//         let jstate = slurm::get_job_state(*job,60);
//         job_states.insert(vol.clone(),jstate.clone());
//     });
//
//     /*
//         If for some reason a volume manager cannot advance state (commonly because it is waiting for
//         image scaling information from volume 00), it will return and the slurm state will say "completed."
//         In this case, we need to check for inactivity of volume managers that still have work to do. If this is
//         the case, we need to restart them, returning a new slurm job id to track
//     */
//     job_states.iter().for_each(|(vol,state)|{
//         if *state == JobState::Completed && !VolumeManager::is_done(vol.to_str().unwrap()){
//             //println!("restarting {:?}",vol);
//             let workdir = vol.to_str().unwrap();
//             let job_id = re_launch_volume_manager_job(workdir);
//             vol_man_jobs.insert(vol.clone(),job_id);
//         }
//     });
//
//     /*
//         Here we need to build up some info about the overall progress of the system for reporting and as a
//         stop condition for rescheduling.
//     */
//
//     //let mc = all_mrds.clone();
//     let mut m:Vec<&String> = all_mrds.keys().collect();
//     m.sort();
//     //println!("sorted idx: {:?}",m);
//
//     let mut state_str = String::new();
//     let mut n_completed:usize = 0;
//     let states:Vec<VmState> = m.iter().map(|index| {
//         let voldir = cwd.join(index);
//         let s = VolumeManager::state(voldir.to_str().unwrap());
//         if s == VmState::Done {n_completed += 1};
//         let slurm_state = job_states.get(&voldir);
//         match slurm_state {
//             Some(state) => state_str.push_str(&format!("{} : slurm job : {:?}; volume-manager : {:?}\t\n",index,state,&s)),
//             None => state_str.push_str(&format!("{} : slurm job : not submitted; volume-manager : {:?}\t\n",index,&s))
//         }
//         return s;
//     }).collect();
//
//     println!("{}",state_str);
//     println!("{} completed out of {}.",n_completed,m.len());
//     /*
//         Here we save information we want to load up the next time this code runs. Right now, this only has
//         to be the slurm job ids of the volume managers
//     */
//     let vol_man_jobs_str = toml::to_string(&vol_man_jobs).expect("cannot serialize hash");
//     utils::write_to_file(volman_jobs_file.to_str().unwrap(),"toml",&vol_man_jobs_str);
//
//     // if all work isn't done, schedule this code to run again later (2 minutes seems good?)
//     // if n_vols != n_complete{
//     //     /* reschedule for later */
//     // }
//
// }


pub const SCALE_FILENAME:&str = "volume_scale_info";
pub const DEFAULT_HIST_PERCENT:f32 = 0.9995;

pub fn test_updated() {
    let work_dir = Path::new("/privateShares/wa41/N60tacos.work/m00");
    let vmc_file = work_dir.join("vm_config");

    let mut vmc  = VolumeManagerConfig::default(work_dir);
    vmc.remote_host = Some("stejskal".to_string());
    vmc.remote_user = Some("mrs".to_string());
    vmc.resource_dir = Some(PathBuf::from("/d/dev/221111/acq/m00"));
    vmc.to_file(&work_dir.join("vm_config"));

    let vma = VolumeManagerArgs::new(work_dir,&vmc_file);

    VolumeManager::launch(vma);


}

// #[derive(Debug,clap::Subcommand)]
// pub enum Action {
//     Launch(VolumeManagerArgs),
// }
//
//
// #[derive(clap::Parser,Debug)]
// pub struct VolumeManagerCmd {
//     #[command(subcommand)]
//     pub action: Action,
// }
//
// #[derive(Debug,clap::Parser)]
// pub struct RelaunchArgs {
//     work_dir: PathBuf
// }


#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct VolumeManagerArgs {
    work_dir:PathBuf,
    config:PathBuf
}

impl VolumeManagerArgs {
    pub fn new(work_dir:&Path,config_file:&Path) -> Self {
        Self {
            work_dir:work_dir.to_owned(),
            config:config_file.to_owned()
        }
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct VolumeManagerConfig {
    resource_dir:Option<PathBuf>,
    remote_user:Option<String>,
    remote_host:Option<String>,
    recon_headfile:Option<ReconHeadfile>,
    recon_settings:Option<BartPicsSettings>,
    scale_dependent:Option<bool>,
    scale_setter:Option<bool>,
    scale_hist_percent:Option<f32>,
}

impl VolumeManagerConfig {
    pub fn from_file(work_dir:&Path) -> Self {
        let mut f = File::open(Self::file_name(work_dir)).expect("file not found");
        let mut s = String::new();
        f.read_to_string(&mut s).expect("cannot read file");
        serde_json::from_str(&s).expect("cannot deserialize args")
    }

    pub fn to_file(&self,work_dir:&Path) {
        let fname = Self::file_name(work_dir);
        let mut f = File::create(fname).expect("cannot create file");
        let s = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        f.write_all(s.as_bytes()).expect("cannot write to file");
    }

    pub fn default(work_dir:&Path) -> Self {
        Self {
            resource_dir: None,
            remote_user: None,
            remote_host: None,
            recon_headfile: Some(ReconHeadfile::default()),
            recon_settings: Some(BartPicsSettings::default()),
            scale_dependent: None,
            scale_setter: None,
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

    pub fn to_file(&self) {
        let filename = Self::file_name(&self.work_dir);
        let s = serde_json::to_string_pretty(&self).unwrap();
        let mut f = File::create(filename).unwrap();
        f.write_all(s.as_bytes()).unwrap();
    }

    pub fn from_file(config_file:&Path) -> Self {
        let mut f = File::open(config_file).unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        serde_json::from_str(&s).unwrap()
    }
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

#[derive(Clone,Serialize,Deserialize)]
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


impl VolumeManagerResources {

    // pub fn find_cs_table(work_dir:&Path) -> Option<PathBuf> {
    //     get_first_match(work_dir,"cs_table")
    // }

    pub fn from_dir(work_dir:&Path) -> Result<Self,ResourceError> {
        let work_dir = &Self::resource_dir(work_dir);
        let cs_table = get_first_match(work_dir,"*cs_table").ok_or(ResourceError::CsTableNotFound)?;
        let raw_mrd = get_first_match(work_dir,"*.mrd").ok_or(ResourceError::MrdNotFound)?;
        let acq_complete = get_first_match(work_dir,"*.ac").ok_or(ResourceError::MrdNotComplete)?;
        let kspace_config = get_first_match(work_dir,".mtk").ok_or(ResourceError::KspaceConfigNotFound)?;
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
        work_dir.join("resources")
    }

    pub fn exist(work_dir:&Path) -> bool {
        Self::from_dir(work_dir).is_ok()
    }
}



#[derive(Serialize,Deserialize)]
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

#[derive(Serialize,Deserialize)]
pub struct ImageScale {
    histogram_percent:f32,
    scale_factor:f32
}

impl ImageScale {
    pub fn new(histogram_percent:f32,scale_factor:f32) -> Self {
        Self {
            histogram_percent,
            scale_factor
        }
    }
    pub fn from_file(file_path:&Path) -> Self {
        let mut f = File::open(file_path).expect("cannot open file");
        let mut s = String::new();
        f.read_to_string(&mut s).expect("cannot read from file");
        serde_json::from_str(&s).expect("cannot deserialize file")
    }
    pub fn to_file(&self,file_path:&Path) {
        let s = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        let mut f = File::create(file_path).expect("cannot create file");
        f.write_all(s.as_bytes()).expect("cannot write to file");
    }
}


impl VolumeManager {


    fn image_vol(&self) -> PathBuf {
        self.args.work_dir.join("image_vol")
    }

    fn kspace_vol_name(&self) -> PathBuf {
        self.args.work_dir.join("kspace_vol")
    }

    fn file_name(work_dir:&Path) -> PathBuf {
        work_dir.join("volume_manager")
    }

    pub fn to_file(&self) {
        let mut f = File::create(Self::file_name(&self.args.work_dir)).expect("cannot create file");
        let s = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        f.write_all(s.as_bytes()).expect("cannot write to file");
    }

    pub fn from_file(work_dir:&Path) -> Option<Self> {
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

    fn open(vma:&VolumeManagerArgs) -> Self {
        match Self::from_file(&vma.work_dir) {
            Some(vm) => vm,
            None => VolumeManager {
                args: vma.clone(),
                config: VolumeManagerConfig {},
                resources: None,
                kspace_data: None,
                image_data: None,
                image_output: None,
                image_scale: None,
                state:VolumeManagerState::Idle
            }
        }
    }


    fn advance_state(&mut self) {
        use VolumeManagerState::*;
        match &self.state {
            Idle | NeedsResources(_) => {
                // sync resources here
                match VolumeManagerResources::from_dir(&self.args.work_dir) {
                    Ok(resources) => {
                        self.resources = Some(resources);
                        self.state = FormattingKspace;
                    },
                    Err(e) => {
                        self.state = NeedsResources(e);
                        // schedule to run again later
                    }
                }
            }
            FormattingKspace => {
                match &self.resources {
                    Some(res) => {
                        let mtk = MrdToKspaceParams::from_file(&res.kspace_config);
                        cs_mrd_to_kspace(&res.raw_mrd,&res.cs_table,&self.kspace_vol_name(),&mtk);
                        self.kspace_data = Some(self.kspace_vol_name());
                        self.state = Reconstructing;
                    }
                    None => {
                        //self.state = NeedsResources(ResourceError::Unknown);
                        panic!("raw data is not available to format!")
                    }
                }
            }
            Reconstructing => {
                match &self.kspace_data {
                    Some(kspace) => {
                        let mut recon_settings = BartPicsSettings::default();
                        recon_settings.max_iter = 2;
                        bart_pics(kspace,&self.image_vol(),&mut recon_settings);
                        self.image_data = Some(self.image_vol());
                        self.state = Scaling;
                    }
                    None => {
                        //self.state = FormattingKspace;
                        panic!("kspace data is not available to reconstruct!")
                    }
                }
            }
            Scaling => {
                match &self.image_data {
                    Some(image) => {
                        match self.args.scale_dependent.unwrap_or(false) {
                            false => {
                                let scale = cfl::find_u16_scale(image,self.args.scale_hist_percent.unwrap_or(0.9995) as f64);
                                self.image_scale = Some(scale);
                                self.state = WritingImageData;
                            }
                            true => {
                                match &self.resources.expect("resources not found! How did we get here!?").scaling_info {
                                    Some(scale_file) => {
                                        let scale = ImageScale::from_file(scale_file);
                                        self.image_scale = Some(scale.scale_factor);
                                        self.state = WritingImageData;
                                    }
                                    None => {
                                        // schedule to run again later
                                    }
                                }
                            }
                        }
                        if self.args.scale_setter.unwrap_or(false) {
                            let scale_file = self.args.work_dir.parent().expect("path has no parent").join(SCALE_FILENAME);
                            write_u16_scale(image,self.args.scale_hist_percent.unwrap_or(0.9995) as f64,&scale_file);
                            let scale = ImageScale::from_file(&scale_file);
                            self.image_scale = Some(scale.scale_factor);
                            self.state = WritingImageData;
                        }
                    },
                    None => {
                        //self.state = Reconstructing;
                        panic!("image data is not available to scale!")
                    }
                }
            }
            WritingImageData => {
                match self.image_scale {
                    Some(scale) => {
                        let image_dir = self.args.work_dir.join("images");
                        let image = self.image_data.expect("where did the image data go!?");
                        let image_code = "t9".to_string();
                        let image_tag = "imx".to_string();
                        let raw_prefix = format!("{}{}",image_code,image_tag);
                        let vname = self.args.work_dir.file_name().unwrap().to_str().unwrap();
                        cfl::to_civm_raw_u16(&image,&image_dir,vname,&raw_prefix,scale);
                        // write headfile if possible
                    }
                    //None => self.state = Scaling;
                    panic!("image scale is undetermined!");
                }
            }
            _=> {}
        }
    }


    pub fn launch(vma:VolumeManagerArgs) {

        let config = VolumeManagerConfig::from_file(&vma.config);

        let vm = Self::open(&vma);

        // create work dir
        if !vma.work_dir.exists() {
            println!("creating new volume manager working directory at {:?}",vma.work_dir);
            create_dir_all(&vma.work_dir).expect("failed to create working directory");
        }
        vm.to_file();



        println!("{:?}",vma);

        let vmr = VolumeManagerResources::from_dir(&vma.work_dir);


        match &vmr {
            Ok(res) => {
                //find state and advance
            }
            Err(e) => {
                println!("resources aren't available");
                // schedule to run later
            }
        }

    }




    // pub fn launch(local_base_dir:&Path,remote_base_dir:&Path,params:&VolumeManagerParams) {
    //
    //     let vm_filename = local_base_dir.join(&params.vol_name).with_file_name("volume_manager_state").with_extension("json");
    //     let vm = match vm_filename.exists() {
    //         true => {
    //             println!("resuming volume manager");
    //             let mut f = File::open(&vm_filename).expect("cannot open file");
    //             let mut json_txt = String::new();
    //             f.read_to_string(&mut json_txt).expect("cannot read file");
    //             serde_json::from_str(&json_txt).expect("cannot deserialize volume manager")
    //         }
    //         false => {
    //             println!("launching new volume manager");
    //             Self {
    //                 local_dir: local_base_dir.join(&params.vol_name),
    //                 label: params.vol_name.clone(),
    //                 params: params.clone()
    //             }
    //         }
    //     };
    //
    //     let state_txt = serde_json::to_string_pretty(&vm).expect("cannot serialize volume manager");
    //
    //     let remote_dir = remote_base_dir.join(&params.vol_name).to_owned();
    //
    //     let work_dir_name = &format!("{}.work",&params.run_number);
    //     let local_dir = local_base_dir.join(work_dir_name).join(&params.vol_name);
    //
    //     let rparams = ReconHeadfileParams {
    //         dti_vols: Some(params.n_dti_vols),
    //         project_code: params.project_code.to_string(),
    //         civm_id: params.civm_id.to_string(),
    //         spec_id: params.specimen_id.to_string(),
    //         scanner_vendor: params.scanner_vendor.to_string(),
    //         run_number: params.run_number.to_string(),
    //         m_number: params.m_number.to_string(),
    //         image_code: "t9".to_string(),
    //         image_tag: "imx".to_string(),
    //         engine_work_dir: local_base_dir.to_owned(),
    //     };
    //     let vol_name = rparams.m_number.clone();
    //     let vmr = VolumeManagerResources::from_dir(&local_dir,&vol_name);
    //     let mut puller_cmd = Command::new("puller_simple");
    //     puller_cmd.args(
    //         vec![
    //             "-oer",
    //             &params.remote_host,
    //             remote_dir.to_str().unwrap(),
    //             local_dir.to_str().unwrap(),
    //         ]
    //     );
    //     if !vmr.exist() {
    //         println!("fetching data ...");
    //         let o = puller_cmd.output().expect("puller_simple failed to launch");
    //         match o.status.success() {
    //             true => {}
    //             false => {
    //                 println!("failed to transfer directory. Removing residual files ...");
    //                 //todo!(delete any leftover files)
    //                 println!("puller_simple command: {:?}", puller_cmd);
    //             }
    //         }
    //     }
    //     if !vmr.exist() {
    //         panic!("puller simple is having issues");
    //     }
    //
    //
    //     println!("formatting kspace ...");
    //     let mtk = MrdToKspaceParams::from_file(&vmr.kspace_config);
    //     let kspace_cfl = vmr.raw_mrd.with_extension("");
    //     let image = kspace_cfl.with_file_name("image");
    //     cs_mrd_to_kspace(&vmr.raw_mrd,&vmr.cs_table,&kspace_cfl,&mtk);
    //     let mut bart_settings = params.bart_settings.clone();
    //
    //     println!("reconstructing ...");
    //     bart_pics(kspace_cfl.to_str().unwrap(),image.to_str().unwrap(),&mut bart_settings);
    //
    //     println!("scaling image ...");
    //     let scale = cfl::find_u16_scale(&image,0.9995);
    //     let img_dir = local_dir.join("images");
    //     let vname = format!("{}_{}",&params.run_number,vol_name);
    //     create_dir_all(&img_dir).expect("cannot create directory");
    //     let raw_prefix = format!("{}{}",rparams.image_code,rparams.image_tag);
    //
    //     println!("writing volume slices ...");
    //     cfl::to_civm_raw_u16(&image,&img_dir,&vname,&raw_prefix,scale);
    //
    //     println!("updating headfile ...");
    //     std::fs::copy(&vmr.meta, &vmr.meta.with_file_name("temp")).expect("cannot copy headfile to temp");
    //     Headfile::open(&vmr.meta.with_file_name("temp")).append(&rparams.to_hash());
    //     std::fs::rename(&vmr.meta.with_file_name("temp"), img_dir.with_file_name(vmr.meta.file_name().unwrap())).expect("cannot move headfile");
    // }
}