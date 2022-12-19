use std::{collections::HashMap, hash};
use std::path::{Path,PathBuf};
use std::fs::{create_dir_all, File};
use std::io::{Read,Write};
use std::process::Command;
use byteorder::{ByteOrder,BigEndian,LittleEndian};
use ndarray::{s, Array3, Array4, Order, Dim, ArrayD, IxDyn, concatenate, Ix, ArrayViewMut, OwnedRepr, Ix3, ArrayBase, AssignElem, Array2};
use ndarray::{Array, ArrayView, array, Axis};
use ndarray::iter::Axes;
use ndarray::Order::RowMajor;
use serde::{Deserialize, Serialize};
use rustfft::{FftPlanner};
use serde_json;
use utils;
use num_complex::Complex;
//use nifti::{NiftiObject, ReaderOptions, NiftiVolume};
use nifti::writer::WriterOptions;


#[derive(Serialize,Deserialize)]
pub struct ImageScale {
    histogram_percent:f32,
    pub scale_factor:f32
}

impl ImageScale {
    pub fn new(histogram_percent:f32,scale_factor:f32) -> Self {
        Self {
            histogram_percent,
            scale_factor
        }
    }
    pub fn from_file(file_path:&Path) -> Self {
        let mut f = File::open(file_path).expect("cannot open file");
        let mut s = String::new();
        f.read_to_string(&mut s).expect("cannot read from file");
        serde_json::from_str(&s).expect("cannot deserialize file")
    }
    pub fn to_file(&self,file_path:&Path) {
        let s = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        let mut f = File::create(file_path).expect("cannot create file");
        f.write_all(s.as_bytes()).expect("cannot write to file");
    }
}

pub fn get_dims(path:&Path) -> Vec<usize>{
    let h = load_cfl_header(path);
    let d = h.get("# Dimensions").expect("Couldn't find # dimesions").to_owned();
    let dim_str:Vec<&str> = d.split_whitespace().collect();
    let dims:Vec<usize> = dim_str.iter().flat_map(|str| str.to_string().parse()).collect();
    let non_singleton:Vec<usize> = dims.into_iter().filter(|dimension| *dimension != 1).collect();
    return non_singleton;
}

pub fn load_cfl_header(cfl_base:&Path) -> HashMap<String,String>{
    let (hdr,_) = cfl_base_decode(cfl_base);
    let mut f = File::open(&hdr).expect(&format!("cannot open file {:?}",hdr));
    let mut s = String::new();
    f.read_to_string(&mut s).expect("cannot read from file");
    //let s = utils::read_to_string(path,"hdr").expect("cannot open file");
    let mut h = HashMap::<String,String>::new();
    let lines:Vec<&str> = s.lines().collect();
    lines.iter().enumerate().for_each( |(i,line)|
    {
        if line.starts_with("#"){
            let key = line.to_owned().to_string();
            h.insert(key,lines[i+1].to_string());
        }
    });
    return h;
}

pub fn write_cfl_header(vol:&Array3<Complex<f32>>,cfl_base:&Path) {
    let shape = vol.shape();
    let (hdr,_) = cfl_base_decode(cfl_base);
    let mut hdr = File::create(hdr).expect("cannot create file");
    let hdr_str = format!("# Dimensions\n{} {} {} 1 1",shape[2],shape[1],shape[0]);
    hdr.write_all(hdr_str.as_bytes()).expect("a problem occurred writing to cfl header");
}

pub fn to_civm_raw_u16(cfl_base:&Path, output_dir:&Path, volume_label:&str, raw_prefix:&str, scale:f32, axis_inversion: (bool, bool, bool)){
    let (hdr,_) = cfl_base_decode(cfl_base);
    if !output_dir.exists(){
        create_dir_all(output_dir).expect("cannot crate output image directory");
    }
    let dims = get_dims(&hdr);
    if dims.len() !=3 {panic!("we don't know how to write data {}-D data!",dims.len())}
    let mut mag = Array3::from_shape_vec((dims[2],dims[1],dims[0]),to_magnitude(cfl_base)).expect("raw floats cannot fit into shape");

    if axis_inversion.0 {
        mag.invert_axis(Axis(0));
    }

    if axis_inversion.1 {
        mag.invert_axis(Axis(1));
    }

    if axis_inversion.2 {
        mag.invert_axis(Axis(2));
    }

    let numel_per_img = dims[1]*dims[0];
    let mut byte_buff:Vec<u8> = vec![0;2*numel_per_img];
    println!("writing to civm_raw ...");
    for i in 0..dims[1] {
        let slice = mag.slice(s![..,i,..]);
        let flat = slice.to_shape((numel_per_img,Order::RowMajor)).expect("unexpected data size");
        let v = flat.to_vec();
        let uints:Vec<u16> = v.iter().map(|float| (*float*scale) as u16).collect();
        let fname = output_dir.join(&format!("{}{}.{:03}.raw",volume_label,raw_prefix,i+1));
        let mut f = File::create(fname).expect("trouble creating file");
        BigEndian::write_u16_into(&uints,&mut byte_buff);
        f.write_all(&mut byte_buff).expect("touble writing to file");
    }
}

pub fn load(cfl:&Path) -> Vec<f32>{
    let mut f = File::open(cfl).expect("cannot open file");
    let mut buf = Vec::<u8>::new();
    f.read_to_end(&mut buf).expect("trouble reading file");
    let mut fbuf:Vec<f32> = vec![0.0;buf.len()/4];
    LittleEndian::read_f32_into(&buf,&mut fbuf);
    return fbuf;
}

pub fn to_magnitude(cfl:&Path) -> Vec<f32>{
    let (hdr,cfl) = cfl_base_decode(cfl);
    let dims = get_dims(&hdr);
    let mut complex = Array4::from_shape_vec((dims[2],dims[1],dims[0],2),load(&cfl)).expect("cannot fit data vector in ndarray");
    let square = |x:&mut f32| *x = (*x).powi(2);
    // magnitude is calculated from complex values
    // "square root of the sum of the squares"
    complex.slice_mut(s![..,..,..,0]).map_inplace(square);
    complex.slice_mut(s![..,..,..,1]).map_inplace(square);
    let mut mag = complex.sum_axis(Axis(3));
    mag.mapv_inplace(f32::sqrt);
    let f = mag.to_shape((dims[2]*dims[1]*dims[0],Order::RowMajor)).expect("cannot flatten array");
    return f.to_vec();
}

pub fn find_u16_scale(cfl:&Path,histo_percent:f64) -> f32{
    let mag = to_magnitude(cfl);
    return u16_scale_from_vec(&mag,histo_percent);
}

pub fn write_u16_scale(cfl:&Path,histo_percent:f64,output_file:&Path){
    let scale = find_u16_scale(cfl,histo_percent);
    ImageScale::new(histo_percent as f32,scale).to_file(output_file);
}

// typical histo %: 0.999500
pub fn u16_scale_from_vec(magnitude_img:&Vec<f32>,histo_percent:f64) -> f32{
    let mut mag = magnitude_img.clone();
    println!("sorting image ...");
    mag.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // find scale factor as a float
    let n_voxels = mag.len();
    let n_to_saturate = (n_voxels as f64 * (1.0-histo_percent)).round() as usize;
    return 65535.0/mag[n_voxels - n_to_saturate + 1];
}
pub fn write_cfl_vol(complex_volume:&Array3<Complex<f32>>,cfl_base:&Path) {
    let flat = complex_vol_to_vec(complex_volume);
    write_data(&flat,cfl_base);
    write_cfl_header(complex_volume,cfl_base);
}

pub fn write_data(flat:&Vec<f32>, cfl_base:&Path) {
    let (_,cfl) = cfl_base_decode(cfl_base);
    let n_bytes = flat.len()*4;
    let mut byte_buff:Vec<u8> = vec![0;n_bytes];
    LittleEndian::write_f32_into(&flat,&mut byte_buff);
    let mut cfl = File::create(cfl).expect("cannot create file");
    cfl.write_all(&byte_buff).expect("problem writing to file");
}

fn to_complex_volume(cfl_base:&Path) -> Array3<Complex<f32>> {
    let (hdr,vol) = cfl_base_decode(cfl_base);
    let v = load(&vol);
    let dims = get_dims(&hdr);
    if dims.len() != 3 {
        panic!("cfl data must have 3 non-singleton dimensions");
    }
    let dims = (dims[0],dims[1],dims[2]);
    vec_to_complex_vol(&v,dims)
}

pub fn complex_vol_to_magnitude(vol:&Array3<Complex<f32>>) -> Array3<f32> {
    let shape = vol.shape();
    let vol = vol.to_shape((vol.len(),Order::RowMajor)).unwrap().to_vec();
    let mag:Vec<f32> = vol.iter().map(|complex_number| complex_number.norm()).collect();
    Array3::<f32>::from_shape_vec((shape[0],shape[1],shape[2]),mag).expect("cannot create array")
}

pub fn complex_slice_to_magnitude(slice:&Array2<Complex<f32>>) -> Array2<f32> {
    let shape = slice.shape();
    let slice = slice.to_shape((slice.len(),Order::RowMajor)).unwrap().to_vec();
    let mag:Vec<f32> = slice.iter().map(|complex_number| complex_number.norm()).collect();
    Array2::<f32>::from_shape_vec((shape[0],shape[1]),mag).expect("cannot create array")
}


pub fn from_complex_volume(vol:&Array3<Complex<f32>>,cfl_base:&Path) {
    let flat = complex_vol_to_vec(vol);
    let shape = vol.shape();
    let dims = (shape[2],shape[1],shape[0]);
    write_data(&flat, cfl_base);
    write_cfl_header(vol,cfl_base);
}

pub fn to_nifti(vol:&Array3<Complex<f32>>,nifti_base:&Path) {
    let mag_vol = complex_vol_to_magnitude(vol);
    let nii = WriterOptions::new(nifti_base);
    nii.write_nifti(&mag_vol).expect("trouble writing to nifti");
}

fn vec_to_complex_vol(flat:&Vec<f32>,dims:(usize,usize,usize)) -> Array3<Complex<f32>> {
    let complex_arr:Vec::<Complex<f32>> = (0..flat.len()/2).map(|i| Complex::new(flat[2*i],flat[2*i+1])).collect();
    Array3::<Complex<f32>>::from_shape_vec((dims.2,dims.1,dims.0),complex_arr).expect(&format!("cannot coerce cfl raw data to shape {:?}",dims))
}

fn complex_vol_to_vec(vol:&Array3<Complex<f32>>) -> Vec<f32> {
    let dims = vol.shape();
    let numel = dims[0]*dims[1]*dims[2];
    let flat:Vec<Complex<f32>> = vol.to_shape((numel,Order::RowMajor)).expect("cannot flatten array").to_vec();
    let mut cfl_flat:Vec<f32> = vec![0.0;numel*2];
    flat.iter().enumerate().for_each(|(i,c_val)|{
        cfl_flat[2*i] = c_val.re;
        cfl_flat[2*i+1] = c_val.im;
    });
    cfl_flat
}

pub fn fft3_axis(vol:Array3<Complex<f32>>, axis:usize,fftshift:bool) -> Array3<Complex<f32>> {
    let process_order = match axis {
        0 => ([2,1,0],[2,1,0]),
        1 => ([2,0,1],[1,2,0]),
        2 => ([1,0,2],[1,0,2]),
        _=> panic!("axis is out of range. must not be greater than 2")
    };

    // permute axes to ensure the correct dimension is getting the transform
    let mut vol = vol.permuted_axes(process_order.0);

    let n = vol.shape()[2];

    let mut fft_planner = FftPlanner::<f32>::new();
    let fft = fft_planner.plan_fft_forward(n);

    vol.outer_iter_mut().for_each(|mut slice|{
        slice.outer_iter_mut().for_each(|mut line|{
            let mut temp = line.to_vec();
            fft.process(&mut temp);
            // normalize the result
            temp.iter_mut().for_each(|e| *e /= (n as f32).sqrt());
            if fftshift {
                temp.rotate_right(n/2);
            }
            // assign temp back to line
            line.assign(&Array::from_vec(temp));
        })
    });
    // permute axes back to be consistent with input
    let vol = vol.permuted_axes(process_order.1);
    return vol;
}

pub fn fft2(slice:&Array2<Complex<f32>>,fftshift:bool) -> Array2<Complex<f32>> {
    let mut slice = slice.clone();
    let mut shape = slice.shape().to_owned();
    shape.reverse();
    let mut fft_planner = FftPlanner::<f32>::new();
    for axis in 0..2 {
        let fft = fft_planner.plan_fft_forward(shape[axis]);
        for mut line in slice.axis_iter_mut(Axis(axis)){
            let mut temp = line.to_vec();
            let n = temp.len();
            fft.process(&mut temp);
            // normalize the result
            temp.iter_mut().for_each(|e| *e /= (n as f32).sqrt());
            if fftshift {
                temp.rotate_right(n/2);
            }
            // assign temp back to line
            line.assign(&Array::from_vec(temp));
        }
    }
    slice
}

pub fn kspace2d_to_image(slice:&Array2<Complex<f32>>) -> Array2<f32> {
    complex_slice_to_magnitude(&fft2(slice,true))
}

pub fn fermi_filter_image(cfl_base_in:&Path,cfl_base_out:&Path,w1:f32,w2:f32) {
    let img_vol = to_complex_volume(cfl_base_in);
    let mut k = fft3_axis(fft3_axis(fft3_axis(img_vol,0,true),1,true),2,true);
    _fermi_filter(&mut k,w1,w2);
    let img_filt = fft3_axis(fft3_axis(fft3_axis(k,0,false),1,false),2,false);
    write_cfl_vol(&img_filt,cfl_base_out);
}

fn cfl_base_decode(cfl_base:&Path) -> (PathBuf,PathBuf) {
    (cfl_base.with_extension("hdr"),cfl_base.with_extension("cfl"))
}

fn _fermi_filter(vol:&mut Array3<Complex<f32>>,w1:f32,w2:f32) -> &mut Array3<Complex<f32>> {
    let dims = vol.shape();
    let max_dim = dims.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).expect("dimension vector is empty").clone() as f32;
    let fermi_t = max_dim*w1/2.0;
    let fermi_u = max_dim*w2/2.0;
    // the norm_factor ensures that the filter coefficients do not exceed 1.
    // this approach requires less computation and memory than the previous approach
    //todo!(maybe figure out how to do this multi-threaded)
    let norm_factor = 1.0+(-fermi_u/fermi_t).exp();

    let dx = dims[0];
    let dy = dims[1];
    let dz = dims[2];

    let x_n = (dx as f32/max_dim).powi(2);
    let y_n = (dy as f32/max_dim).powi(2);
    let z_n = (dz as f32/max_dim).powi(2);

    vol.outer_iter_mut().enumerate().for_each(|(x_i,mut slice)|{
        slice.outer_iter_mut().enumerate().for_each(|(y_i,mut line)| {
            line.iter_mut().enumerate().for_each(|(z_i,mut sample)|{
                let x_c_sq = ((x_i as f32) - (dx/2) as f32).powi(2);
                let y_c_sq = ((y_i as f32) - (dy/2) as f32).powi(2);
                let z_c_sq = ((z_i as f32) - (dz/2) as f32).powi(2);
                let k_radius = (x_c_sq/x_n + y_c_sq/y_n + z_c_sq/z_n).sqrt();
                let filt_param = (k_radius - fermi_u)/fermi_t;
                let coeff = 1.0/(1.0 + filt_param.exp());
                *sample *= coeff*norm_factor;
            })
        })
    });
    vol
}
