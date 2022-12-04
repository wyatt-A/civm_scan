use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Write, Read};
use glob::glob;
use walkdir::WalkDir;
use rustfft::{FftPlanner};
use num_complex::Complex;
use argmin::solver::newton::Newton;

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

pub fn get_first_match(dir:&Path,pattern:&str) -> Option<PathBuf>  {
    let pat = dir.join(pattern);
    let pat = pat.to_str().expect("cannot coerce to str");
    let matches:Vec<PathBuf> = glob(pat).expect("Failed to read glob pattern").flat_map(|m| m).collect();
    match matches.is_empty() {
        true => None,
        false => Some(matches[0].clone())
    }
}


// single depth search
pub fn get_all_matches(dir:&Path,pattern:&str) -> Option<Vec<PathBuf>> {
    let pat = dir.join(pattern);
    let pat = pat.to_str().expect("cannot coerce to str");
    let matches:Vec<PathBuf> = glob(pat).expect("Failed to read glob pattern").flat_map(|m| m).collect();
    match matches.is_empty() {
        true => None,
        false => Some(matches)
    }
}


// recursive walk
pub fn find_files(base_dir:&Path,extension:&str) -> Option<Vec<PathBuf>>  {
    let mut files = Vec::<PathBuf>::new();
    for entry in WalkDir::new(base_dir).into_iter().filter_map(|e| e.ok()) {
        match entry.path().extension() {
            Some(ext) => {
                match ext.to_str().unwrap() == extension {
                    true => {
                        files.push(entry.path().to_owned());
                    }
                    false => {}
                }
            }
            None => {}
        }
    }
    match files.len(){
        0 => None,
        _=> Some(files)
    }
}

/// fourier transform array of real floating points and return the resulting real magnitude.
/// fft shift=true will shift zero-frequency to the center of the array
pub fn fft_real_abs(real:&Vec<f32>,fftshift:bool) -> Vec<f32> {
    let n = real.len();
    let mut fft_planner = FftPlanner::<f32>::new();
    let fft = fft_planner.plan_fft_forward(n);
    let mut complex_tmp:Vec<Complex<f32>> = real.iter().map(|val| Complex::<f32>::new(*val, 0.0)).collect();
    fft.process(&mut complex_tmp);
    if fftshift {
        complex_tmp.rotate_right(n/2);
    }
    complex_tmp.iter().map(|complex_val| complex_val.norm()).collect()
}

pub fn normalize(real:&Vec<f32>) -> Vec<f32> {
    let abs_max = real
        .iter()
        .max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap())
        .unwrap();
    real.iter().map(|x| x/abs_max).collect()
}

/// get the index of the zero-crossing point of the real-valued vector
// pub fn fzero_idx(real:&Vec<f32>) -> Option<usize> {
//     let solver:Newton<f32> = Newton::new();
// }



//    let mut fft_planner = FftPlanner::<f32>::new();
//     let fft = fft_planner.plan_fft_forward(n);
//
//     vol.outer_iter_mut().for_each(|mut slice|{
//         slice.outer_iter_mut().for_each(|mut line|{
//             let mut temp = line.to_vec();
//             fft.process(&mut temp);

//fn is_hidden(entry: &DirEntry) -> bool {
//     entry.file_name()
//          .to_str()
//          .map(|s| s.starts_with("."))
//          .unwrap_or(false)
// }
//
// let walker = WalkDir::new("foo").into_iter();
// for entry in walker.filter_entry(|e| !is_hidden(e)) {
//     println!("{}", entry?.path().display());
// }
