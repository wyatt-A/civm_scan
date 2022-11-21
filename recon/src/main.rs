use std::fs::{create_dir, create_dir_all};
use std::io::{stdin, stdout, Write};
//use recon::volume_manager::{launch_volume_manager,re_launch_volume_manager};
//use recon::test::{main_test_cluster};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Command;
use recon::{recon_config, slurm};
use recon::recon_config::{Config, ConfigFile, ProjectSettings, RemoteSystem, VolumeManagerConfig};
use recon::slurm::{BatchScript, get_job_state};
use recon::vol_manager::{VolumeManager,};
use utils::m_number_formatter;

#[derive(clap::Parser,Debug)]
pub struct ReconArgs {
    #[command(subcommand)]
    pub action: ReconAction,
}

#[derive(clap::Subcommand,Debug)]
pub enum ReconAction {
    VolumeManager(VolumeManagerCmd),
    Dti(DtiRecon),
    NewProjectTemplate(TemplateConfigArgs),
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
pub struct DtiRecon {
    /// civm id (cof,wa41 ...)
    civm_id:String,
    /// base configuration used to define recon parameters
    project_settings:PathBuf,
    /// run number for the collection of DTI volumes
    run_number:String,
    /// base path for the collection of DTI volumes
    raw_data_base_dir:PathBuf,
    /// civm specimen id
    specimen_id:String,
    /// index of the volume used for scaling. This is typically the first volume (defaults to 0)
    scaling_volume:Option<u32>,
    /// run sequentially
    #[clap(long)]
    disable_slurm:Option<bool>
}

#[derive(Clone,clap::Args,Debug)]
pub struct TemplateConfigArgs {
    output_config:PathBuf,
}

#[derive(Clone,clap::Args,Debug)]
pub struct VolumeMangerLaunchArgs {
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
        ReconAction::Dti(args) => {

            // Where are we going to live?
            let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS must be set on this workstation");
            let engine_work_dir = Path::new(&bg);

            // Test connections to scanner are archive engine
            let p = ProjectSettings::from_file(&args.project_settings);
            if !p.archive_engine_settings.test_connection() {
                //p.archive_engine_settings.copy_ssh_key();
                println!("you must fix the remote connection");
                return
            }
            if !p.scanner_settings.test_connection() {
                //p.scanner_settings.copy_ssh_key();
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
                        let job_state = get_job_state(jid,60);
                        println!("{} job submitted... {:?}",conf.name(),job_state);
                    }
                }
            });
        }
    }
}