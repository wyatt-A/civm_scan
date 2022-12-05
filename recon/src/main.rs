use std::fs::{create_dir, create_dir_all};
use std::io::{stdin, stdout, Write};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use recon::{recon_config, slurm};
use recon::recon_config::{Config, ConfigFile, ProjectSettings, RemoteSystem, VolumeManagerConfig};
use recon::slurm::{BatchScript, get_job_state};
use recon::vol_manager::{VolumeManager, VolumeManagerState};
use utils::m_number_formatter;

#[derive(clap::Parser,Debug)]
pub struct ReconArgs {
    #[command(subcommand)]
    pub action: ReconAction,
}

#[derive(clap::Subcommand,Debug)]
pub enum ReconAction {
    /// reconstruct a diffusion-weighted series of volumes
    New(NewRecon),
    /// check the status of a reconstruction by run number
    Status(RunnoArgs),
    /// restart a recon by run number
    Restart(RestartArgs),
    /// cancel jobs associated with a run number
    Cancel(RunnoArgs),
    /// wait for this run to complete before returning
    WaitForCompletion(WaitForCompletionArgs),
    /// create a new project template to modify for a new protocol
    NewProjectTemplate(TemplateConfigArgs),
    /// interact with a single volume manager
    VolumeManager(VolumeManagerCmd),
}

#[derive(clap::Args,Debug)]
pub struct VolumeManagerCmd {
    #[command(subcommand)]
    action:VolumeMangerAction,
}

#[derive(Clone,clap::Subcommand,Debug)]
pub enum VolumeMangerAction {
    Launch(VolumeMangerLaunchArgs),
    NewConfig(NewConfigArgs),
}

#[derive(Clone,clap::Args,Debug)]
pub struct NewConfigArgs {
    template_config:PathBuf,
    run_number:String,
    specimen_id:String,
    volume_name:String,
    output_config:PathBuf,
    is_scale_setter:bool,
    is_scale_dependent:bool
}

#[derive(Clone,clap::Args,Debug)]
pub struct RestartArgs {
    run_number:String,
    #[clap(long)]
    disable_slurm:Option<bool>,
    /// set the state of each volume manager on re-launch
    #[clap(long)]
    forced_state:Option<String>,
    #[clap(long,short)]
    send_to_engine:Option<bool>,
}

#[derive(Clone,clap::Args,Debug)]
pub struct SetStateArgs {
    run_number:String,
    state:String,
}

#[derive(Clone,clap::Args,Debug)]
pub struct WaitForCompletionArgs {
    run_number:String,
    /// time between completion checks in minutes
    #[clap(long)]
    refresh_period:Option<f32>,
}

#[derive(Clone,clap::Args,Debug)]
pub struct RunnoArgs {
    run_number:String,
}

#[derive(Clone,clap::Args,Debug)]
pub struct NewRecon {
    /// recon type (dti,single-volume,multi-echo)
    recon_type:String,
    /// civm id (cof,wa41,kjh ...)
    civm_id:String,
    /// base configuration used to define recon parameters
    project_settings:PathBuf,
    /// run number for the set of DTI volumes
    run_number:String,
    /// base path for the raw data. This is an absolute path on the scanner.
    raw_data_base_dir:PathBuf,
    /// civm specimen id
    specimen_id:String,
    /// index of the volume used for scaling. This is typically the first volume (defaults to 0)
    #[clap(long)]
    scaling_volume:Option<u32>,
    /// run without slurm scheduling. This will reconstruct each volume serially in your terminal
    #[clap(long)]
    disable_slurm:Option<bool>,
    /// enable this option if you want to skip the meta data check for archival
    #[clap(long)]
    no_archive:Option<bool>,
    /// define the index of the last volume to reconstruct. If you only want to reconstruct the first volume, use 0
    #[clap(long)]
    last_volume:Option<usize>,
    /// define the index of the first volume to reconstruct. The first volume has index 0
    #[clap(long)]
    first_volume:Option<usize>,
    /// if you don't want to be reminded of the recon parameters and be asked if they are correct, enable this
    #[clap(long,short)]
    batch_mode:Option<bool>,
    /// supply an email to get a notification when the recon is done
    #[clap(long,short)]
    email:Option<String>,
    /// set this to false to disable sending data to archive engine
    #[clap(long,short)]
    send_to_engine:Option<bool>,
}

#[derive(Clone,clap::Args,Debug)]
pub struct TemplateConfigArgs {
    /// absolute path to the new config, or just a file name to save to default location.
    /// you're file extension will not be respected.
    output_config:PathBuf,
}

#[derive(Clone,clap::Args,Debug)]
pub struct VolumeMangerLaunchArgs {
    /// path to a volume manager configuration file. This path will be the working directory
    /// of the volume manager
    config_file:PathBuf
}

fn main() {
    let args = ReconArgs::parse();
    match args.action {
        ReconAction::VolumeManager(vm_cmd) => {
            match vm_cmd.action {
                VolumeMangerAction::Launch(launch_cmd) => {
                    VolumeManager::launch(&launch_cmd.config_file)
                }
                _=> println!("not yet implemented!")
            }
        }
        ReconAction::NewProjectTemplate(args) => {
            ProjectSettings::default().to_file(&args.output_config)
        }
        ReconAction::Restart(args) => restart(args),
        ReconAction::Status(args) => status(args),
        ReconAction::New(args) => recon(args),
        ReconAction::Cancel(args) => cancel(args),
        ReconAction::WaitForCompletion(args) => wait_for_completion(args),
    }
}


/*
    Do stuff to a collection of volumes managers. This may launching, relaunching, configurations ...
 */

#[derive(Clone,Debug)]
pub enum VMCollectionType {
    /// volume managers are operating on a collection of mr raw files leading to a collection
    /// of volume managers
    Dti,
    /// volume managers are operating on a single mr raw file from the scanner, but require multiple volumes
    /// to be reconstructed
    MultiEcho,
    /// a single volume is to be reconstructed
    Single,
}

impl VMCollectionType {
    pub fn decode(type_str:&str) -> Self {
        match type_str{
            "Dti" | "dti" | "DTI" => Self::Dti,
            "multi-echo" | "multiecho" | "MULTIECHO" => Self::MultiEcho,
            "single-volume" | "single" | "Single" | "SINGLE" => Self::Single,
            _=> panic!("recon type {} not recognized. Recognized types are \n{}\n",type_str,
                format!("{}\n{}\n{}",
                    "dti",
                    "single",
                    "multi-echo"
                )
            )
        }
    }
}


pub struct VolumeManagerCollection {
    work_dir:PathBuf,
    vm_config_files:Vec<PathBuf>
}

impl VolumeManagerCollection {
    pub fn new(collection_type:VMCollectionType,settings:&NewRecon,work_dir:&Path) -> Self {
        use VMCollectionType::*;
        let cfgs = match &collection_type {
            // handle initial setup
            Dti => VolumeManagerConfig::new_dti_config(
                    &settings.project_settings,
                    &settings.civm_id,
                    &settings.run_number,
                    &settings.specimen_id,
                    &settings.raw_data_base_dir
                ),
            Single => VolumeManagerConfig::new_single_volume(
                &settings.project_settings,
                &settings.civm_id,
                &settings.run_number,
                &settings.specimen_id,
                &settings.raw_data_base_dir
            ),
            _=> panic!("{:?} not yet implemented for volume manager collection!",collection_type)
        };

        // trim configs down if a subset is specified
        let user_last_vol = settings.last_volume.unwrap_or(cfgs.len()-1);
        let upper_index = if user_last_vol >= cfgs.len() {cfgs.len()-1} else {user_last_vol};
        let user_first_vol = settings.first_volume.unwrap_or(0);
        let lower_index = if user_first_vol >= cfgs.len() {cfgs.len()-1} else {user_first_vol};
        let mut cfgs = Vec::from_iter(cfgs[lower_index..(upper_index +1)].iter().cloned());
        //println!("launching recon for range {} to {}. {} volumes will be launched for reconstruction", lower_index, upper_index, upper_index - lower_index +1);
        Self::_apply_settings_new(&mut cfgs,settings,work_dir);
        let cfg_files = Self::to_disk(&cfgs,work_dir);
        Self {
            work_dir:work_dir.to_owned(),
            vm_config_files:cfg_files
        }
    }

    pub fn from_work_dir(work_dir:&Path) -> Option<Self> {
        let cfg_files = utils::find_files(work_dir,"volman_config")?;
        Some(Self{
            work_dir: work_dir.to_owned(),
            vm_config_files: cfg_files,
        })
    }

    pub fn launch(&self) {
        self.vm_config_files.iter().for_each(|file|{
            let cfg = VolumeManagerConfig::from_file(file);
            match cfg.is_slurm_disabled() {
                true => {
                    println!("launching volume manager without slurm {:?}",file);
                    VolumeManager::launch(file);
                },
                false => {
                    let jid = VolumeManager::launch_with_slurm_now(file);
                    println!("{} job submitted with id {}",cfg.name(),jid);
                }
            }
        });
    }

    pub fn cancel(&self) {
        let mut n_cancelled = 0;
        self.vm_config_files.iter().for_each(|file| {
            match VolumeManager::read(file) {
                Some(vm) => {
                    if vm.cancel_job(){
                        n_cancelled += 1;
                    }
                },
                None => {}
            }
        });
        println!("{} volume managers cancelled",n_cancelled);
    }

    pub fn restart(&self) {
        self.cancel();
        self.launch();
        println!("restarted {} volume managers.",self.vm_config_files.len());
    }

    pub fn force_state(&self,state:&str) {
        self.vm_config_files.iter().for_each(|file|{
            match VolumeManager::read(file){
                Some(mut vm) => {
                    vm.set_state(state);
                    vm.to_file();
                },
                None => {}
            }
        });
    }

    pub fn incomplete(&self) -> Self {
        let incomplete = self.vm_config_files.iter().filter(|file|{
            match VolumeManager::read(file){
                Some(vm) => !vm.is_done(),
                None => true
            }
        }).map(|file| file.to_owned()).collect();
        Self {
            work_dir: self.work_dir.clone(),
            vm_config_files: incomplete,
        }
    }

    fn vm_configs(&self) -> Vec<VolumeManagerConfig>{
        self.vm_config_files.iter().map(|file| VolumeManagerConfig::from_file(file)).collect()
    }

    fn to_disk(cfgs:&Vec<VolumeManagerConfig>,work_dir:&Path) -> Vec<PathBuf> {
        cfgs.iter().map(|cfg|{
            let config_path = work_dir.join(cfg.name());
            create_dir_all(&config_path).expect(&format!("unable to create {:?}",config_path));
            let file = config_path.join(cfg.name());
            cfg.to_file(&file);
            file
        }).collect()
    }

    fn from_disk(&self) -> Vec<VolumeManagerConfig> {
        // find volume managers in work dir
        self.vm_config_files.iter().map(|file| VolumeManagerConfig::from_file(file)).collect()
    }

    fn _apply_settings_new(cfgs:&mut Vec<VolumeManagerConfig>,settings:&NewRecon,work_dir:&Path){
        cfgs.iter_mut().for_each(|cfg|{
            cfg.slurm_disabled = settings.disable_slurm.unwrap_or(false);
            cfg.send_to_engine = settings.send_to_engine.unwrap_or(true);
            cfg.vm_settings.engine_work_dir = work_dir.to_owned();
        });
        Self::to_disk(&cfgs,work_dir);
    }

    pub fn apply_settings_new(&mut self,settings:&NewRecon) {
        let mut cfgs = self.from_disk();
        Self::_apply_settings_new(&mut cfgs,settings,&self.work_dir);
    }

    pub fn apply_settings_restart(&mut self,settings:&RestartArgs){
        let mut cfgs = self.from_disk();
        cfgs.iter_mut().for_each(|cfg|{
            cfg.slurm_disabled = settings.disable_slurm.unwrap_or(false);
            cfg.send_to_engine = settings.send_to_engine.unwrap_or(true);
        });
        Self::to_disk(&cfgs,&self.work_dir);
    }

}






// fn restart(args:RestartArgs){
//     let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS must be set on this workstation");
//     let engine_work_dir = Path::new(&bg);
//     let work_dir = engine_work_dir.join(format!("{}.work",&args.run_number));
//     // find all volume manager config files recursively
//     let config_files = utils::find_files(&work_dir,"volman_config");
//     let mut n_restarted = 0;
//     match config_files {
//         Some(mut files) => {
//             files.sort();
//             for config_file in files.iter() {
//                 let mut c = VolumeManagerConfig::from_file(config_file);
//                 if args.disable_slurm.is_some() { c.slurm_disabled = args.disable_slurm.unwrap(); }
//                 if args.send_to_engine.is_some() { c.send_to_engine = args.send_to_engine.unwrap(); }
//                 c.to_file(config_file);
//                 /*
//                     If we see a volume manager (vm) state file, and we are forcing a particular state,
//                     set the state of the vm and write the state file back to disk. Then, if the volume manager
//                     is not in the "Done" state, cancel the existing job and relaunch it.
//                  */
//                 let mut vm = VolumeManager::read(config_file);
//                 match vm {
//                     Some(mut vm) => {
//                         match &args.forced_state {
//                             Some(state) => {
//                                 vm.set_state(state);
//                                 vm.to_file();
//                             }
//                             _=> {}
//                         }
//                         match vm.state(){
//                             VolumeManagerState::Done => {},
//                             _=> {
//                                 match c.is_slurm_disabled() {
//                                     true => {VolumeManager::launch(config_file);}
//                                     false => {
//                                         vm.cancel_job();
//                                         VolumeManager::launch_with_slurm_now(config_file);
//                                     }
//                                 }
//                                 n_restarted += 1;
//                             }
//                         }
//                     }
//                     _=> {}
//                 }
//             }
//             status(RunnoArgs{
//                 run_number:args.run_number.clone()
//             });
//             println!("restarted {} volume managers.",n_restarted);
//         },
//         None => {
//             println!("no volume manager configs found in {:?}",work_dir);
//         }
//     }
// }


fn restart(args:RestartArgs){
    let work_dir = work_dir_big_disk(&args.run_number);
    let mut vm_collection = VolumeManagerCollection::from_work_dir(&work_dir).expect(&format!("no volume manager configs found in {:?}",work_dir));
    vm_collection.apply_settings_restart(&args);
    vm_collection.incomplete().restart();
}




fn cancel(args:RunnoArgs) {
    println!("finding jobs to cancel for {} ...",args.run_number);
    let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS must be set on this workstation");
    let engine_work_dir = Path::new(&bg);
    let work_dir = engine_work_dir.join(format!("{}.work",&args.run_number));

    if !work_dir.exists(){
        println!("{} not found. {:?} doesn't exist.",args.run_number,work_dir);
        return
    }

    // find all volume manager state files recursively
    let state_files = utils::find_files(&work_dir,"vol_man");

    let mut states = match state_files {
        None => {
            println!("no volume managers found!");
            return
        }
        Some(state_files) => state_files
    };

    states.sort();

    for s in states {
        let vm = VolumeManager::read(&s).unwrap();
        match vm.job_id() {
            Some(jid) => match slurm::cancel(jid){
                true => println!("{} cancelled",vm.name()),
                false => println!("a problem occurred when attempting to cancel {}",vm.name())
            }
            None => {
                println!("no job id found for {}",vm.name())
            }
        }
    }


}


fn work_dir_big_disk(run_number:&str) -> PathBuf {
    let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS must be set on this workstation");
    let engine_work_dir = Path::new(&bg);
    engine_work_dir.join(format!("{}.work",run_number))
}


fn status(args:RunnoArgs) {
    println!("running recon status check on {} ...",args.run_number);
    let work_dir = work_dir_big_disk(&args.run_number);

    if !work_dir.exists(){
        println!("{} not found. {:?} doesn't exist.",args.run_number,work_dir);
        return
    }

    // find all volume manager state files recursively
    let state_files = utils::find_files(&work_dir,"vol_man");


    // match state_files {
    //     Some(mut files) => {
    //         files.sort();
    //         let job_ids:Vec<u32> = files.iter().flat_map(|file|{
    //             let vm = VolumeManager::read(file).unwrap();
    //             vm.job_id()
    //         }).collect();
    //
    //         let job_states = slurm::JobCollection::from_array(&job_ids).state();
    //
    //     }
    //     None => {
    //         println!("no volumes managers found in {:?}",work_dir);
    //     }
    // }


    let mut n_done = 0;
    let mut total = 0;

    match state_files {
        Some(mut files) => {
            files.sort();
            files.iter().for_each(|state_file|{
                let vm = VolumeManager::read(state_file).unwrap();
                let jstate = vm.slurm_status();
                let status = vm.state_string();
                let rep = match jstate {
                    Some(slurm_state) => {
                        println!("{} state:{}    slurm job status:{:?}",vm.name(),status,slurm_state)
                    }
                    None => {
                        println!("{} state:{}    slurm job status:{}",vm.name(),status,"not scheduled")
                    }
                };
                total += 1;
                if vm.is_done(){
                    n_done += 1;
                }
            });
        }
        None => {
            println!("no volumes managers found in {:?}",work_dir);
        }
    }
    println!("{} volume managers have completed of {}",n_done,total);
}


const DEFAULT_TIME_TO_WAIT:f32 = 2.0; //minutes
fn wait_for_completion(args:WaitForCompletionArgs){
    let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS must be set on this workstation");
    let engine_work_dir = Path::new(&bg);
    let work_dir = engine_work_dir.join(format!("{}.work",&args.run_number));

    if !work_dir.exists(){
        panic!("{} not found. {:?} doesn't exist.",args.run_number,work_dir)
    }

    // find all volume manager state files recursively
    let mut state_files = utils::find_files(&work_dir,"vol_man").expect(&format!("no volumes managers found in {:?}", work_dir));
    state_files.sort();

    loop {
        let mut n_done = 0;
        let mut total = 0;
        state_files.iter().for_each(|state_file| {
            let vm = VolumeManager::read(state_file).unwrap();
            if vm.is_done() {
                n_done += 1
            }
            total += 1;
        });

        println!("{}: {} of {} are complete",args.run_number,n_done,total);

        match n_done == total {
            true => break,
            false => std::thread::sleep(Duration::from_secs_f32(args.refresh_period.unwrap_or(DEFAULT_TIME_TO_WAIT)*60.0))
        }
    }
}

fn slurm_recon_watch(args:WaitForCompletionArgs,email:&str) {

    let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS must be set on this workstation");
    let engine_work_dir = Path::new(&bg);
    let work_dir = engine_work_dir.join(format!("{}.work",&args.run_number));

    let refresh_period = args.refresh_period.unwrap_or(DEFAULT_TIME_TO_WAIT);

    let job_name = format!("{}_watcher",args.run_number);

    let this_exe = std::env::current_exe().expect("cannot determine this executable");

    let mut cmd = Command::new(this_exe);
    cmd.arg("wait-for-completion");
    cmd.arg(&format!("--refresh-period={}",refresh_period));
    cmd.arg(&args.run_number);

    let mut b = slurm::BatchScript::new(&job_name,&vec![cmd]);
    b.options.email = Some(String::from(email));
    b.options.memory = Some(String::from("20M"));
    b.options.output = work_dir.join("recon_watcher-%j").with_extension("out").into_os_string().to_str().unwrap().to_string();

    b.submit_now(&work_dir);
}

fn recon(args: NewRecon){
    let vmc_type = VMCollectionType::decode(&args.recon_type);
    let work_dir = work_dir_big_disk(&args.run_number);
    if !preflight_check(&args){
        println!("pre-flight check failed.");
        return
    }
    VolumeManagerCollection::new(vmc_type,&args,&work_dir).launch();
}

// fn _dti(args: NewRecon) {
//     let work_dir = work_dir_big_disk(&args.run_number);
//     if !preflight_check(&args){
//         println!("pre-flight check failed.");
//         return
//     }
//     let vm_collection = VolumeManagerCollection::new(VMCollectionType::Dti,&args,&work_dir);
//     vm_collection.launch();
// }

fn preflight_check(args: &NewRecon) -> bool {
    let p = ProjectSettings::from_file(&args.project_settings);

    if !p.archive_info.is_valid(&p.project_code,&args.civm_id) {
        match args.no_archive.unwrap_or(false) {
            false => {
                println!("project meta data is incorrect for archiving. You must repair the project settings at {:?} and try again.",args.project_settings);
                println!("if you want to run the recon anyway, re-run with the no archive option. Use --help to find it.");
                return false
            }
            true => {
                println!("you have opted into running the recon despite not passing the meta data validation for archiving!");
            }
        }
    }

    if args.send_to_engine.unwrap_or(true) && !p.archive_engine_settings.test_connection(){
        println!("you must fix the remote connection");
        return false
    }

    if !p.scanner_settings.test_connection() {
        println!("you must fix the remote connection");
        return false
    }

    review_settings(args);

    true
}

fn review_settings(args:&NewRecon){
    let p = ProjectSettings::from_file(&args.project_settings);
    let mut user_in = String::new();
    println!("----------------SETTINGS----------------");
    println!("project_file = '{:?}'",args.project_settings);
    println!("specimen_id = '{}'",args.specimen_id);
    println!("{}",p.to_txt());
    println!("----------------------------------------");
    println!("is this configuration correct? Hit enter to continue or control-C to cancel");
    stdin().read_line(&mut user_in).expect("provide an input!");
}

// fn dti(args: NewRecon){
//     // Where are we going to live?
//     let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS must be set on this workstation");
//     let engine_work_dir = Path::new(&bg);
//
//     // Test connections to scanner and archive engine
//     let p = ProjectSettings::from_file(&args.project_settings);
//
//     if !p.archive_info.is_valid(&p.project_code,&args.civm_id) {
//         match args.no_archive.unwrap_or(false) {
//             false => {
//                 println!("project meta data is incorrect for archiving. You must repair the project settings at {:?} and try again.",args.project_settings);
//                 println!("if you want to run the recon anyway, re-run with the no archive option. Use --help to find it.");
//                 return
//             }
//             true => {
//                 println!("you have opted into running the recon despite not passing the meta data validation for archiving!");
//             }
//         }
//     }
//
//     if args.send_to_engine.unwrap_or(true) && !p.archive_engine_settings.test_connection(){
//         println!("you must fix the remote connection");
//         return
//     }
//
//     if !p.scanner_settings.test_connection() {
//         println!("you must fix the remote connection");
//         return
//     }
//
//
//     // Generate a vector of volume manager configurations that will be operated on
//     let mut vm_configs = recon_config::VolumeManagerConfig::new_dti_config(
//         &args.project_settings,
//         &args.civm_id,
//         &args.run_number,
//         &args.specimen_id,
//         &args.raw_data_base_dir,
//     );
//
//     // get subset of vm_configs based on user input (first and last volumes)
//     let user_last_vol = args.last_volume.unwrap_or(vm_configs.len()-1);
//     let upper_index = if user_last_vol >= vm_configs.len() {vm_configs.len()-1} else {user_last_vol};
//     let user_first_vol = args.first_volume.unwrap_or(0);
//     let lower_index = if user_first_vol >= vm_configs.len() {vm_configs.len()-1} else {user_first_vol};
//     let mut vm_configs = Vec::from_iter(vm_configs[lower_index..(upper_index +1)].iter().cloned());
//     println!("launching recon for range {} to {}. {} volumes will be launched for reconstruction", lower_index, upper_index, upper_index - lower_index +1);
//
//     // remind the user of their settings and confirm with them
//     if !args.batch_mode.unwrap_or(false) {
//         let mut user_in = String::new();
//         println!("----------------SETTINGS----------------");
//         println!("project_file = '{:?}'",args.project_settings);
//         println!("specimen_id = '{}'",args.specimen_id);
//         println!("{}",p.to_txt());
//         println!("----------------------------------------");
//         println!("is this configuration correct? Hit enter to continue or control-C to cancel");
//         stdin().read_line(&mut user_in).expect("provide an input!");
//     }
//
//     // Modify the volume manager configs based on options
//     vm_configs.iter_mut().for_each(|conf| {
//         conf.vm_settings.engine_work_dir = engine_work_dir.to_owned();
//         conf.send_to_engine = args.send_to_engine.unwrap_or(true);
//         conf.slurm_disabled = args.disable_slurm.unwrap_or(false);
//     });
//
//     // Make the .work directory
//     let work_dir = engine_work_dir.join(format!("{}.work",&args.run_number));
//     if !work_dir.exists() {
//         create_dir(&work_dir).expect(&format!("unable to create working directory {:?}",work_dir));
//     }
//
//     // Write fresh volume manager config files if they don't already exist
//     vm_configs.iter().for_each(|conf|{
//         let config_path = work_dir.join(conf.name());
//         create_dir_all(&config_path).expect(&format!("unable to create {:?}",config_path));
//         let conf_file = config_path.join(conf.name());
//         match VolumeManagerConfig::exists(&conf_file){
//             true => {
//                 println!("config already found. Will not re-initialize");
//             }
//             false => {
//                 println!("creating new configuration for volume manager {}",conf.name());
//                 conf.to_file(&conf_file);
//             }
//         }
//     });
//
//     // launch the volume managers
//     vm_configs.iter().for_each(|conf|{
//         let config_path = work_dir.join(conf.name());
//         let conf_file = config_path.join(conf.name());
//         match conf.is_slurm_disabled() {
//             true => {
//                 println!("launching volume manager without slurm {:?}",&conf_file);
//                 VolumeManager::launch(&conf_file);
//             },
//             false => {
//                 let jid = VolumeManager::launch_with_slurm_now(&conf_file);
//                 println!("{} job submitted with id {}",conf.name(),jid);
//             }
//         }
//     });
//
//     // launch a recon watcher to send email notifications when recon is complete
//     if args.email.is_some() && !args.disable_slurm.unwrap_or(false) {
//         slurm_recon_watch(
//             WaitForCompletionArgs{
//                 run_number: args.run_number.clone(),
//                 refresh_period:None,
//             },
//             &args.email.clone().unwrap()
//         );
//         println!("a recon watcher was launched on your behalf. Check your email {} for notifications",&args.email.unwrap());
//     }
//
// }