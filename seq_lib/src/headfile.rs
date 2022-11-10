use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path,PathBuf};

pub struct Headfile{
    items:HashMap<String,String>
}

impl Headfile{

    pub fn append_field<T,U>(&mut self,key:T,value:U)
    where T:std::string::ToString, U:std::string::ToString
    {
        let old_val = self.items.insert(key.to_string(),value.to_string());
        if old_val.is_some(){
            println!("value {} updated to {}",old_val.unwrap(),value.to_string());
        }
    }

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

#[test]
fn test_make_headfile() {
    let test_file = "/Users/Wyatt/cs_recon/test_data/N20220808_00/_02_ICO61_6b0/220808T12_m00_meta.txt";
    let headfile = "test.headfile";
    let mut hf = Headfile::from_mrd_meta(&Path::new(test_file));
    hf.write_headfile(Path::new(headfile));
    hf.append_field("DUMMYFIELD",6.5);
    hf.write_headfile(&Path::new(test_file));
}