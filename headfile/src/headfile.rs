use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Index;
use std::path::{Path,PathBuf};
use serde::{Deserialize, Serialize};
use regex::Regex;
use utils;



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


#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct ReconHeadfile {
    pub spec_id: String,
    pub civmid: String,
    pub project_code: String,
    pub dti_vols:Option<usize>,
    pub scanner_vendor:String,
    pub run_number:String,
    pub m_number:String,
    pub image_code:String,
    pub image_tag:String,
    pub engine_work_dir:PathBuf,
    pub more_archive_info:ArchiveInfo
}


#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct ArchiveInfo {
    pub coil:String,
    pub nucleus:String,
    pub species:String,
    pub state:String,
    pub orient:String,
    pub type_:String,
    pub focus:String,
    pub rplane:String,
    pub xmit:String,
    pub optional:String,
    pub status:String,
}

impl ArchiveInfo {
    pub fn default() -> Self {
        Self {
            coil: String::from("9T_So13"),
            nucleus: String::from("H"),
            species: String::from("mouse"),
            state: String::from("ex vivo"),
            orient: String::from("NA"),
            type_: String::from("brain"),
            focus: String::from("whole"),
            rplane: String::from("cor"),
            xmit: String::from("0"),
            optional: String::from(""),
            status: String::from("ok")
        }
    }

    pub fn to_hash(&self) -> HashMap<String,String> {
        let mut h = HashMap::new();
        h.insert(String::from("U_coil"),self.coil.clone());
        h.insert(String::from("U_nucleus"),self.nucleus.clone());
        h.insert(String::from("U_species"),self.species.clone());
        h.insert(String::from("U_state"),self.state.clone());
        h.insert(String::from("U_orient"),self.orient.clone());
        h.insert(String::from("U_type"),self.type_.clone());
        h.insert(String::from("U_focus"),self.focus.clone());
        h.insert(String::from("U_rplane"),self.rplane.clone());
        h.insert(String::from("U_xmit"),self.xmit.clone());
        h.insert(String::from("U_status"),self.status.clone());
        h
    }

    // need to run a check on this information to ensure it is correct
    pub fn is_valid(&self,project_code:&str,civm_user:&str) -> bool {

        // will will assume the fields are valid until proven otherwise
        let mut is_valid = true;

        //$WKS_SETTINGS/recon_menu.txt contains the fields and valid values. We read them to a string here
        let workstation_settings = std::env::var("WKS_SETTINGS").expect("WKS_SETTINGS not set!");
        let filepath = Path::new(&workstation_settings).join("recon_menu.txt");
        let mut f = File::open(&filepath).expect(&format!("cannot open file! {:?}",filepath));
        let mut recon_menu_txt = String::new();
        f.read_to_string(&mut recon_menu_txt).expect("trouble reading file");


        // define the format of the menu file with regex. Each line is one of these 3 categories
        let all_menu_types_pattern = Regex::new(r"ALLMENUTYPES;(\w+)").expect("invalid regex!");
        let menu_field_pattern = Regex::new(r"^(.*?);").expect("invalid regex!");
        let menu_type_pattern = Regex::new(r"MENUTYPE;(\w+)").expect("invalid regex!");

        // internal data structure for the file
        let mut recon_menu = HashMap::<String,HashSet<String>>::new();

        // we need to store the last menu type because we will parse the file in a single pass
        let mut last_menu_type = String::new();

        // parse the recon menu, ignoring commented lines and the "all_menu_types" pattern
        recon_menu_txt.lines().for_each(|line|{
            if !line.starts_with("#") && !all_menu_types_pattern.is_match(line){
                let c = menu_type_pattern.captures(line);
                match c {
                    Some(capture) =>{
                        let m = capture.get(1).unwrap();
                        last_menu_type = m.as_str().to_string();
                        recon_menu.insert(last_menu_type.clone(), HashSet::<String>::new());
                    }
                    None => {
                        let c = menu_field_pattern.captures(line).expect(&format!("unknown format!{}", line));
                        let m = c.get(1).expect("capture group not found");
                        recon_menu.get_mut(&last_menu_type).unwrap().insert(m.as_str().to_string());
                    }
                }
            }
        });

        // here we check that this struct contains valid field entries with the exception
        // of transmit, which needs to be a "number" (assuming integer for now)
        let mut user_archive_info = self.to_hash();
        user_archive_info.insert(String::from("U_code"), project_code.to_string());
        user_archive_info.insert(String::from("U_civmid"), civm_user.to_string());
        user_archive_info.iter().for_each(|(key,val)|{
            let t = key.replace("U_","");
            match recon_menu.get(&t) {
                Some(set) => {
                    match &set.contains(val) {
                        false => {
                            match t.as_str() {
                                "xmit" => { // check that transmit is a "number" (what is a number?)
                                    val.chars().for_each(|char| {
                                        if !char.is_numeric(){
                                            println!("xmit contains non-numeric characters: {}",val);
                                            is_valid = false
                                        }
                                    });
                                }
                                _=> {
                                    println!("{} is not a valid entry for {}.",val,t);
                                    is_valid = false;
                                }
                            }
                        }
                        _=> {}
                    }
                }
                None => {}
            }
        });

        // here we check that our struct contains all fields required by the recon menu, with the
        // exception of runno. Is runno formatting actually enforced??
        recon_menu.iter().for_each(|(key,val)|{
            if !val.is_empty(){
                match key.as_str() {
                    "runno" => {},
                    _=> {
                        if !user_archive_info.contains_key(&format!("U_{}", key)) {
                            println!("{} is not present in meta-data struct", key);
                            is_valid = false;
                        }
                    }
                }
            }
        });
        is_valid
    }
}

impl ReconHeadfile {

    pub fn default() -> Self {
        Self {
            spec_id: String::from("mr_tacos"),
            civmid: String::from("wa41"),
            project_code: String::from("00.project.00"),
            dti_vols: Some(1),
            scanner_vendor: "mrsolutions".to_string(),
            run_number: "N60tacos".to_string(),
            m_number: "m00".to_string(),
            image_code: "t9".to_string(),
            image_tag: "imx".to_string(),
            engine_work_dir: PathBuf::from(std::env::var("BIGGUS_DISKUS").expect("biggus diskus not set!")),
            more_archive_info:ArchiveInfo::default()
        }
    }

    pub fn to_hash(&self) -> HashMap<String,String> {
        let mut h = HashMap::<String,String>::new();
        h.insert(String::from("U_specid"),self.spec_id.clone());
        h.insert(String::from("U_civmid"),self.civmid.clone());
        h.insert(String::from("U_code"),self.project_code.clone());
        h.insert(String::from("dti_vols"),self.dti_vols.unwrap_or(0).to_string());
        h.insert(String::from("scanner_vendor"),self.scanner_vendor.clone());
        h.insert(String::from("U_runno"),format!("{}_{}",self.run_number.clone(),self.m_number.clone()));
        h.insert(String::from("civm_image_code"),self.image_code.clone());
        h.insert(String::from("civm_image_source_tag"),self.image_tag.clone());
        h.insert(String::from("engine_work_directory"),self.engine_work_dir.to_str().unwrap_or("").to_string());
        h.insert(String::from("F_imgformat"),String::from("raw"));
        h.extend(self.more_archive_info.to_hash());
        h
    }

    pub fn to_file(&self,file_path:&Path) {
        let default = Self::default();
        let s = serde_json::to_string_pretty(&default).expect("cannot serialize struct");
        let mut f = File::create(file_path).expect("cannot create file");
        f.write_all(s.as_bytes()).expect("cannot write to file");
    }

    fn from_file(file_path:&Path) -> Self {
        let mut f = File::open(file_path).expect("cannot open file");
        let mut s = String::new();
        f.read_to_string(&mut s).expect("cannot read from file");
        serde_json::from_str(&s).expect("cannot deserialize file")
    }
}

impl AcqHeadfileParams {
    pub fn to_hash(&self) -> HashMap<String,String> {
        let mut h = HashMap::<String,String>::new();
        h.insert(String::from("dim_X"),self.dim_x.to_string());
        h.insert(String::from("dim_Y"),self.dim_y.to_string());
        h.insert(String::from("dim_Z"),self.dim_z.to_string());
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

    pub fn to_hash(&self) -> HashMap<String,String> {
        let mut f = File::open(&self.file).expect("where did the headfile go!?");
        let mut s = String::new();
        f.read_to_string(&mut s).expect("trouble reading file");
        Self::txt_to_hash(s)
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

    pub fn append_comment_block(&self,txt:&str,comment_char:&str) {
        let mut f = File::open(&self.file).expect("where did the headfile go!?");
        let mut s = String::new();
        f.read_to_string(&mut s).expect("trouble reading file");
        s.push('\n');
        txt.lines().for_each(|line|{
            let commented_line = format!("{}{}\n",comment_char,line);
            s.push_str(&commented_line);
        });
        let mut f = File::create(&self.file).expect("can't create new file");
        f.write_all(s.as_bytes()).expect("unable to update headfile");
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

//N60200_m41,/Volumes/delosspace,256,18.abb.11,.raw
// # recon_person=wa41
// # tag_file_creator=James_matlab

pub struct ArchiveTag {
    pub runno:String,
    pub civm_id:String,
    pub archive_engine_base_dir:PathBuf,
    pub n_raw_files:usize,
    pub project_code:String,
    pub raw_file_ext:String,
}

impl ArchiveTag {
    fn name_ready(&self) -> String {
        format!("READY_{}",self.runno)
    }
    pub fn to_file(&self,location:&Path){
        let base_dir = self.archive_engine_base_dir.to_str().unwrap();
        let txt = vec![
            format!("{},{},{},{},.{}",self.runno,base_dir,self.n_raw_files,self.project_code,self.raw_file_ext),
            format!("# recon_person={}",self.civm_id),
            format!("# tag_file_creator=Wyatt_rust\n"),
        ].join("\n");
        utils::write_to_file(&self.filepath(location), "", &txt);
    }
    pub fn filepath(&self,location:&Path) -> PathBuf {
        location.join(self.name_ready())
    }
}
