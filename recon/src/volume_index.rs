use std::io::Read;
use std::path::{Path,PathBuf};
use std::collections::HashMap;
use std::fs::File;
use crate::resource::{Resource,Host};

pub struct VolumeIndex{

}

impl VolumeIndex{
    pub fn read_ready(path:&str) -> HashMap<String,String>{
        let mut h = HashMap::<String,String>::new();
        let str = VolumeIndex::read_to_string(path);
        str.lines().for_each(|line| {
            let strvec:Vec<&str> = line.split_whitespace().collect();
            if strvec.len() == 2 {
                h.insert(strvec[1].to_string(),strvec[0].to_string());
            }
        }
        );
        return h;
    }

    pub fn read_all(path:&str) -> HashMap<String,Option<String>>{
        let mut h = HashMap::<String,Option<String>>::new();
        let str = VolumeIndex::read_to_string(path);
        str.lines().for_each(|line| {
            let strvec:Vec<&str> = line.split_whitespace().collect();
            match strvec.len() {
                1 => {h.insert(strvec[0].to_string(),None)},
                2 => {h.insert(strvec[0].to_string(),Some(strvec[1].to_string()))},
                _ => panic!("has the volume index been corrupted?")
            };
        });
        return h;
    }

    fn read_to_string(path:&str) -> String{
        let fpath = Path::new(path);
        let mut f = File::open(fpath).expect("cannot open file");
        let mut strbuff = String::new();
        f.read_to_string(&mut strbuff).expect("problem reading file");
        return strbuff;
    }

    pub fn fetch_from(remote_path:&str,remote_host:&Host,destination:&str) -> String{
        let mut r = Resource::new(remote_path,destination);
        r.set_remote_host(remote_host);
        r.update(false);
        return r.local_path();
    }

}


#[test]
fn test(){
    let vpath = "/Users/Wyatt/local_recon/volume_index.txt";
    let h = VolumeIndex::read_ready(vpath);
    let h1 = VolumeIndex::read_all(vpath);
    println!("{:?}",h);
    println!("{:?}",h1);
}