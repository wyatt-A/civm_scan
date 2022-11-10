use crate::utils;
use std::{collections::HashMap, hash};
use std::path::{Path,PathBuf};
use std::fs::{File};
use std::io::{Read,Write};
use byteorder::{ByteOrder,BigEndian,LittleEndian};
use ndarray::{s,Array3,Array4,Order,Axis};

pub fn get_dims(path:&Path) -> Vec<usize>{
    let p = path.to_str().unwrap();
    let h = load_cfl_header(p);
    let d = h.get("# Dimensions").expect("Couldn't find # dimesions").to_owned();
    let dim_str:Vec<&str> = d.split_whitespace().collect();
    let dims:Vec<usize> = dim_str.iter().flat_map(|str| str.to_string().parse()).collect();
    let non_singleton:Vec<usize> = dims.into_iter().filter(|dimension| *dimension != 1).collect();
    return non_singleton;
}

pub fn load_cfl_header(path:&str) -> HashMap<String,String>{
    let s = utils::read_to_string(path, "hdr").expect("cannot open file");
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

pub fn to_civm_raw_u16(cfl:&Path,output_dir:&Path,volume_label:&str,raw_prefix:&str,scale:f32){
    let dims = get_dims(cfl);
    if dims.len() !=3 {panic!("we don't know how to write data {}-D data!",dims.len())}
    let mag = Array3::from_shape_vec((dims[2],dims[1],dims[0]),to_magnitude(cfl)).expect("raw floats cannot fit into shape");
    let numel_per_img = dims[1]*dims[0];
    let mut byte_buff:Vec<u8> = vec![0;2*numel_per_img];
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
    let p = cfl.with_extension("cfl");
    let mut f = File::open(p).expect("cannot open file");
    let mut buf = Vec::<u8>::new();
    f.read_to_end(&mut buf).expect("trouble reading file");
    let mut fbuf:Vec<f32> = vec![0.0;buf.len()/4];
    LittleEndian::read_f32_into(&buf,&mut fbuf);
    return fbuf;
}

pub fn to_magnitude(cfl:&Path) -> Vec<f32>{
    let dims = get_dims(cfl);
    let mut complex = Array4::from_shape_vec((dims[2],dims[1],dims[0],2),load(cfl)).expect("cannot fit data vector in ndarray");
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

#[test]
fn test_write(){
    let cfl = Path::new("C:\\Users\\waust\\OneDrive\\Desktop\\cs_reco\\test_data\\220816T11_m00_imspace.cfl");
    let out = Path::new("./home");
    let label = "test_runno_m00";
    let raw_prefix = "t9imx";
    let scale = find_u16_scale(cfl,0.999500);
    to_civm_raw_u16(&cfl,&out,label,raw_prefix,scale);
}