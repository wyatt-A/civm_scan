use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use seq_lib::pulse_sequence::{AcqDims, MrdFormat, MrdToKspaceParams};
use acquire::build::acq_dims;
use cs_table::cs_table::CSTable;
use byteorder::{LittleEndian,ByteOrder};
use ndarray::{s, Array3, Array4, Order, Dim, ArrayD, IxDyn, concatenate, Ix};
use ndarray::{Array, ArrayView, array, Axis};
use ndarray::iter::Axes;
use ndarray::Order::RowMajor;


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

    // FSE kspace formatting

    let ksp_fse = MrdToKspaceParams {
        mrd_format: MrdFormat::FseCSVol,
        n_read:788,
        n_phase1:480,
        n_phase2:480,
        n_views:28800,
        view_acceleration:2,
        dummy_excitations:20,
        n_objects:1
    };
    let mrd_path = Path::new("/Users/Wyatt/IdeaProjects/test_data/acq/m01/m01.mrd");
    let table = Path::new("/Users/Wyatt/IdeaProjects/test_data/acq/m01/cs_table");
    let out_base = Path::new("/Users/Wyatt/IdeaProjects/test_data/acq/m01/mgre_ksp");
    fse_raw_to_cfl(mrd_path,table,out_base,&ksp_fse);


    // Single echo/multi echo formatting

/*
    let ksp = RawToKspaceParams {
        n_read:788,
        n_phase1:480,
        n_phase2:480,
        n_views:28800,
        view_acceleration:1,
        dummy_excitations:0,
        n_vols:4
    };

    let data_path = Path::new("/Users/Wyatt/IdeaProjects/test_data/mgre/mgre.mrd");
    let table = Path::new("/Users/Wyatt/IdeaProjects/test_data/se/stream_CS480_8x_pa18_pb54");
    let out_base = Path::new("/Users/Wyatt/IdeaProjects/test_data/mgre/mgre_ksp");
    multi_echo_raw_to_cfl(data_path,table,out_base,&ksp)
*/
}


pub fn cs_mrd_to_kspace(mrd:&Path,cs_table:&Path,cfl_base:&Path,params:&MrdToKspaceParams) {
    match params.mrd_format {
        MrdFormat::FseCSVol => fse_raw_to_cfl(mrd,cs_table,cfl_base,params),
        MrdFormat::StandardCSVol => multi_echo_raw_to_cfl(mrd,cs_table,cfl_base,params),
        _=> panic!("not yet implemented")
    }
}


pub fn fse_raw_to_cfl(mrd:&Path,cs_table:&Path,cfl_out:&Path,params:&MrdToKspaceParams) {
    let formatted = format_fse_raw(mrd,params.n_read,params.n_views,params.dummy_excitations);
    let vol = zero_fill(&formatted,cs_table,(params.n_read,params.n_phase1,params.n_phase2),params.dummy_excitations,params.view_acceleration);
    write_cfl_vol(&vol,cfl_out);
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
        write_cfl_vol(&vol,&cfl);
    }
}


fn format_fse_raw(mrd:&Path,n_read:usize,n_views:usize,n_dummy_excitations:usize) -> Array<f32, Dim<[Ix; 3]>> {
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
    trimmed.to_shape(((n_views,n_read,2), Order::RowMajor)).expect("cannot reshape array").to_owned()
}


fn format_multi_echo_raw(mrd:&Path,n_read:usize,n_views:usize,n_dummy_excitations:usize,vol_index:usize) -> Array<f32, Dim<[Ix; 3]>> {
    let mrd = MRData::new(mrd);
    let mut mrd_dims = mrd.char_dim_array().to_vec();
    mrd_dims.reverse();
    let mrd_array = ArrayD::<f32>::from_shape_vec(IxDyn(&mrd_dims), mrd.float_stream()).expect("unexpected number of samples");
    let echo = mrd_array.slice(s![..,vol_index,..,..,..,..,..]).to_owned().permuted_axes([0,3,2,1,4,5]);
    let trimmed = echo.slice(s![..,n_dummy_excitations..,..,..,..,..]).to_owned();
    trimmed.to_shape(((n_views,n_read,2), Order::RowMajor)).expect("cannot reshape array").to_owned()
}


fn zero_fill(array:&Array<f32, Dim<[Ix; 3]>>,
             cs_table:&Path,
             dims:(usize,usize,usize),
             dummy_excitations:usize,
             view_acceleration:usize) ->  Array<f32, Dim<[Ix; 4]>>{
    let cs_table = CSTable::open(cs_table,dims.1 as i16,dims.2 as i16);
    let mut zf_arr = Array4::<f32>::zeros([dims.2,dims.1,dims.0,2]);
    for (i,index) in cs_table.indices(dummy_excitations*view_acceleration).iter().enumerate() {
        let mut zf_slice = zf_arr.slice_mut(s![index.0 as usize,index.1 as usize,..,..]);
        zf_slice += &array.slice(s![i,..,..]);
    }
    zf_arr
}

fn write_cfl_vol(complex_volume:&Array<f32, Dim<[Ix; 4]>>,filename:&Path) {
    let shape = complex_volume.shape();
    let numel = shape[0]*shape[1]*shape[2]*shape[3];
    let hdr_str = format!("# Dimensions\n{} {} {} 1 1",shape[2],shape[1],shape[0]);
    let flat = complex_volume.to_shape((numel,Order::RowMajor)).expect("cannot flatten with specified number of elements").to_vec();
    let n_bytes = flat.len()*4;
    let mut byte_buff:Vec<u8> = vec![0;n_bytes];
    LittleEndian::write_f32_into(&flat,&mut byte_buff);
    let mut cfl = File::create(filename.with_extension("cfl")).expect("cannot create file");
    cfl.write_all(&byte_buff).expect("problem writing to file");
    let mut hdr = File::create(filename.with_extension("hdr")).expect("cannot create file");
    hdr.write_all(hdr_str.as_bytes()).expect("a problem occurred writing to cfl header");
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

    fn n_read(&self) -> i32 {
        self.dimensions().n_read.clone()
    }

    fn n_phase1(&self) -> i32 {
        self.dimensions().n_phase1.clone()
    }

    fn n_phase2(&self) -> i32 {self.dimensions().n_phase2.clone()}

    fn n_slice(&self) -> i32 {
        self.dimensions().n_slices.clone()
    }

    fn n_views(&self) -> i32 {
        self.n_phase1()*self.n_phase2()
    }

    fn n_echos(&self) -> i32 {
        self.dimensions().n_echos.clone()
    }

    fn n_experiments(&self) -> i32 {
        self.dimensions().n_experiments.clone()
    }

    fn n_samples(&self) -> i32 {
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

    fn char_dim_array(&self) -> [usize;7] {
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

    pub fn byte_stream(&self) -> Vec<u8> {
        let mut f = File::open(&self.file_path).expect("cannot open file");
        let mut reader = BufReader::new(&mut f);
        reader.seek(SeekFrom::Start(OFFSET_TO_DATA as u64)).expect("cannot seek to data proper");
        let mut raw:Vec<u8> = vec![0;self.n_data_bytes()];
        reader.read_exact(&mut raw).expect("a problem occurred reading mrd data");
        raw
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