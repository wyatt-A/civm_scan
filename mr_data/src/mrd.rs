use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use seq_lib::pulse_sequence::{AcqDims, MrdFormat, MrdToKspaceParams};
//use acquire::build::acq_dims;
use cs_table::cs_table::CSTable;
use byteorder::{LittleEndian,ByteOrder};
use ndarray::{s, Array3, Array4, Array6, Order, Dim, ArrayD, IxDyn, concatenate, Ix, Array2, Ix6};
use ndarray::{Array, ArrayView, array, Axis};
use ndarray::iter::Axes;
use ndarray::Order::RowMajor;
use num_complex::Complex;
use crate::cfl;


const OFFSET_TO_DATA:usize = 512;
const HEADER_SIZE:usize = 256;
const CHARCODE_BYTES:Range<usize> = 18..20;
const N_READ_BYTES:Range<usize> = 0..4;
const N_PHASE_1_BYTES:Range<usize> = 4..8;
const N_PHASE_2_BYTES:Range<usize> = 8..12;
const N_SLICE_BYTES:Range<usize> = 12..16;
const N_ECHOS_BYTES:Range<usize> = 152..156;
const N_EXPERIMENT_BYTES:Range<usize> = 156..160;



#[test]
fn test(){
    let mrd = Path::new("/Users/Wyatt/scratch/N60187.work/N60187_m00/resources/m00.mrd");

    let cs_table = Path::new("/Users/Wyatt/scratch/N60187.work/N60187_m00/resources/cs_table");

    let p = MrdToKspaceParams::from_file(Path::new("/Users/Wyatt/scratch/N60187.work/N60187_m00/resources/mrd_to_kspace.mtk"));

    let vol = fse_raw_to_vol(mrd,cs_table,&p);
    cfl::to_nifti(&vol,&mrd.with_file_name("m00_ksapce.nii"));
}


pub fn cs_mrd_to_kspace(mrd:&Path,cs_table:&Path,cfl_base:&Path,params:&MrdToKspaceParams) {
    match params.mrd_format {
        MrdFormat::FseCSVol => fse_raw_to_cfl(mrd,cs_table,cfl_base,params),
        MrdFormat::StandardCSVol => se_raw_to_vol(mrd,cs_table,cfl_base,params),
        _=> panic!("not yet implemented")
    }
}

pub fn mrd_to_2d_image(mrd:&Path) -> Array2<f32> {
    let raw = MRData::new(mrd);
    let arr = raw.complex_array();
    let s = arr.slice(s![0,0,0,0,..,..]);
    cfl::kspace2d_to_image(&s.to_owned())
}


pub fn fse_raw_to_cfl(mrd:&Path,cs_table:&Path,cfl_base:&Path,params:&MrdToKspaceParams) {
    let vol = fse_raw_to_vol(mrd,cs_table,params);
    cfl::write_cfl_vol(&vol,cfl_base);
}

pub fn fse_raw_to_vol(mrd:&Path,cs_table:&Path,params:&MrdToKspaceParams) -> Array3<Complex<f32>> {
    let formatted = format_fse_raw(mrd,params.n_read,params.n_views,params.dummy_excitations);
    zero_fill(&formatted,cs_table,(params.n_read,params.n_phase1,params.n_phase2),params.dummy_excitations,params.view_acceleration)
}

pub fn se_raw_to_vol(mrd:&Path,cs_table:&Path,cfl_out_base_name:&Path,params:&MrdToKspaceParams){
    let formatted = format_multi_echo_raw(mrd,params.n_read,params.n_views,params.dummy_excitations,0);
    let vol = zero_fill(&formatted,cs_table,(params.n_read,params.n_phase1,params.n_phase2),params.dummy_excitations,params.view_acceleration);
    cfl::write_cfl_vol(&vol,cfl_out_base_name);
}

fn multi_echo_raw_to_cfl(mrd:&Path,cs_table:&Path,cfl_out_base_name:&Path,params:&MrdToKspaceParams) {
    let fname = cfl_out_base_name.file_name().expect(&format!("cannot determine base name from {:?}",cfl_out_base_name)).to_str().unwrap();
    let n = params.n_objects;
    let w = ((n-1) as f32).log10().floor() as usize + 1;
    let formatter = |index:usize| format!("m{:0width$ }",index,width=w);
    for i in 0..n {
        let postfix = formatter(i);
        let qualified_name = format!("{}_{}",fname,postfix);
        let cfl = cfl_out_base_name.with_file_name(qualified_name);
        let formatted = format_multi_echo_raw(mrd,params.n_read,params.n_views,params.dummy_excitations,i);
        let vol = zero_fill(&formatted,cs_table,(params.n_read,params.n_phase1,params.n_phase2),params.dummy_excitations,params.view_acceleration);
        cfl::write_cfl_vol(&vol,&cfl);
    }
}


fn format_fse_raw(mrd:&Path,n_read:usize,n_views:usize,n_dummy_excitations:usize) -> Array2::<Complex<f32>> {
    let mrd = MRData::new(mrd);
    let mut mrd_dims = mrd.char_dim_array().to_vec();
    mrd_dims.reverse();
    let mrd_array = ArrayD::<f32>::from_shape_vec(IxDyn(&mrd_dims), mrd.float_stream()).expect("unexpected number of samples");
    let echo1 = mrd_array.slice(s![..,0,..,..,..,..,..]);
    let mut echo2 = mrd_array.slice(s![..,1,..,..,..,..,..]).to_owned();
    let echo3 = mrd_array.slice(s![..,2,..,..,..,..,..]);
    echo2 += &echo3;
    let combined_echos = concatenate(Axis(1), &[echo1, echo2.view()]).expect("unable to concatenate arrays").permuted_axes([0,3,2,1,4,5]);
    let trimmed = combined_echos.slice(s![..,n_dummy_excitations..,..,..,..,..]).to_owned();
    let trimmed = trimmed.to_shape(((n_views,n_read,2), Order::RowMajor)).expect("cannot reshape array").to_owned();

    let mut cf = Array2::<Complex<f32>>::zeros((n_views,n_read));
    trimmed.outer_iter().enumerate().for_each(|(i,read)|{
        read.outer_iter().enumerate().for_each(|(j,sample)|{
            cf[[i,j]] = Complex::new(sample[0],sample[1])
        })
    });
    cf
}


fn format_multi_echo_raw(mrd:&Path,n_read:usize,n_views:usize,n_dummy_excitations:usize,vol_index:usize) -> Array2::<Complex<f32>> {
    let mrd = MRData::new(mrd);
    let mut mrd_dims = mrd.char_dim_array().to_vec();
    mrd_dims.reverse();
    let mrd_array = ArrayD::<f32>::from_shape_vec(IxDyn(&mrd_dims), mrd.float_stream()).expect("unexpected number of samples");
    let echo = mrd_array.slice(s![..,vol_index,..,..,..,..,..]).to_owned().permuted_axes([0,3,2,1,4,5]);
    let trimmed = echo.slice(s![..,n_dummy_excitations..,..,..,..,..]).to_owned();
    let trimmed = trimmed.to_shape(((n_views,n_read,2), Order::RowMajor)).expect("cannot reshape array").to_owned();
    let mut cf = Array2::<Complex<f32>>::zeros((n_views,n_read));
    trimmed.outer_iter().enumerate().for_each(|(i,read)|{
        read.outer_iter().enumerate().for_each(|(j,sample)|{
            cf[[i,j]] = Complex::new(sample[0],sample[1])
        })
    });
    cf
}


fn zero_fill(array:&Array2::<Complex<f32>>,
             cs_table:&Path,
             dims:(usize,usize,usize),
             dummy_excitations:usize,
             view_acceleration:usize) ->  Array3::<Complex<f32>>{
    let cs_table = CSTable::open(cs_table,dims.1 as i16,dims.2 as i16);
    let mut zf_arr = Array4::<f32>::zeros([dims.2,dims.1,dims.0,2]);
    let mut zf_arr = Array3::<Complex<f32>>::zeros([dims.2,dims.1,dims.0]);

    let indices = cs_table.indices(dummy_excitations*view_acceleration);
    // scan the indices to make sure non are out of range.

    // the min index must be 0, and the max must be dim - 1 (if they are off by one we will attempt a correction with an offset)
    let mut offset = (0,0);
    for index in indices.iter(){
        if index.0 as usize == dims.2 {
            offset.0 = -1;
        }
        if index.1 as usize == dims.1 {
            offset.1 = -1;
        }
        if index.0 < 0 || index.1 < 0 {
            panic!("this cs table is producing negative matrix indices! Please fix it!");
        }
    }

    for (i,index) in cs_table.indices(dummy_excitations*view_acceleration).iter().enumerate() {
        let mut zf_slice = zf_arr.slice_mut(s![(index.0+offset.0) as usize,(index.1+offset.1) as usize,..]);
        zf_slice += &array.slice(s![i,..]);
    }
    zf_arr
}




pub struct MRData {
    file_path:PathBuf,
}

impl MRData {

    pub fn new(file_path:&Path) -> Self{
        match file_path.exists(){
            true => Self {
                file_path:file_path.to_owned()
            },
            false => panic!("file doesn't exist {:?}",file_path)
        }
    }

    fn open(&self) -> File {
        File::open(&self.file_path).expect("cannot open file")
    }


    fn header(&self) -> [u8;HEADER_SIZE] {
        let mut f = self.open();
        let mut header = [0;HEADER_SIZE];
        f.read_exact(&mut header).expect("trouble reading header");
        header
    }

    pub fn n_read(&self) -> i32 {
        self.dimensions().n_read.clone()
    }

    pub fn n_phase1(&self) -> i32 {
        self.dimensions().n_phase1.clone()
    }

    pub fn n_phase2(&self) -> i32 {self.dimensions().n_phase2.clone()}

    pub fn n_slice(&self) -> i32 {
        self.dimensions().n_slices.clone()
    }

    pub fn n_views(&self) -> i32 {
        self.n_phase1()*self.n_phase2()
    }

    pub fn n_echos(&self) -> i32 {
        self.dimensions().n_echos.clone()
    }

    pub fn n_experiments(&self) -> i32 {
        self.dimensions().n_experiments.clone()
    }

    pub fn n_samples(&self) -> i32 {
        self.n_read()*self.n_views()*self.n_echos()*self.n_experiments()
    }


    fn dimensions(&self) -> AcqDims {
        let h = self.header();
        AcqDims {
            n_read:bytes_to_long(&h[N_READ_BYTES]),
            n_phase1:bytes_to_long(&h[N_PHASE_1_BYTES]),
            n_phase2:bytes_to_long(&h[N_PHASE_2_BYTES]),
            n_slices:bytes_to_long(&h[N_SLICE_BYTES]),
            n_echos:bytes_to_long(&h[N_ECHOS_BYTES]),
            n_experiments:bytes_to_long(&h[N_EXPERIMENT_BYTES]),
        }
    }

    pub fn char_dim_array(&self) -> [usize;7] {
        let dims = self.dimensions();
        match self.is_complex(){
            true => {
                [2,dims.n_read as usize,dims.n_phase1 as usize,
                 dims.n_phase2 as usize,dims.n_slices as usize,
                 dims.n_echos as usize,dims.n_experiments as usize]
            }
            false =>{
                [1,dims.n_read as usize,dims.n_phase1 as usize,
                     dims.n_phase2 as usize,dims.n_slices as usize,
                     dims.n_echos as usize,dims.n_experiments as usize]
            }
        }
    }

    pub fn complex_dims(&self) -> [usize;6] {
        let dims = self.dimensions();
        [dims.n_read as usize,dims.n_phase1 as usize,
        dims.n_phase2 as usize,dims.n_slices as usize,
        dims.n_echos as usize,dims.n_experiments as usize]
    }

    pub fn byte_stream(&self) -> Vec<u8> {
        let mut f = File::open(&self.file_path).expect("cannot open file");
        let mut reader = BufReader::new(&mut f);
        reader.seek(SeekFrom::Start(OFFSET_TO_DATA as u64)).expect("cannot seek to data proper");
        let mut raw:Vec<u8> = vec![0;self.n_data_bytes()];
        reader.read_exact(&mut raw).expect("a problem occurred reading mrd data");
        raw
    }

    pub fn complex_stream(&self) -> Vec<Complex<f32>> {
        let f = self.float_stream();
        let mut c = Vec::<Complex<f32>>::with_capacity(f.len()/2);
        for i in 0..f.len()/2 {
            c.push(Complex::<f32>::new(f[2*i],f[2*i+1]));
        }
        c
    }

    pub fn complex_array(&self) -> Array6<Complex<f32>> {
        let mut dims = self.complex_dims();
        dims.reverse();
        Array6::<Complex<f32>>::from_shape_vec(dims, self.complex_stream()).expect("unexpected number of samples")
    }

    pub fn float_stream(&self) -> Vec<f32> {
        if self.bit_depth() != 4 {
            panic!("mrd data type is incompatible with floating point. Bit depth is {}",self.bit_depth());
        }
        let mut floats:Vec<f32> = vec![0.0;self.n_chars()];
        LittleEndian::read_f32_into(&self.byte_stream(),&mut floats);
        floats
    }

    fn n_data_bytes(&self) -> usize {
        self.n_chars()*self.bit_depth() as usize
    }

    fn n_chars(&self) -> usize {
        match self.is_complex(){
            true => 2 * self.n_samples() as usize,
            false => self.n_samples() as usize
        }
    }

    fn character_code(&self) -> i16 {
        let h = self.header();
        bytes_to_int(&h[CHARCODE_BYTES])
    }

    fn is_complex(&self) -> bool {
        let code = self.character_code();
        if code >= 16 {true} else {false}
    }

    fn bit_depth(&self) -> u16 {
        /* Determine data type from header (charcode) */
        let mut code = self.character_code();
        let is_complex = if code >= 16 {true} else {false};
        if is_complex {code -= 16};
        let bit_depth:u16 = match code{
            0 | 1 => 1,
            2 | 3 => 2,
            4 | 5 => 4,
            6 => 8,
            _ => panic!("problem determining character bytes. mrd may be corrupt"),
        };
        if bit_depth != 4 || !is_complex {panic!("only complex floats are supported for now")};
        bit_depth
    }

}

fn bytes_to_long(byte_slice:&[u8]) -> i32 {
    let mut buff = [0;4];
    buff.copy_from_slice(&byte_slice);
    i32::from_le_bytes(buff)
}

fn bytes_to_int(byte_slice:&[u8]) -> i16 {
    let mut buff = [0;2];
    buff.copy_from_slice(&byte_slice);
    i16::from_le_bytes(buff)
}


fn read_to_string(file_path:&Path) -> String {
    let mut f = File::open(file_path).expect("cannot open file");
    let mut s = String::new();
    f.read_to_string(&mut s).expect("cannot read from file");
    s
}





