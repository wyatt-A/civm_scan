use std::fs::{create_dir, create_dir_all};
use std::io::{stdin, stdout, Write};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Command;
use recon::{recon_config, slurm};
use recon::recon_config::{Config, ConfigFile, ProjectSettings, RemoteSystem, VolumeManagerConfig};
use recon::slurm::{BatchScript, get_job_state};
use recon::vol_manager::{VolumeManager};
use utils::m_number_formatter;

#[derive(clap::Parser,Debug)]
pub struct ReconArgs {
    #[command(subcommand)]
    pub action: ReconAction,
}

#[derive(clap::Subcommand,Debug)]
pub enum ReconAction {
    /// reconstruct a diffusion-weighted series of volumes
    Dti(DtiRecon),
    /// check the status of a reconstruction by run number
    Status(RunnoArgs),
    /// restart a recon by run number
    Restart(RunnoArgs),
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
pub struct RunnoArgs {
    run_number:String,
}

#[derive(Clone,clap::Args,Debug)]
pub struct DtiRecon {
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
        ReconAction::Dti(args) => dti(args)
    }
}

fn restart(args:RunnoArgs){
    let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS must be set on this workstation");
    let engine_work_dir = Path::new(&bg);
    let work_dir = engine_work_dir.join(format!("{}.work",&args.run_number));
    // find all volume manager config files recursively
    let config_files = utils::find_files(&work_dir,"volman_config");
    match config_files {
        Some(files) => {
            for config_file in files.iter() {
                let c = VolumeManagerConfig::from_file(config_file);
                match c.is_slurm_disabled() {
                    true => {
                        VolumeManager::launch(config_file);
                    },
                    false => {
                        VolumeManager::launch_with_slurm_now(config_file);
                    },
                }
            }
            status(args);
        },
        None => {
            println!("no volume manager configs found in {:?}",work_dir);
        }
    }

}

fn status(args:RunnoArgs) {
    let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS must be set on this workstation");
    let engine_work_dir = Path::new(&bg);
    let work_dir = engine_work_dir.join(format!("{}.work",&args.run_number));
    // find all volume manager state files recursively
    let state_files = utils::find_files(&work_dir,"vol_man");

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

fn dti(args:DtiRecon){
    // Where are we going to live?
    let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS must be set on this workstation");
    let engine_work_dir = Path::new(&bg);

    // Test connections to scanner and archive engine
    let p = ProjectSettings::from_file(&args.project_settings);

    if !p.archive_info.is_valid(&p.project_code,&args.civm_id) {
        match args.no_archive.unwrap_or(false) {
            false => {
                println!("project meta data is incorrect for archiving. You must repair the project settings at {:?} and try again.",args.project_settings);
                println!("if you want to run the recon anyway, re-run with the no archive option. Use --help to find it.");
                return
            }
            true => {
                println!("you have opted into running the recon despite not passing the meta data validation for archiving!");
            }
        }
    }

    if !p.archive_engine_settings.test_connection() {
        println!("you must fix the remote connection");
        return
    }
    if !p.scanner_settings.test_connection() {
        println!("you must fix the remote connection");
        return
    }

    // Generate a vector of volume manager configurations that will be operated on
    let mut vm_configs = recon_config::VolumeManagerConfig::new_dti_config(
        &args.project_settings,
        &args.civm_id,
        &args.run_number,
        &args.specimen_id,
        &args.raw_data_base_dir,
        args.disable_slurm.unwrap_or(false)
    );

    // get subset of vm_configs based on user input (first and last volumes)
    let user_last_vol = args.last_volume.unwrap_or(vm_configs.len()-1);
    let upper_index = if user_last_vol >= vm_configs.len() {vm_configs.len()-1} else {user_last_vol};
    let user_first_vol = args.first_volume.unwrap_or(0);
    let lower_index = if user_first_vol >= vm_configs.len() {vm_configs.len()-1} else {user_first_vol};
    let mut vm_configs = Vec::from_iter(vm_configs[lower_index..(upper_index +1)].iter().cloned());
    println!("launching recon for range {} to {}. {} volumes will be launched for reconstruction", lower_index, upper_index, upper_index - lower_index +1);

    // remind the user of their settings and confirm with them
    if !args.batch_mode.unwrap_or(false) {
        let mut user_in = String::new();
        println!("----------------SETTINGS----------------");
        println!("project_file = '{:?}'",args.project_settings);
        println!("specimen_id = '{}'",args.specimen_id);
        println!("{}",p.to_txt());
        println!("----------------------------------------");
        println!("is this configuration correct? Hit enter to continue or control-C to cancel");
        stdin().read_line(&mut user_in).expect("provide an input!");
    }
    // Modify the engine work dir for the configs
    vm_configs.iter_mut().for_each(|conf| {
        conf.vm_settings.engine_work_dir = engine_work_dir.to_owned();
    });

    // Make the .work directory
    let work_dir = engine_work_dir.join(format!("{}.work",&args.run_number));
    if !work_dir.exists() {
        create_dir(&work_dir).expect(&format!("unable to create working directory {:?}",work_dir));
    }

    // Write fresh volume manager config files if they don't already exist
    vm_configs.iter().for_each(|conf|{
        let config_path = work_dir.join(conf.name());
        create_dir_all(&config_path).expect(&format!("unable to create {:?}",config_path));
        let conf_file = config_path.join(conf.name());
        match VolumeManagerConfig::exists(&conf_file){
            true => {
                println!("config already found. Will not re-initialize");
            }
            false => {
                println!("creating new configuration for volume manager {}",conf.name());
                conf.to_file(&conf_file);
            }
        }
    });

    // launch the volume managers
    vm_configs.iter().for_each(|conf|{
        let config_path = work_dir.join(conf.name());
        let conf_file = config_path.join(conf.name());
        match conf.is_slurm_disabled() {
            true => {
                println!("launching volume manager without slurm {:?}",&conf_file);
                VolumeManager::launch(&conf_file);
            },
            false => {
                let jid = VolumeManager::launch_with_slurm_now(&conf_file);
                println!("{} job submitted with id {}",conf.name(),jid);
            }
        }
    });
}