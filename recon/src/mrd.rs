use ndarray::{s,Array3,Array4,Order};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use byteorder::{ByteOrder, LittleEndian};
use std::path::Path;
use core::ops::Range;
use std::fmt;
use std::mem::size_of;
use crate::utils;
use crate::pe_table::Petable;

/*
mrd_to_cfl
attempts to write a complex floating point format (bart format) for use with their tools
it requires an ASCII table that determines where lines of kspace get placed
it also requires a table size and assumes the table is square for now :)
*/
pub fn mrd_to_cfl(mrd:&str,vol_index:&str,petable:&str,table_size:&str,cfl:&str){
    let vidx:u16 = vol_index.parse().expect("cannot parse mrd volume offset. check the input");
    let pesize:usize = table_size.parse().expect("cannot parse pe table size. check the input");
    let mut mrd = Mrd::new(mrd);
    println!("found raw dims: {:?}",mrd.dimension);
    mrd.load_volume(vidx);
    let petab = Petable::new(petable);
    mrd.write_zero_filled_cfl(cfl,&petab);
}

const OFFSET_TO_DATA:usize = 512;
const HEADER_SIZE:usize = 256;
const CHARCODE_BYTES:Range<usize> = 18..20;

#[derive(Debug)]
pub struct Mrd{
    pub dimension:[i32;6],
    pub is_complex:bool,
    pub charbytes:usize,
    pub charcode:i16,
    pub numel:i32,
    pub num_chars:i32,
    pub data_bytes:usize,
    pub bytes_per_vol:usize,
    pub num_vols:i32,
    file:File,
    pub vol_bytes:Vec<u8>,
    pub zero_filled:Vec<u8>,
    pub zero_filled_dimension:[i32;3],
    is_loaded:bool
}

impl Mrd{

    /* Load volume bytes from an offset */
    pub fn load_volume(&mut self,vol_idx:u16){
        let mut reader = BufReader::new(&mut self.file);
        let vol_byte_offset = self.bytes_per_vol*vol_idx as usize + OFFSET_TO_DATA;
        reader.seek(SeekFrom::Start(vol_byte_offset as u64)).expect("Problem reaching data... is the file corrupt?");
        println!("reading mrd bits with offset {} ...",vol_idx);
        reader.read_exact(&mut self.vol_bytes).expect("an issue occured reading mrd bytes... is the file corrupt?");
        self.is_loaded = true;
    }

    pub fn write_out(&self,filename:&str){
        let out_path = Path::new(filename);
        let mut o = File::create(out_path).expect(&format!("trouble making output file {}",filename));
        o.write_all(&self.vol_bytes).expect("problem occured writing to file");
    }

    pub fn to_f32(&mut self) -> Vec<f32>{
        if !self.is_loaded {self.load_volume(0)};
        let mut floats:Vec<f32> = vec![0.0;self.bytes_per_vol/size_of::<f32>()];
        LittleEndian::read_f32_into(&self.vol_bytes,&mut floats);
        return floats;
    }

    pub fn set_bytes_from_f32(&mut self,floats:&Vec<f32>){
        LittleEndian::write_f32_into(floats,&mut self.vol_bytes);
    }

    pub fn new(path:&str) -> Mrd {

        /* Open file and get header into memory */
        let mrd_file = Mrd::open(path);
        let mut mrd_reader = BufReader::new(&mrd_file);
        let mut header_bytes = [0;HEADER_SIZE];
        mrd_reader.read_exact(&mut header_bytes).expect("a problem occured reading mrd header");
        /* Determine data dimesions from header */
        let mut dimension:[i32;6] = [0;6];
        dimension.iter_mut().enumerate()
        .for_each(
            |(idx,i)| match idx {
                0 => {*i = utils::bytes_to_long(&header_bytes[0..4]);}
                1 => {*i = utils::bytes_to_long(&header_bytes[4..8]);}
                2 => {*i = utils::bytes_to_long(&header_bytes[8..12]);}
                3 => {*i = utils::bytes_to_long(&header_bytes[12..16]);}
                4 => {*i = utils::bytes_to_long(&header_bytes[152..156]);}
                5 => {*i = utils::bytes_to_long(&header_bytes[156..160]);}
                _ => {} //no op
            }
        );

        /* Determine data type from header (charcode) */
        let mut charcode = utils::bytes_to_int(&header_bytes[CHARCODE_BYTES]);
        let is_complex = if charcode >= 16 {true} else {false};
        if is_complex {charcode -= 16};
        let charbytes:usize = match charcode{
            0 | 1 => 1,
            2 | 3 => 2,
            4 | 5 => 4,
            6 => 8,
            _ => panic!("problem determining character bytes. mrd may be currupt"),
        };
        if charbytes != 4 || !is_complex {panic!("only complex floats are supported for now")}

        /* Parse extra info that may be useful */
        let mut numel = 1;
        dimension.iter().for_each(|d| numel *= d);
        let complex_mult = is_complex as i32 + 1;
        let num_chars = numel*complex_mult;
        let data_bytes:usize = charbytes*num_chars as usize;
        let num_vols = dimension[3]*dimension[4]*dimension[5];
        let bytes_per_vol = data_bytes/(num_vols as usize);

        return Mrd{
            dimension:dimension,
            is_complex:is_complex,
            charbytes:charbytes,
            charcode:charcode,
            numel:numel,
            num_chars:num_chars,
            data_bytes:data_bytes,
            bytes_per_vol:bytes_per_vol,
            num_vols:num_vols,
            file:mrd_file,
            vol_bytes:vec![0;bytes_per_vol],
            is_loaded:false,
            zero_filled:Vec::new(),
            zero_filled_dimension:[1,1,1],
        };

    }

    pub fn data_type(&self) -> String{
        let data_type:&str = match self.charcode{
            0 => "uchar",
            1 => "char",
            2 => "short",
            3 => "int",
            4 => "long",
            5 => "float",
            6 => "double",
            _ => panic!("problem determining character bytes. mrd may be currupt"),
        };
        return data_type.to_string();
    }

    fn open(path:&str) -> File{
        let mrd_path = Path::new(path);
        let handle = File::open(mrd_path).expect("Problem opening mrd. The file path is probably incorrect");
        return handle
    }

    fn complex_mult(&self) -> usize{
        return self.is_complex as usize + 1;
    }

    pub fn dim_tuple(&self) -> (usize,usize,usize){
        return (
            (self.dimension[1]*self.dimension[2]) as usize,
            self.dimension[0] as usize,
            self.complex_mult()
        )
    }

    pub fn numel(&self) -> usize{
        let n_points = self.dimension[0]*self.dimension[1]*self.dimension[3];
        return n_points as usize * self.complex_mult();
    }

    pub fn zero_fill(&mut self,pe_table:&Petable) -> Vec<f32>{
        // Array for raw floats
        let mut raw_floats:Vec<f32> = vec![0.0;self.numel()];
        LittleEndian::read_f32_into(&self.vol_bytes,&mut raw_floats);
        let raw_arr = Array3::from_shape_vec(self.dim_tuple(), raw_floats).expect("raw floats cannot fit into shape");
        let r = self.dimension[0] as usize;
        let zf_dims = (pe_table.size,pe_table.size,r,self.complex_mult());
        let numel = zf_dims.0*zf_dims.1*zf_dims.2*zf_dims.3;
        let mut zf_arr = Array4::<f32>::zeros(zf_dims);
        println!("zero-filling compressed data ...");
        let indices = pe_table.indices();
        for (i,index) in indices.iter().enumerate() {
            let mut zf_slice = zf_arr.slice_mut(s![index.0,index.1,..,..]);
            zf_slice += &raw_arr.slice(s![i,..,..]);
        }
        println!("reshaping zero-filled ...");
        let flat = zf_arr.to_shape((numel,Order::RowMajor)).expect("unexpected data size");
        println!("flattening ...");
        return flat.to_vec();
    }

    pub fn raw_ndarray(&self){
        
    }

    pub fn write_zero_filled_cfl(&mut self,filename:&str,pe_table:&Petable){
        let zf = self.zero_fill(pe_table);
        let dims = (self.dimension[0] as usize,pe_table.size,pe_table.size);
        self.write_cfl_vol_from_vec(filename,&zf,dims);
    }

    fn write_cfl_vol_from_vec(&self,filepath:&str,data:&Vec<f32>,dim:(usize,usize,usize)){
        println!("writing to cfl ...");
        let base = Path::new(filepath);
        let cfl_raw = base.with_extension("cfl");
        let cfl_header = base.with_extension("hdr");
        let mut raw_f = File::create(cfl_raw).expect("cannot create file");
        let mut header_f = File::create(cfl_header).expect("cannot create file");
        let nbytes = self.complex_mult()*dim.0*dim.1*dim.2*self.charbytes;
        let mut bytebuff:Vec<u8> = vec![0;nbytes];
        LittleEndian::write_f32_into(data,&mut bytebuff);
        raw_f.write_all(&bytebuff).expect("a problem occured writing to cfl raw");
        let hdr_str = format!("# Dimensions\n{} {} {} 1 1",dim.0,dim.1,dim.2);
        header_f.write_all(hdr_str.as_bytes()).expect("a problem occured writing to cfl header");
    }

}

impl fmt::Display for Mrd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dims: {:?}\nn_volumes: {}", self.dimension, self.num_vols)
    }
}