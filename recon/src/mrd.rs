use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::ops::Range;

const OFFSET_TO_DATA:usize = 512;
const HEADER_SIZE:usize = 256;
const CHARCODE_BYTES:Range<usize> = 18..20;

pub struct MRData {
    file_path:PathBuf,
}

impl MRData {

    pub fn new(file_path:&Path) -> Self{
        match file_path.exists(){
            true => Self {file_path:file_path.to_owned()},
            false => panic!("file doesn't exist {:?}",file_path)
        }
    }

    fn open(&self) -> File {
        File::open(&self.file_path).expect("cannot open file")
    }

    fn _open(file_path:&Path) {
        let mut f = File::open(file_path).expect(&format!("cannot open file {:?}",file_path));
        let mut header = [0;HEADER_SIZE];
        f.read_exact(&mut header).expect("trouble reading from file");
    }

    fn header(&self) -> [u8;HEADER_SIZE] {
        let mut f = self.open();
        let mut header = [0;HEADER_SIZE];
        f.read_exact(&mut header).expect("trouble reading header");
        header
    }

    fn n_read(&self) -> i32 {
        self.dimensions()[0]
    }

    fn n_phase(&self) -> i32 {
        self.dimensions()[1]
    }

    fn n_slice(&self) -> i32 {
        self.dimensions()[2]
    }

    fn n_views(&self) -> i32 {
        self.n_slice()*self.n_read()
    }

    fn n_echos(&self) -> i32 {
        self.dimensions()[5]
    }





    fn dimensions(&self) -> [i32;6] {
        let h = self.header();
        let mut dimension:[i32;6] = [0;6];
        dimension.iter_mut().enumerate()
            .for_each(
                |(idx,i)| match idx {
                    0 => {*i = i32::from_le_bytes(h[0..4].clone());},
                    1 => {*i = i32::from_le_bytes(h[4..8].clone());},
                    2 => {*i = i32::from_le_bytes(h[8..12].clone());},
                    3 => {*i = i32::from_le_bytes(h[12..16].clone());},
                    4 => {*i = i32::from_le_bytes(h[152..156].clone());},
                    5 => {*i = i32::from_le_bytes(h[156..160].clone());},
                    _ => {}
                }
            );
        dimension
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

    pub fn _new(path:&str) -> Mrd {
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
            _ => panic!("problem determining character bytes. mrd may be corrupt"),
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
}