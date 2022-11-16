/*
    cs_recon main is the entry point for the civm reconstruction pipeline that is using BART under
    the hood.
*/
use std::fs::create_dir_all;
use std::io::{stdin, stdout, Write};
//use recon::volume_manager::{launch_volume_manager,re_launch_volume_manager};
//use recon::test::{main_test_cluster};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Command;
use recon::slurm;
use recon::slurm::{BatchScript, get_job_state};
use recon::vol_manager::{VolumeManager, VolumeManagerArgs, VolumeManagerConfig};
use recon::vol_manager::test_updated;
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
    TemplateConfig(TemplateConfigArgs),
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
    base_config:PathBuf,
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
                VolumeMangerAction::TemplateConfig(args) => {
                    VolumeManagerConfig::default().to_file(&args.output_config);
                }
                _=> println!("not yet implemented!")
            }
        }
        ReconAction::DtiRecon(args) => {

            // load up config file
            let template_config = VolumeManagerConfig::from_file(&args.base_config);
            let recon_meta = template_config.recon_headfile.clone().expect("recon headfile must be defined in config for this reconstruction");
            let n_vols = recon_meta.dti_vols.expect("number of dti_vols must be defined in config for this reconstruction");

            // create dirs for volumes managers to live in
            let bg = std::env::var("BIGGUS_DISKUS").expect("BIGGUS_DISKUS is not set!");
            let base_path = Path::new(&bg);
            let runno_dir = base_path.join(&args.run_number).with_extension("work");
            let m_numbers = m_number_formatter(n_vols as usize);

            let mut this_config = template_config.clone();
            let mut this_recon_meta = recon_meta.clone();
            // configure meta/headfile info
            this_recon_meta.run_number = args.run_number.clone();
            this_recon_meta.civm_id = args.civm_id.clone();
            this_recon_meta.spec_id = args.specimen_id.clone();
            this_config.recon_headfile = Some(this_recon_meta.clone());

            println!("before we launch the reconstruction lets review the settings ...");

            let s = serde_json::to_string_pretty(&this_config).expect("cannot serialize struct");
            println!("{}",s);
            println!("hit enter to accept, or ^C to try again");
            let _=stdout().flush();
            let mut user_in = String::new();
            stdin().read_line(&mut user_in).expect("Did not enter a correct string");

            println!("configuring volume managers ...");
            m_numbers.iter().enumerate().for_each(|(index,mnum)| {
                let d = format!("{}_{}",&args.run_number,mnum);
                let local_dir = runno_dir.join(&d);

                if !local_dir.exists() {
                    create_dir_all(&local_dir).expect("cannot create directory");
                }

                // general volume manager config
                this_config.resource_dir = Some(args.raw_data_base_dir.join(mnum));
                // configure if this volume manager is setting the image scale for the others
                if args.scaling_volume.unwrap_or(0) as usize == index {
                    this_config.is_scale_dependent = Some(false);
                    this_config.is_scale_setter = Some(true);
                    this_config.scale_hist_percent = Some(this_config.recon_settings.clone().expect("bart pics settings not defined!").image_scale_histo_percent as f32);
                }
                else {
                    this_config.is_scale_dependent = Some(true);
                }
                let vmc = local_dir.join("volume_manager_config");
                this_config.to_file(&vmc);
                let vma = VolumeManagerArgs::new(&local_dir,&vmc);
                let args_file = vma.to_file();

                let vm = VolumeManager::new(&vma);

                match VolumeManager::is_sequential_mode() {
                    true => VolumeManager::launch(&args_file),
                    false => {
                        let jid = vm.launch_with_slurm_now();
                        let job_state = get_job_state(jid,60);
                        println!("{} submitted... {:?}",d,job_state);
                    }
                }
            });
            println!("done");
        }
    }
}