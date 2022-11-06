use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::ops::Range;
use seq_lib::pulse_sequence::AcqDims;
use acquire::build::acq_dims;
use cs_table::cs_table::CSTable;
use byteorder::{LittleEndian,ByteOrder};
use ndarray::{s, Array3, Array4, Order, Dim, ArrayD, IxDyn, concatenate};
use ndarray::{Array, ArrayView, array, Axis};
use ndarray::iter::Axes;

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
    let p = Path::new("/home/wyatt/projects/test_data/acq/m00/m00.mrd");
    let table = Path::new("/home/wyatt/projects/test_data/acq/m00/cs_table");
    let param_file = Path::new("/home/wyatt/projects/test_data/acq/m00/fse_dti.json");

    let dims = acq_dims(param_file);

    let mrd = MRData::new(p);

    let cs_table = CSTable::open(table, dims.n_phase1 as i16, dims.n_phase2 as i16);

    println!("{:?}",dims);
    println!("{}",mrd.n_views());
    println!("{}",mrd.n_samples());


    let a = ArrayD::<f32>::from_shape_vec(IxDyn(&mrd.char_dim_array()),mrd.float_stream()).expect("unexpected number of samples");
    let e1 = a.slice(s![..,..,..,..,..,0,..]);
    let mut v1 = a.slice(s![..,..,..,..,..,1,..]).to_owned();
    let v2 = a.slice(s![..,..,..,..,..,2,..]);
    v1 += &v2;
    let f = concatenate(Axis(5),&[e1,v1.view()]);
    println!("{:?}",f);
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

    // pub fn zero_fill(&self,pe_table:&Petable) -> Vec<f32>{
    //     // Array for raw floats
    //     let mut raw_floats:Vec<f32> = vec![0.0;self.n_chars()];
    //     LittleEndian::read_f32_into(&self.vol_bytes,&mut raw_floats);
    //     let raw_arr = Array3::from_shape_vec((2,self.n_read(),self.n_views()), raw_floats).expect("raw floats cannot fit into shape");
    //     let r = self.dimension[0] as usize;
    //     let zf_dims = (pe_table.size,pe_table.size,r,self.complex_mult());
    //     let numel = zf_dims.0*zf_dims.1*zf_dims.2*zf_dims.3;
    //     let mut zf_arr = Array4::<f32>::zeros(zf_dims);
    //     println!("zero-filling compressed data ...");
    //     let indices = pe_table.indices();
    //     for (i,index) in indices.iter().enumerate() {
    //         let mut zf_slice = zf_arr.slice_mut(s![index.0,index.1,..,..]);
    //         zf_slice += &raw_arr.slice(s![i,..,..]);
    //     }
    //     println!("reshaping zero-filled ...");
    //     let flat = zf_arr.to_shape((numel,Order::RowMajor)).expect("unexpected data size");
    //     println!("flattening ...");
    //     return flat.to_vec();
    // }
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