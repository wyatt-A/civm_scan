use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const MAX_TABLE_ELEMENTS:usize = 196095;

pub struct CSTable {
    source:PathBuf,
    elements:Vec<i16>,
}

pub struct KspaceCoord {
    pub k_phase:i16,
    pub k_slice:i16
}

impl CSTable {
    pub fn open(source: &Path) -> Self {
        if !source.exists() {
            println!("{:?}",source);
            panic!("cs table not found!");
        }
        let mut s = String::new();
        let mut f = File::open(&source).expect("cannot open file");
        f.read_to_string(&mut s).expect("cannot read from file");
        // we are expecting a list of integers
        let e = s.lines().flat_map(|line| line.parse()).collect();
        Self {
            source:source.to_owned(),
            elements:e,
        }
    }

    pub fn n_elements(&self) -> usize {
        self.elements.len()
    }

    pub fn elements(&self) -> Vec<i16> {
        self.elements.clone()
    }

    pub fn n_views(&self) -> usize {
        self.elements.len()/2
    }

    pub fn coordinates(&self,read_element_offset:usize) -> Vec<KspaceCoord> {
        if (self.n_elements() % 2) != 0 {
            panic!("table must have an even number of elements");
        }
        let mut coords = Vec::<KspaceCoord>::with_capacity(self.n_elements()/2);
        let range = read_element_offset..self.n_elements()/2;
        for i in range {
            coords.push(
                KspaceCoord {
                    k_phase:self.elements[2*i],
                    k_slice:self.elements[2*i+1],
                }
            )
        }
        coords
    }

    pub fn indices(&self,read_element_offset:usize,matrix_size:[i16;2]) -> Vec<(i16,i16)> {
        let phase_off = matrix_size[0]/2;
        let slice_off = matrix_size[1]/2;
        self.coordinates(read_element_offset).iter().map(|coord| (coord.k_phase + phase_off,coord.k_slice + slice_off)).collect()
    }

    pub fn copy_to(&self,dest:&Path,file_name:&str) {
        let mut s = String::new();
        let mut f = File::open(&self.source).expect("cannot open file");
        let fname = dest.join(file_name);
        f.read_to_string(&mut s).expect("cannot read from file");
        let mut d = File::create(fname).expect("cannot create file");
        d.write_all(s.as_bytes()).expect("trouble writing to file");
    }
}


// sets number of repetitions accordingly
pub trait CompressedSensing {
    fn set_cs_table(&mut self);
    fn cs_table(&self) -> PathBuf;
}
