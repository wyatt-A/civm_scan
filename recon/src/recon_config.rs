use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Deserialize, Serialize};
use serde_json;
use toml;

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ScannerSettings {
    pub scanner_vendor:String,
    pub image_code:String,
    pub image_tag:String,
    pub remote_user:String,
    pub remote_host:String,
}

impl Config for ScannerSettings {
    fn default() -> Self {
        Self {
            scanner_vendor: String::from("mrsolutions"),
            image_code: String::from("t9"),
            image_tag: String::from("imx"),
            remote_user: String::from("mrs"),
            remote_host: String::from("stejskal"),
        }
    }
}

impl RemoteSystem for ScannerSettings {
    fn hostname(&self) -> String {
        self.remote_host.clone()
    }
    fn user(&self) -> String {
        self.remote_user.clone()
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ArchiveEngineSettings {
    pub remote_user:String,
    pub remote_host:String,
    pub base_dir:PathBuf,
}

impl Config for ArchiveEngineSettings {
    fn default() -> Self {
        Self {
            remote_user: String::from("wyatt"),
            remote_host: String::from("delos"),
            base_dir: PathBuf::from("/Volumes/delosspace"),
        }
    }
}

impl RemoteSystem for ArchiveEngineSettings {
    fn hostname(&self) -> String {
        self.remote_host.clone()
    }
    fn user(&self) -> String {
        self.remote_user.clone()
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub enum BartPicsAlgo {
    L1,
    L2
}

impl BartPicsAlgo {
    pub fn print(&self) -> String {
        match &self {
            BartPicsAlgo::L1 => String::from("l1"),
            BartPicsAlgo::L2 => String::from("l2")
        }
    }
}

impl Config for BartPicsAlgo {
    fn default() -> Self {
        BartPicsAlgo::L1
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ReconSettings {
    pub bart_binary:PathBuf,
    pub max_iter:u32,
    pub algorithm:BartPicsAlgo,
    pub respect_scaling:bool,
    pub regularization:f32,
    pub fermi_filter_w1:f32,
    pub fermi_filter_w2:f32,
    pub image_scale_hist_percent:f32
}

impl Config for ReconSettings {
    fn default() -> Self {
        Self {
            bart_binary: PathBuf::from("bart"),
            max_iter: 30,
            algorithm: BartPicsAlgo::default(),
            respect_scaling: true,
            regularization: 0.005,
            fermi_filter_w1: 0.15,
            fermi_filter_w2: 0.75,
            image_scale_hist_percent: 0.9995,
        }
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ProjectSettings {
    pub project_code:String,
    pub dti_vols:Option<usize>,
    pub recon_settings:ReconSettings,
    pub scanner_settings:ScannerSettings,
    pub archive_engine_settings:ArchiveEngineSettings,
}

impl Config for ProjectSettings {
    fn default() -> Self {
        Self {
            project_code: String::from("20.5xfad.01"),
            dti_vols: Some(67),
            recon_settings: ReconSettings::default(),
            scanner_settings: ScannerSettings::default(),
            archive_engine_settings: ArchiveEngineSettings::default(),
        }
    }
}

impl ConfigFile for ProjectSettings {

    fn to_file(&self, filename: &Path) {
        let t = toml::to_string_pretty(&self).unwrap();
        utils::write_to_file(&filename,&Self::file_ext(),&t);
    }
    fn from_file(filename: &Path) -> Self {
        let t = utils::read_to_string(filename,&Self::file_ext());
        toml::from_str(&t).expect("project settings must be corrupt")
    }
    fn file_ext() -> String {
        String::from("project_settings")
    }

}


#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct RunSettings {
    pub run_number:String,
    pub civm_id:String,
    pub spec_id:String,
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct VolumeManagerSettings {
    //pub work_dir:PathBuf,
    //pub m_number:String,
    pub volume_index:Option<usize>,
    pub engine_work_dir:PathBuf,
    pub resource_dir:PathBuf,
    pub is_scale_dependent:bool,
    pub is_scale_setter:bool,
}

impl VolumeManagerSettings {
    pub fn new(resource_directory:&Path,is_scale_setter:bool,is_scale_dependent:bool,vol_index:Option<usize>) -> Self {
        Self {
            volume_index: vol_index,
            engine_work_dir: PathBuf::from("/privateShares/wa41"),
            resource_dir: resource_directory.to_owned(),
            is_scale_dependent,
            is_scale_setter
        }
    }
    pub fn new_dti_settings(resource_base_dir:&Path,n_volumes:usize) -> Vec<Self> {
        let m_numbers = utils::m_number_formatter(n_volumes);
        let mut vms:Vec<VolumeManagerSettings> = m_numbers.iter()
            .map(|m| resource_base_dir.join(m))
            .enumerate().map(|(vol_index,dir)|
            VolumeManagerSettings::new(&dir,false,true,Some(vol_index))
        ).collect();
        vms[0].is_scale_setter = true;
        vms[0].is_scale_dependent = false;
        vms
    }
}



#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct VolumeManagerConfig {
    pub slurm_disabled:bool,
    pub run_settings:RunSettings,
    pub project_settings:ProjectSettings,
    pub vm_settings:VolumeManagerSettings,
}

impl ConfigFile for VolumeManagerConfig {

    fn to_file(&self, filename: &Path) {
        let t = toml::to_string_pretty(&self).unwrap();
        utils::write_to_file(filename,&Self::file_ext(),&t);
    }

    fn from_file(filename: &Path) -> Self {
        let t = utils::read_to_string(filename,&Self::file_ext());
        toml::from_str(&t).expect("volume manager config file is corrupt")
    }

    fn file_ext() -> String {
        String::from("volman_config")
    }

}

impl VolumeManagerConfig {
    pub fn new_dti_config(project_settings:&Path,civm_id:&str,run_number:&str,spec_id:&str,resource_dir:&Path,slurm_disabled:bool) -> Vec<Self> {
        let p = ProjectSettings::from_file(project_settings);
        let r = RunSettings {
            run_number: run_number.to_string(),
            civm_id: civm_id.to_string(),
            spec_id: spec_id.to_string()
        };
        let vms = VolumeManagerSettings::new_dti_settings(resource_dir,p.dti_vols.unwrap_or(1));
        vms.iter().map(|s| VolumeManagerConfig{
            project_settings:p.clone(),
            vm_settings:s.clone(),
            run_settings:r.clone(),
            slurm_disabled
        }).collect()
    }

    pub fn m_number(&self) -> String {
        let i = self.vm_settings.volume_index.unwrap_or(0);
        let n = self.project_settings.dti_vols.unwrap_or(1);
        utils::m_number(i,n)
    }

    pub fn name(&self) -> String {
        format!("{}_{}",self.run_settings.run_number,self.m_number())
    }

    pub fn exists(filename:&Path) -> bool {
        filename.with_extension(Self::file_ext()).exists()
    }
    pub fn is_slurm_disabled(&self) -> bool{
        self.slurm_disabled
    }
}


pub trait Config {
    fn default() -> Self;
}

pub trait ConfigFile {
    fn to_file(&self, filename:&Path);
    fn from_file(filename:&Path) -> Self;
    fn file_ext() -> String;
}

pub trait RemoteSystem {
    fn hostname(&self) -> String;
    fn user(&self) -> String;
    fn test_connection(&self) -> bool {
        println!("testing connection for user {} on {}", self.user(), self.hostname());
        let mut cmd = Command::new("ssh");
        cmd.arg("-o BatchMode=yes");
        cmd.arg(format!("{}@{}", self.user(), self.hostname()));
        cmd.arg("exit");
        match cmd.output().expect("failed to launch ssh").status.success() {
            true => {
                println!("connection successful");
                true
            }
            false => {
                println!("passwordless connection failed for {} on {}.", self.user(), self.hostname());
                println!("try  to run ssh-copy-id for {} on {} to fix the connection", self.user(), self.hostname());
                false
            }
        }
    }
    // fn copy_ssh_key(&self) {
    //     println!("enter password for user {} on {}", self.user(), self.hostname());
    //     let mut cmd = Command::new("ssh-copy-id");
    //     cmd.arg(format!("{}@{}", self.user(), self.hostname()));
    //     println!("attempting to run {:?}", cmd);
    //     let o = cmd.output().expect("failed to launch ssh-copy-id");
    // }
}

#[test]
fn test(){
    println!("running test!");
    let rec_settings = Path::new(r"C:\Users\waust\OneDrive\Desktop\test_data\recon_env\project");
    let d = ProjectSettings::default();
    d.to_file(rec_settings);
    let y = ProjectSettings::from_file(rec_settings);
    println!("{:?}",y);

    //let cfg = VolumeManagerConfig::new_dti_config(rec_settings,"wa41","N6000001","mr_tacos",Path::new("/some/path"));

    //cfg[0].to_file(&rec_settings.with_file_name("vol_00"));

    let g = VolumeManagerConfig::from_file(&rec_settings.with_file_name("vol_00"));
    println!("{:?}",g);
}

