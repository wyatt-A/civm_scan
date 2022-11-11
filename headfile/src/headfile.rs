use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path,PathBuf};

pub struct Headfile{
    file:PathBuf
}

impl Headfile{

    pub fn new(file_path:&Path) -> Self {
        File::create(file_path).expect("cannot create file");
        Self {
            file:file_path.to_owned()
        }
    }

    pub fn open(file_path:&Path) -> Self {
        match file_path.exists() {
            false => Headfile::new(file_path),
            true => Self{
                file:file_path.to_owned()
            }
        }
    }

    pub fn append(&self,hash:&HashMap<String,String>) {
        let mut f = File::open(&self.file).expect("where did the headfile go!?");
        let mut s = String::new();
        f.read_to_string(&mut s).expect("trouble reading file");
        let mut h1 = Self::txt_to_hash(s);
        h1 = Self::merge(h1,hash.clone());
        let txt = Self::hash_to_txt(&h1);
        let mut f = File::create(&self.file).expect("can't create new file");
        f.write_all(txt.as_bytes()).expect("trouble writing to file");
    }

    fn merge(map1:HashMap<String,String>,map2:HashMap<String,String>) -> HashMap<String,String> {
        map1.into_iter().chain(map2).collect()
    }

    // pub fn append_field<T,U>(&mut self,key:T,value:U)
    // where T:std::string::ToString, U:std::string::ToString
    // {
    //     let old_val = self.items.insert(key.to_string(),value.to_string());
    //     if old_val.is_some(){
    //         println!("value {} updated to {}",old_val.unwrap(),value.to_string());
    //     }
    // }

    pub fn hash_to_txt(hash:&HashMap<String,String>) -> String {
        let mut strbuf = String::new();
        for (key, val) in hash.iter() {
            strbuf.push_str(key);
            strbuf.push('=');
            strbuf.push_str(val);
            strbuf.push('\n');
        }
        strbuf
    }

    fn txt_to_hash(headfile_str:String) -> HashMap<String,String>{
        let mut hf = HashMap::<String,String>::new();
        headfile_str.lines().for_each(|line|{
            // split on the first = we find
            match line.find("="){
                Some(index) => {
                    let (key,val) = line.split_at(index);
                    let key = key.to_string();
                    let mut val = val.to_string();
                    val.remove(0);// remove leading "="
                    hf.insert(key.to_string(),val);
                },
                None => () // do not add to hash if "=" not found
            }
        });
        return hf;
    }

}

fn transcribe_numeric<T>(hash:&mut HashMap<String,String>,old_name:&str,new_name:&str,scale:T)
where T: std::fmt::Display + std::str::FromStr + std::ops::MulAssign,
<T as std::str::FromStr>::Err: std::fmt::Debug
{
    match hash.get(old_name){
        Some(string) => {
            let mut num:T = string.parse().expect("cannot parse value");
            num *= scale;
            let str = num.to_string();
            hash.insert(new_name.to_string(),str);
        }
        None => {println!("{} field not found... not transcribing",old_name);}
    }
}

fn transcribe_string(hash:&mut HashMap<String,String>,old_name:&str,new_name:&str)
{
    match hash.get(old_name){
        Some(str) => {
            hash.insert(new_name.to_string(),str.to_string());
        },
        None => {
            println!("{} field not found... not transcribing",old_name);
        }
    }
}

pub trait AcqHeadfile {
    fn acq_params(&self) -> AcqHeadfileParams;
}

pub trait DWHeadfile:AcqHeadfile {
    fn diffusion_params(&self) -> DWHeadfileParams;
}


pub struct AcqHeadfileParams {
    pub dim_x:i32,
    pub dim_y:i32,
    pub dim_z:i32,
    pub fovx_mm:f32,
    pub fovy_mm:f32,
    pub fovz_mm:f32,
    pub te_ms:f32,
    pub tr_us:f32,
    pub alpha:f32,
    pub bw:f32,
    pub n_echos:i32,
    pub S_PSDname:String,
}

pub struct DWHeadfileParams {
    pub bvalue:f32,
    pub bval_dir:(f32,f32,f32)
}


/*
                headfile=mrs_meta_data(mrd);
                headfile.dti_vols = n_volumes;
                headfile.U_code = project_code;
                headfile.U_civmid = civm_userid;
                headfile.U_specid = specimen_id;
                headfile.scanner_vendor = scanner_vendor;
                headfile.U_runno = strcat(run_number,'_',mnum);
                headfile.dim_X = vol_size(1);
                headfile.dim_Y = vol_size(2);
                headfile.dim_Z = vol_size(3);
                headfile.civm_image_code = 't9';
                headfile.civm_image_source_tag = 'imx';
                headfile.engine_work_directory = pwd;
                */

pub struct ReconHeadfileParams {
    pub dti_vols:Option<i32>,
    pub project_code:String,
    pub civm_id:String,
    pub spec_id:String,
    pub scanner_vendor:String,
    pub run_number:String,
    pub m_number:String,
    pub image_code:String,
    pub image_tag:String,
    pub engine_work_dir:PathBuf,
}

impl ReconHeadfileParams {
    pub fn to_hash(&self) -> HashMap<String,String> {
        let mut h = HashMap::<String,String>::new();
        h.insert(String::from("dti_vols"),self.dti_vols.unwrap_or(0).to_string());
        h.insert(String::from("U_code"),self.project_code.clone());
        h.insert(String::from("U_civmid"),self.civm_id.clone());
        h.insert(String::from("U_specid"),self.spec_id.clone());
        h.insert(String::from("scanner_vendor"),self.scanner_vendor.clone());
        h.insert(String::from("U_runno"),format!("{}_{}",self.run_number.clone(),self.m_number.clone()));
        h.insert(String::from("civm_image_code"),self.image_code.clone());
        h.insert(String::from("civm_image_source_tag"),self.image_tag.clone());
        h.insert(String::from("engine_work_directory"),self.engine_work_dir.to_str().unwrap_or("").to_string());
        h
    }
}


impl AcqHeadfileParams {
    pub fn to_hash(&self) -> HashMap<String,String> {
        let mut h = HashMap::<String,String>::new();
        h.insert(String::from("dim_X"),self.dim_x.to_string());
        h.insert(String::from("dim_Y"),self.dim_y.to_string());
        h.insert(String::from("dim_Y"),self.dim_z.to_string());
        h.insert(String::from("fovx"),self.fovx_mm.to_string());
        h.insert(String::from("fovy"),self.fovy_mm.to_string());
        h.insert(String::from("fovz"),self.fovz_mm.to_string());
        h.insert(String::from("tr"),self.tr_us.to_string());
        h.insert(String::from("te"),self.te_ms.to_string());
        h.insert(String::from("bw"),self.bw.to_string());
        h.insert(String::from("ne"),self.n_echos.to_string());
        h.insert(String::from("S_PSDname"),self.S_PSDname.to_string());
        h
    }
}

impl DWHeadfileParams {
    pub fn to_hash(&self) -> HashMap<String,String> {
        let mut h = HashMap::<String,String>::new();
        let bval_dir = format!("3:1,{} {} {}",self.bval_dir.0,self.bval_dir.1,self.bval_dir.2);
        h.insert(String::from("bval_dir"),bval_dir);
        h.insert(String::from("bvalue"),self.bvalue.to_string());
        h
    }
}