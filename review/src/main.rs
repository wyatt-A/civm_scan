use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use utils;
use mr_data;
use headfile;
use headfile::headfile::Headfile;
use ndarray::{s, Array3, Array4, Order, Dim, ArrayD, IxDyn, concatenate, Ix, ArrayViewMut, OwnedRepr, Ix3, ArrayBase, AssignElem, Array2};
use byteorder::{ByteOrder,BigEndian};
//
// fn main() {
//     println!("data set review");
//
//     // biggus_diskus
//     let work_dir = Path::new("c:/Users/waust/review");
//
//     let runno = "N60204";
//
//
//     // crawl work dir for matches to runno
//     // walk those dirs to find headfiles
//     // get dims and n_vols from headfile
//
//     let runs = utils::get_all_matches(work_dir,&format!("{}_m*",runno));
//     if runs.is_none(){
//         println!("no runnos found for {}.",runno);
//         return
//     }
//     else {
//         println!("found {} dirs for {}",runs.as_ref().unwrap().len(),runno);
//     }
//     let runs = runs.unwrap();
//     // find all head files
//     // expecting 1 headfile per directory
//     let headfiles:Vec<PathBuf> = runs.iter().map(|dir| {
//         utils::find_files(dir,"headfile").expect(&format!("did not find a headfile in {:?}",dir))[0].clone()
//     }).collect();
//
//     println!("{:?}",headfiles);
//
//     // dimension consistency check
//     let dims:Vec<(usize,usize,usize)> = headfiles.iter().map(|hf|{
//         let h = Headfile::open(hf).to_hash();
//         let x:usize = h.get("dim_X").expect("dim_X not found in headfile!").parse().expect("cannot parse dim_X to int");
//         let y:usize = h.get("dim_Y").expect("dim_Y not found in headfile!").parse().expect("cannot parse dim_Y to int");
//         let z:usize = h.get("dim_Z").expect("dim_Z not found in headfile!").parse().expect("cannot parse dim_Z to int");
//         (x,y,z)
//     }).collect();
//
//     let first_dim = dims[0].clone();
//     dims.iter().for_each(|dim|{
//         if first_dim.0 != dim.0 || first_dim.1 != dim.1 || first_dim.2 != dim.2 {
//             panic!("image dimension consistency check failed! {:?} != {:?}",first_dim,dim);
//         }
//     });
//
//     headfiles.iter().for_each(|hf|{
//        // get list of raw files
//         let raw = utils::find_files(hf.parent().unwrap(),"raw").unwrap();
//         let mut vol_buffer = Vec::<u16>::with_capacity(first_dim.0*first_dim.1*first_dim.2);
//         if raw.len() != first_dim.2 {panic!("some raw files are missing. Expected {} found {}",first_dim.2,raw.len())}
//         raw.iter().enumerate().for_each(|(index,raw_f)|{
//             let mut f = File::open(raw_f).expect("coudn't open file");
//             let mut bytebuff = Vec::<u8>::new();
//             f.read_to_end(&mut bytebuff).expect("cannot read from file");
//             let mut ints = Vec::<u16>::with_capacity(first_dim.0*first_dim.1);
//             BigEndian::read_u16_into(&bytebuff,&mut ints);
//             vol_buffer.extend(ints);
//         });
//         Array3::<u16>::from_shape_vec((first_dim.2,first_dim.1,first_dim.0),vol_buffer).expect("ain't gone fit");
//     });
//
//
//
//
//     // runs.iter().for_each(|dir|{
//     //     // crawl for raw files and a headfile
//     //     let raw = utils::find_files(dir,"raw");
//     //     let headfile = ;
//     //     let f = &headfile[0];
//     //     let hf = Headfile::open(f);
//     //     let h = hf.to_hash();
//     //
//     //     let x:usize = h.get("dim_X").unwrap().parse().expect("unable to parse value");
//     //     let y:usize = h.get("dim_Y").unwrap().parse().expect("unable to parse value");
//     //     let z:usize = h.get("dim_Z").unwrap().parse().expect("unable to parse value");
//     //
//     //
//     //
//     //
//     //
//     // });
//
//
// }

//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::fs;
use eframe::egui;
use eframe::egui::Widget;


fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Civm Scan",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

struct MyApp {
    image_buffer:Option<egui::TextureHandle>,
    image_volume:Option<ImageVolume>,
    image_dir:Option<String>,
    valid_dir:Option<PathBuf>,
    index:usize,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            image_buffer:None,
            image_volume:None,
            index:200,
            image_dir:None,
            valid_dir:None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {

            // get image info if not loaded


            ui.heading("Image Review");
            ui.label("slice and dice");


            if self.valid_dir.is_some(){
                println!("this is running");
                let p = self.valid_dir.clone().unwrap();
                let texture:&egui::TextureHandle = self.image_buffer.get_or_insert_with(|| {
                    let v = self.image_volume.get_or_insert(ImageVolume::from_dir(&p));
                    ui.ctx().load_texture(
                        "my-image",
                        v.color_image(self.index),
                        egui::TextureFilter::Linear
                    )
                });
                ui.image(texture,texture.size_vec2());
            }

            ui.text_edit_singleline(self.image_dir.get_or_insert(String::from("")));
            if ui.button("open").clicked(){
                let imgp = self.image_dir.clone().unwrap_or(String::from("-"));
                let img_path = Path::new(&imgp);
                if img_path.exists(){
                    println!("setting path {:?}",img_path);
                    self.valid_dir = Some(img_path.to_owned());
                    self.image_buffer = None;
                }
            }

            ui.horizontal(|ui| {
                if ui.button("<--").clicked() && self.index != 0{
                   self.index -= 1;
                    self.image_buffer = None;
                };
                if ui.button("-->").clicked(){
                    self.index += 1;
                    self.image_buffer = None;
                };
                let s = ui.add(egui::Slider::new(&mut self.index, 0..=479).text("slide me"));
                if s.dragged(){
                    self.image_buffer = None;
                }
            });
        });
    }
}


#[derive(Debug)]
struct ImageVolume {
    dims:(usize,usize,usize),
    raw_files:Vec<PathBuf>,
}

impl ImageVolume{
    pub fn from_dir(dir:&Path) -> Self {
        let hf = utils::find_files(dir,"headfile").expect(&format!("did not find a headfile in {:?}",dir))[0].clone();
        let h = Headfile::open(&hf).to_hash();
        let x:usize = h.get("dim_X").expect("dim_X not found in headfile!").parse().expect("cannot parse dim_X to int");
        let y:usize = h.get("dim_Y").expect("dim_Y not found in headfile!").parse().expect("cannot parse dim_Y to int");
        let z:usize = h.get("dim_Z").expect("dim_Z not found in headfile!").parse().expect("cannot parse dim_Z to int");
        let raw = utils::find_files(hf.parent().unwrap(),"raw").unwrap();
        //if raw.len() != first_dim.2 {panic!("some raw files are missing. Expected {} found {}",first_dim.2,raw.len())}
        Self {
            raw_files:raw,
            dims:(x,y,z)
        }
    }

    pub fn color_image(&self,index:usize) -> egui::ColorImage {
        let p = &self.raw_files[index];
        let mut f = File::open(p).expect("coudn't open file");
        let mut bytebuff = Vec::<u8>::new();
        f.read_to_end(&mut bytebuff).expect("cannot read from file");
        let mut ints = vec![0;788*480];
        BigEndian::read_u16_into(&bytebuff,&mut ints);
        let mut pixels = Vec::<u8>::new();
        ints.iter().for_each(|pix|{
            pixels.extend(egui::Color32::from_gray((*pix/256) as u8).to_srgba_unmultiplied());
        });
        egui::ColorImage::from_rgba_unmultiplied([788, 480],pixels.as_slice())
    }

}