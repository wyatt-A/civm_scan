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
    DtiRecon(DtiRecon)
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
    NewProjectTemplate(TemplateConfigArgs),
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
    sequential:Option<bool>
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
                VolumeMangerAction::NewProjectTemplate(args) => {
                    ProjectSettings::default().to_file(&args.output_config)
                }
                _=> println!("not yet implemented!")
            }
        }
        ReconAction::DtiRecon(args) => {
            // test connection to remote systems...
            let p = ProjectSettings::from_file(&args.project_settings);
            if !p.archive_engine_settings.test_connection() {
                //p.archive_engine_settings.copy_ssh_key();
            }
            if !p.scanner_settings.test_connection() {
                //p.scanner_settings.copy_ssh_key();
            }
            let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS must be set on this workstation");
            let engine_work_dir = Path::new(&bg);
            let mut vm_configs = recon_config::VolumeManagerConfig::new_dti_config(&args.project_settings,&args.civm_id,&args.run_number,&args.specimen_id,&args.raw_data_base_dir);

            vm_configs.iter_mut().for_each(|conf| {
                conf.vm_settings.engine_work_dir = engine_work_dir.to_owned();
            });

            let work_dir = engine_work_dir.join(format!("{}.work",&args.run_number));
            if !work_dir.exists() {
                create_dir(&work_dir).expect(&format!("unable to create working directory {:?}",work_dir));
            }

            let jids:Vec<Option<u32>> = vm_configs.iter().map(|conf| {
                let config_path = work_dir.join(conf.name());
                create_dir_all(&config_path).expect(&format!("unable to create {:?}",config_path));
                let conf_file = config_path.join(conf.name());
                if !conf_file.exists(){
                    conf.to_file(&conf_file);
                }
                conf_file
            }).map(|config|{
                match VolumeManager::no_cluster_scheduling() {
                    true => {
                        VolumeManager::launch(&config);
                        //VolumeManager::lauch_with_srun(&config);
                        None
                    },
                    false => {
                        Some(VolumeManager::launch_with_slurm_now(&config))
                    }
                }
            }).collect();

            jids.iter().enumerate().for_each(|(i,j)|{
                match j {
                    Some(j) => {
                        let job_state = get_job_state(*j,60);
                        println!("{} job submitted... {:?}",vm_configs[i].name(),job_state);
                    }
                    None => {}
                }
            });
        }
    }
}