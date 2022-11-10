use crate::utils;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use whoami;
use serde_json;
use crate::bart_wrapper::BartPicsSettings;
use crate::resource::Host;

#[derive(Serialize,Deserialize,Debug)]
pub struct Recon{
    pub run_number:String,
    pub specimen_id:String,
    pub volume_data:PathBuf,
    pub engine_work_dir:PathBuf,
    pub recon_person:String,
    pub n_volumes:Option<usize>,
    pub scanner:Scanner,
    pub project:ProjectSettings,
}

#[derive(Serialize,Deserialize,Debug)]
pub struct Scanner{
    pub label:String,
    pub username:String,
    pub hostname:String,
    pub vendor:String,
    pub vol_meta_suffix:String,
    pub image_code:String,
    pub image_source_tag:String,
}

#[derive(Serialize,Deserialize,Debug)]
pub struct ProjectSettings{
    pub label:String,
    pub project_code:String,
    pub recon_settings:BartPicsSettings,
}

// Recon::new("grumpy","test_runno","/some/vol_index.txt","5xfad")

impl Recon{
    pub fn new(scanner:&str,runno:&str,vol_data:&str,project:&str,specimen_id:&str) -> Recon{

        let p = Path::new(runno).with_extension("json").to_owned();
        
        let engine_work_dir = match std::env::var("BIGGUS_DISKUS"){
            Ok(dir) => Path::new(&dir).to_owned(),
            Err(_) => {
                println!("BIGGUS_DISKUS not set. Using home directory instead");
                Path::new(&std::env::var("HOME").expect("HOME not set. Are you on a Windows?")).to_owned()
            },
        };
        let r = Recon{
            run_number:runno.to_string(),
            volume_data:Path::new(vol_data).to_owned(),
            engine_work_dir:engine_work_dir,
            recon_person:whoami::username(),
            scanner:Scanner::open(scanner),
            project:ProjectSettings::open(project),
            specimen_id:specimen_id.to_string(),
            n_volumes:None,
        };
        let s = serde_json::to_string_pretty(&r).expect("cannot serialize struct");
        utils::write_to_file(p.to_str().unwrap(),"json",&s);
        return r;
    }

    pub fn open(runno:&str) -> Option<Recon>{
        let p = Path::new(runno).with_extension("json").to_owned();
        match p.exists(){
            true =>{
                let s = utils::read_to_string(p.to_str().unwrap(),"json").expect("what happened to the file??");
                return serde_json::from_str(&s).expect("cannot deserialize json file. It is likely corrupted!");
            },
            false => {
                println!("recon runno {} doesn't exist. Use new to create one",runno);
                return None;
            }
        }
    }

    pub fn path(&self) -> PathBuf{
        return Path::new(&self.run_number).with_extension("json");
    }

}

impl ProjectSettings{
    pub fn open(label:&str) -> ProjectSettings{
        //let str = utils::read_to_string(label,"toml");
        return match utils::read_to_string(label,"toml"){
            Ok(str) => toml::from_str(&str).expect("Cannot deserialize file. Is it the correct format?"),
            Err(_) => {
                println!("Project settings not found, creating a default template.");
                ProjectSettings::new_template(label)}
        }
    }
    pub fn new_template(label:&str) -> ProjectSettings{
        let project_settings = ProjectSettings{
            label:label.to_string(),
            project_code:"22.project.01".to_string(),
            recon_settings:BartPicsSettings::default(),
        };
        utils::write_to_file(label,"toml",&toml::to_string(&project_settings).expect("cannot serialize struct"));
        return project_settings;
    }
}

impl Scanner {
    pub fn open(label:&str) -> Scanner {
        return match utils::read_to_string(label,"toml"){
            Ok(str) => toml::from_str(&str).expect("Cannot deserialize file. Is it the correct format?"),
            Err(_) => {
                print!("Scanner settings not found. Creating a template.");
                return Scanner::new_template(label);
            }
        }
    }
    pub fn new_template(label:&str) -> Scanner{
        let scanner = Scanner{
            label:label.to_string(),
            vendor:"mrsolutions".to_string(),
            vol_meta_suffix:"_meta.txt".to_string(),
            image_code:"t9".to_string(),
            image_source_tag:"imx".to_string(),
            username:"user".to_string(),
            hostname:"hostname".to_string(),
        };
        let s = toml::to_string(&scanner).expect("cannot serialize struct");
        utils::write_to_file(label,"toml",&s);
        return scanner;
    }

    pub fn host(&self) -> Host{
        Host::new(&self.username,&self.hostname)
    }
}