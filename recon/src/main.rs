/*
    cs_recon main is the entry point for the civm reconstruction pipeline that is using BART under
    the hood.
*/
//use recon::volume_manager::{launch_volume_manager,re_launch_volume_manager};
//use recon::test::{main_test_cluster};
use clap::Parser;
use std::path::{Path, PathBuf};
use recon::vol_manager::{VolumeManager, VolumeManagerArgs};
use recon::vol_manager::test_updated;

#[derive(clap::Parser,Debug)]
pub struct ReconArgs {
    #[command(subcommand)]
    pub action: ReconAction,
}


#[derive(clap::Subcommand,Debug)]
pub enum ReconAction {
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
    TemplateConfig(TemplateConfigArgs)
}

#[derive(Clone,clap::Args,Debug)]
pub struct NewConfigArgs {
    template_config:PathBuf,
    run_number:String,
    specimen_id:String,
    volume_name:String,
    output_config:PathBuf,
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
                VolumeMangerAction::TemplateConfig(args) => {
                    args.output_config
                }
            }
        }
    }
}