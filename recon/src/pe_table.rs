use std::fs::File;
use std::path::Path;
use std::io::{BufReader,Read};
use regex::Regex;

pub struct Petable {
    pub file:File,
    pub size:usize,
    pub compression:u32,
}

impl Petable {
    pub fn new(path:&str) -> Petable{
        let pe_table_path = Path::new(path);
        let file = File::open(pe_table_path).expect("cannot open pe_table");
        let table_name = pe_table_path.file_name().unwrap().to_str().unwrap();
        let re = Regex::new(r"_[Cc][Ss]([0-9]{1,})_([0-9]{1,})x_").unwrap();
        let caps = re.captures(table_name).unwrap();
        let size:usize = caps.get(1).map_or("", |m| m.as_str()).parse().expect("cannot parse table size");
        let compression:u32 = caps.get(2).map_or("", |m| m.as_str()).parse().expect("cannot parse table compression");
        return Petable{file:file,size:size,compression:compression};
    }

    fn read_values(&self) -> Vec<i32> {
        println!("reading phase encode table ...");
        let mut file_reader = BufReader::new(&self.file);
        let mut strbuff = String::new();
        file_reader.read_to_string(&mut strbuff).expect("oops. Something went wrong reading the pe table");
        return strbuff.split("\r\n").flat_map(|x| x.parse()).collect();
    }

    pub fn coordinates(&self) -> Vec<(i32,i32)>{
        let vals = self.read_values();
        let n = vals.len()/2;
        let mut indices:Vec<(i32,i32)> = Vec::with_capacity(n);
        for i in 0..n {
            indices.push((vals[2*i],vals[2*i+1]));
        };
        return indices;
    }

    pub fn indices(&self) -> Vec<(usize,usize)> {
        let coords = self.coordinates();
        let offset = (self.size/2) as i32;
        return coords.iter().map(|coord| (
            (coord.0 + offset) as usize,
            (coord.1 + offset) as usize
        )).collect();
    }
}

#[test]
fn test(){
    //let ptab = Petable::new("/Users/Wyatt/cs_recon/test_data/petableCS_stream/stream_CS480_8x_pa18_pb54",480);

    let re = Regex::new(r"_[Cc][Ss]([0-9]{1,})_([0-9]{1,})x_").unwrap();
    let caps = re.captures("stream_CS480_8x_pa18_pb54").unwrap();
    let size:usize = caps.get(1).map_or("", |m| m.as_str()).parse().expect("cannot parse table size");
    let compression:usize = caps.get(2).map_or("", |m| m.as_str()).parse().expect("cannot parse table compression");

    println!("compression = {}",compression);
    println!("size = {}",size);
}


/*
let re = Regex::new(r"[a-z]+(?:([0-9]+)|([A-Z]+))").unwrap();
let caps = re.captures("abc123").unwrap();

let text1 = caps.get(1).map_or("", |m| m.as_str());
let text2 = caps.get(2).map_or("", |m| m.as_str());
assert_eq!(text1, "123");
assert_eq!(text2, "");
*/