use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Index;
use std::path::{Path,PathBuf};
use serde::{Deserialize, Serialize};
use regex::Regex;

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



// specid=211001-30:1
// civmid=wa41
// code=18.abb.11




// Dir params required for successful archival of data.
// They need to be validated before sending them off to the archive engine



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
            coil: String::from("9T_so13"),
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

        let mut is_valid = true;

        //todo!(implement correctness check)
        //$WKS_SETTINGS/recon_menu.txt
        let workstation_settings = std::env::var("WKS_SETTINGS").expect("WKS_SETTINGS not set!");
        let filepath = Path::new(&workstation_settings).join("recon_menu.txt");
        let mut f = File::open(&filepath).expect(&format!("cannot open file! {:?}",filepath));

        let mut txt = String::new();
        f.read_to_string(&mut txt).expect("trouble reading file");

        println!("{}",txt);


        // each line is either a menu type or a valid menu option
        // igonore if the line begins with #


        let r1 = Regex::new(r"ALLMENUTYPES;(\w+)").expect("invalid regex!");
        let r2 = Regex::new(r"^(.*?);").expect("invalid regex!");
        let r3 = Regex::new(r"MENUTYPE;(\w+)").expect("invalid regex!");

        // menu field -> menu item -> scanners for menu item
        let mut h = HashMap::<String,HashSet<String>>::new();

        let mut last_field = String::new();

        txt.lines().for_each(|line|{
            if !line.starts_with("#") && !r1.is_match(line){
                let c = r3.captures(line);
                match c {
                    Some(capture) =>{
                        let m = capture.get(1).unwrap();
                        last_field = m.as_str().to_string();
                        println!("last_field = {}",last_field);
                        h.insert(last_field.clone(),HashSet::<String>::new());
                    }
                    None => {
                        let c = r2.captures(line).expect(&format!("unknown format!{}",line));
                        let m = c.get(1).expect("capture group not found");
                        h.get_mut(&last_field).unwrap().insert(m.as_str().to_string());
                    }
                }
            }
        });

        // check that recognized fields have valid entries
        let mut h2 = self.to_hash();
        h2.insert(String::from("U_code"),project_code.to_string());
        h2.insert(String::from("U_civmid"),civm_user.to_string());
        h2.iter().for_each(|(key,val)|{
            let t = key.replace("U_","");
            match h.get(&t) {
                Some(set) => {
                    match &set.contains(val) {
                        false => {
                            println!("{} is not a valid entry for {}.",val,t);
                            is_valid = false;
                        }
                        _=> {}
                    }
                }
                None => {}
            }
        });

        // check that all non-empty fields are present
        h.iter().for_each(|(key,val)|{
            if !val.is_empty(){
                if !h2.contains_key(&format!("U_{}",key)){
                    println!("{} is not present in meta-data struct",key);
                    is_valid = false;
                }
            }
        });
        is_valid
    }
}

// coil=9T_So13
// nucleus=H
// species=mouse
// state=ex vivo
// orient=NA
// type=brain
// focus=whole
// rplane=cor
// xmit=0
// optional=
// status=ok

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


#[test]
fn test(){
    let a = ArchiveInfo::default();

    println!("archive info is {}",a.is_valid("20.5xfad.01","wa41"));

}