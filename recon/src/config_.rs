use serde::{Deserialize, Serialize};
use std::path::{Path,PathBuf};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use toml;
use whoami;

const RC_FNAME:&str = "ResourceConfig";
const RF_PATH:&str = ".";

/*
Things we only need to set up occasionally. These things should't be chaning all the time
In short, where the data lives on the network (essentially scanner settings)
*/ 
#[derive(Debug,Deserialize,Serialize)]
pub struct ResourceConfig{
    pub remote_host:String,
    pub remote_user:String,
    pub remote_base_path:String,
}

/*
Things we need to pay attention to every time a recon is launched
*/ 
#[derive(Debug,Deserialize,Serialize)]
pub struct ReconForm {
    pub run_number:String,
    pub relative_index_file_path:String,
    pub relative_mrd_path:String,
}

/*
Things that don't typically change for a given project/imaging protocol (but can)
*/
pub struct ReconProtocol {
    pub project_code:String,
    pub recon_settings:ReconSettings,
}

pub struct ReconSettings {
    // things that determine how recon is performed
}

impl ResourceConfig {
    fn template() -> ResourceConfig{
        return ResourceConfig { 
            remote_host:String::new(),
            remote_user:String::new(),
            remote_base_path:String::new(),
        }
    }

    pub fn new(remote_host:&str,remote_user:&str,remote_base_path:&str) -> ResourceConfig {
        return ResourceConfig{
            remote_host:remote_host.to_string(),
            remote_user:remote_user.to_string(),
            remote_base_path:remote_base_path.to_string()
        };
    }

    pub fn save(&self,filepath:&str){
        let p = Path::new(filepath).to_owned().with_extension("toml");
        let mut f = File::create(p).expect("cannot create file");
        let str = toml::to_string(&self).expect("cannot serialize data struct");
        f.write_all(str.as_bytes()).expect("issue writing to file");
    }

    fn fullpath() -> PathBuf{
        let p = Path::new("./");
        return p.with_file_name(RC_FNAME).with_extension("toml");
    }

    pub fn create_template(){
        if ResourceConfig::fullpath().exists() {
            println!("Config already exists. Will not overwrite.");
            return;
        }
        let mut f = File::create(ResourceConfig::fullpath()).expect("cannot create file");
        let rct = ResourceConfig::template();
        let str = toml::to_string(&rct).expect("cannot serialize data struct");
        f.write_all(str.as_bytes()).expect("issue writing to file");
    }

    pub fn from_toml(filename:&str) -> ResourceConfig{
        let mut f = File::open(filename).expect("cannot open file");
        let mut str = String::new();
        f.read_to_string(&mut str).expect("issue reading file");
        let r:ResourceConfig = toml::from_str(&str).expect("cannot deserialize. Is the file improperly formatted?");
        return r;
    }

    pub fn is_complete(&self) -> bool{
        return !(
            self.remote_user.is_empty() ||
            self.remote_host.is_empty() ||
            self.remote_base_path.is_empty()
        );
    }

}

impl ReconForm {
    pub fn template() -> ReconForm{
        return ReconForm { 
            run_number:String::new(),
            relative_index_file_path:String::new(),
            relative_mrd_path:String::new(),
        }
    }

    pub fn new(run_number:&str,index_path:&str) -> ReconForm{
        let p = Path::new(RF_PATH).to_owned();
        let mut r =  ReconForm::template();
        r.run_number = run_number.to_string();
        r.relative_index_file_path = index_path.to_string();
        let s = toml::to_string(&r).expect("cannot serialize data struct");
        let full = p.with_file_name(run_number).with_extension("toml");
        if !full.exists(){
            let mut f = File::create(full).expect("cannot create file");
            f.write_all(s.as_bytes()).expect("trouble writing to file");
        }else{
            println!("file {} already exists. Will not overwrite.",full.to_str().unwrap())
        }
        return r;
    }

    pub fn from_toml(filename:&str) -> ReconForm{
        let mut f = File::open(filename).expect("cannot open file");
        let mut str = String::new();
        f.read_to_string(&mut str).expect("issue reading file");
        let r:ReconForm = toml::from_str(&str).expect("cannot deserialize. Is the file improperly formatted?");
        return r;
    }

    pub fn save(&self,filepath:&str){
        let p = Path::new(filepath).to_owned().with_extension("toml");
        let mut f = File::create(p).expect("cannot create file");
        let str = toml::to_string(&self).expect("cannot serialize data struct");
        f.write_all(str.as_bytes()).expect("issue writing to file");
    }
}

#[test]
fn recon_form_test(){
    // ReconForm::new(
    //     "N60tacos01",
    //     "N20220804_00/_02_ICO61_6b0/volume_index.txt",
    //     "22.tacos.01"
    // );

    // let r = ResourceConfig::new("stejskal","mrs","/d/smis");
    // r.save("./Grumpy");


    //let r = ResourceConfig::import("./Grumpy.toml");

}