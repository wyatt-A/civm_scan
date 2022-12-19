use std::env::current_dir;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Write, Read};
use std::time::SystemTime;
use glob::glob;
use walkdir::WalkDir;
use rustfft::{FftPlanner};
use num_complex::Complex;
use chrono::{DateTime,Local};
use clean_path::{Clean};


pub fn absolute_path(path:&Path) -> PathBuf {
    match path.is_absolute() {
        true => path.clean().to_owned(),
        false => current_dir().expect("unable to get current directory").join(path).clean()
    }
}

pub fn date_stamp() -> String {
    let datetime: DateTime<Local> = SystemTime::now().into();
    format!("{}",datetime.format("%Y%m%d"))
}

pub fn time_stamp() -> String {
    let datetime: DateTime<Local> = SystemTime::now().into();
    format!("{}",datetime.format("%Y%m%d:%T"))
}


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

pub fn write_to_file(filepath:&Path,extension:&str,string:&str) -> PathBuf{
    let p = filepath.with_extension(extension);
    let mut f = File::create(&p).expect("failed to create file");
    f.write_all(string.as_bytes()).expect("trouble writing to file");
    p.to_owned()
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
pub fn find_files(base_dir:&Path,extension:&str,sort:bool) -> Option<Vec<PathBuf>>  {
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
    if sort {
        files.sort();
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

pub fn real_to_complex(x:&Vec<f32>) -> Vec<Complex<f32>> {
    x.iter().map(|real| Complex::new(*real,0.0)).collect()
}

pub fn complex_to_real(x:&Vec<Complex<f32>>) -> Vec<f32> {
    x.iter().map(|c| c.re).collect()
}

pub fn complex_to_imaginary(x:&Vec<Complex<f32>>) -> Vec<f32> {
    x.iter().map(|c| c.im).collect()
}

pub fn complex_to_norm(x:&Vec<Complex<f32>>) -> Vec<f32> {
    x.iter().map(|c| c.norm()).collect()
}

pub fn fft(x:&Vec<Complex<f32>>,n:usize) -> Vec<Complex<f32>> {
    let mut fft_planner = FftPlanner::<f32>::new();
    let fft = fft_planner.plan_fft_forward(n);
    let mut buff = x.clone();
    fft.process(&mut buff);
    buff
}

pub fn fft_shift(x:&Vec<Complex<f32>>) -> Vec<Complex<f32>> {
    let n = x.len();
    let mut r = x.clone();
    r.rotate_right(n/2);
    r
}

pub fn freq_spectrum(x:&Vec<Complex<f32>>,n:usize) -> Vec<f32> {

    let mut buff = x.clone();

    let lenx = x.len();
    if n > lenx {
        buff.extend(real_to_complex(&vec![0.0;n-lenx]))
    } else if lenx < n {
        buff = buff[0..n].to_vec()
    }

    let y = fft(&buff,n);
    // normalize values based on length of transform
    y.iter().map(|val| val.scale(1.0/n as f32).norm()).collect()
}

pub fn freq_axis(sample_period:f32,n:usize) -> Vec<f32> {
    let fs = 1.0/sample_period;
    (0..n/2+1).map(|s| fs * s as f32 / n as f32).collect()
}

pub fn abs(x:&Vec<f32>) -> Vec<f32> {
    x.iter().map(|val| val.abs()).collect()
}

pub fn complex_abs(x:&Vec<Complex<f32>>) -> Vec<f32> {
    x.iter().map(|c| c.norm()).collect()
}

pub fn arg_max(x:&Vec<f32>) -> usize {
    let mut max_temp = 0.0;
    let mut max_ind = 0;
    for i in 0..x.len(){
        if x[i] > max_temp {
            max_temp = x[i];
            max_ind = i;
        }
    }
    max_ind
}

pub fn max(x:&Vec<f32>) -> f32 {
    let mut max_temp = 0.0;
    for i in 0..x.len(){
        if x[i] > max_temp {
            max_temp = x[i];
        }
    }
    max_temp
}

pub fn arg_min(x:&Vec<f32>) -> usize {
    let mut min_temp = f32::MAX;
    let mut min_ind = 0;
    for i in 0..x.len(){
        if x[i] < min_temp {
            min_temp = x[i];
            min_ind = i;
        }
    }
    min_ind
}



// bandwidth of a finite time domain signal (used for slice thickness calculations)
// result is in hertz
pub fn bandwidth(time_domain_signal:&Vec<Complex<f32>>,dt:f32) -> f32 {
    let n_fft_samples = 65536;

    let spec = freq_spectrum(time_domain_signal,n_fft_samples);
    let spec = normalize(&spec);
    let axis = freq_axis(dt,n_fft_samples);

    // find where spec drops below 0.5 for full-width at half-max
    let mut f_index = 0;
    for i in 0..n_fft_samples-1 {
        if spec[i] >= 0.5 && spec[i+1] < 0.5 {
            f_index = i;
            break
        }
        if i >= axis.len(){
            panic!("error finding full width at half max")
        }
    }
    // full bandwidth of time domain signal
    axis[f_index] + axis[f_index+1]
}


pub fn trapz(x:&Vec<f32>,dt:Option<f32>) -> f32 {
    let mut s = 0.0;
    for i in 0..x.len()-1{
        s = s + x[i] + x[i+1];
    }
    dt.unwrap_or(1.0)*s/2.0
}
