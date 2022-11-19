use std::path::Path;
use std::fs::File;
use std::io::{Write, Read};

pub fn m_number_formatter(n_elements:usize) -> Vec<String>{
    let w = ((n_elements-1) as f32).log10().floor() as usize + 1;
    let formatter = |index:usize| format!("m{:0width$ }",index,width=w);
    (0..n_elements).map(|index| formatter(index)).collect()
}

pub fn m_number(index:usize,n_total:usize) -> String {
    let w = ((n_total-1) as f32).log10().floor() as usize + 1;
    let formatter = |index:usize| format!("m{:0width$ }",index,width=w);
    formatter(index)
}

pub fn read_to_string(filepath:&Path,extension:&str) -> String {
    let p = filepath.with_extension(extension);
    let mut f = File::open(&p).expect(&format!("cannot open file {:?}",p));
    let mut s = String::new();
    f.read_to_string(&mut s).expect("trouble reading file");
    s
}

pub fn write_to_file(filepath:&Path,extension:&str,string:&str){
    let p = filepath.with_extension(extension);
    let mut f = File::create(p).expect("failed to create file");
    f.write_all(string.as_bytes()).expect("trouble writing to file");
}

pub fn vec_to_string<T>(vec:&Vec<T>) -> String
    where T:std::string::ToString {
    let vstr:Vec<String> = vec.iter().map(|num| num.to_string()).collect();
    return vstr.join(" ");
}

pub fn bytes_to_long(byte_slice:&[u8]) -> i32{
    let mut byte_buff = [0;4];
    byte_buff.copy_from_slice(&byte_slice);
    return i32::from_le_bytes(byte_buff);
}

pub fn bytes_to_int(byte_slice:&[u8]) -> i16{
    let mut byte_buff = [0;2];
    byte_buff.copy_from_slice(byte_slice);
    return i16::from_le_bytes(byte_buff);
}

pub fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}
