//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::fs;
use std::path::{Path, PathBuf};
use eframe::egui;
use eframe::egui::{Color32, TextureHandle, Ui};
use mr_data;

use build_sequence::build_directory::build_directory;
use crate::egui::plot::{Line, Plot, PlotPoints};
use acquire::adjustment;
use scan_ui::image_utilities;
use utils;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Civm Scan",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

struct MyApp {
    name: String,
    age: u32,
    work_dir_buff: String,
    work_dir:Option<PathBuf>,
    texture: Option<egui::TextureHandle>,
    scout_view_port:ScoutViewPort,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "How motivated are you?".to_string(),
            age: 42,
            work_dir_buff: "dummy_path".to_string(),
            work_dir:None,
            texture:None,

            scout_view_port:ScoutViewPort::default()
        }
    }
}



struct ScoutViewPort {
    image_textures:Option<[TextureHandle;3]>,
}

impl ScoutViewPort {
    pub fn default() -> Self {
        Self {
            image_textures:None,
        }
    }

    pub fn find_raw_data(&mut self,scout_data_dir:&Path) -> Option<[PathBuf;3]>{
        let raw_files = utils::find_files(scout_data_dir,".mrd");
        match raw_files {
            Some(files) => {
                if files.len() >= 3 {
                    Some([files[0].clone(), files[1].clone(), files[2].clone()])
                }
                else {
                    None
                }
            }
            _ => None
        }
    }

    pub fn clear_textures(&mut self){
        self.image_textures = None;
    }

    pub fn textures(&mut self, ui: &mut Ui) -> &[TextureHandle;3] {

        self.image_textures.get_or_insert_with(||{
            let image_data = mr_data::mrd::mrd_to_2d_image(Path::new("./test_data/scout_data/m0/m0.mrd"));
            let texture1:TextureHandle = ui.ctx().load_texture(
                "view-0",
                image_utilities::array_to_image(&image_data),
                egui::TextureFilter::Linear
            );
            let image_data = mr_data::mrd::mrd_to_2d_image(Path::new("./test_data/scout_data/m1/m1.mrd"));
            let texture2:TextureHandle = ui.ctx().load_texture(
                "view-1",
                image_utilities::array_to_image(&image_data),
                egui::TextureFilter::Linear
            );
            let image_data = mr_data::mrd::mrd_to_2d_image(Path::new("./test_data/scout_data/m2/m2.mrd"));
            let texture3:TextureHandle = ui.ctx().load_texture(
                "view-2",
                image_utilities::array_to_image(&image_data),
                egui::TextureFilter::Linear
            );
            [texture1,texture2,texture3]
        })
    }
}


impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {



        egui::CentralPanel::default().show(ctx, |ui| {

            ui.heading("Civm Scan");


            egui::Window::new("Scout View").collapsible(true).show(ctx, |ui| {

                let textures = self.scout_view_port.textures(ui);
                ui.label("How'd you do?");
                ui.horizontal(|ui|{
                    ui.image(&textures[0],textures[0].size_vec2());
                    ui.image(&textures[1],textures[1].size_vec2());
                    ui.image(&textures[2],textures[2].size_vec2());
                });
                if ui.button("reload").clicked(){
                    self.scout_view_port.clear_textures();
                }
            });

            egui::Window::new("Adjustments").show(ctx, |ui| {
                ui.label("FID spectrum");
                Plot::new("frequency_plot").show_axes([true,true]).view_aspect(1.0).show(ui, |plot_ui| {
                    let adj_data = adjustment::AdjustmentResults::from_file(Path::new("./test_data/adj_data/adjustment_results.json"));
                    let line = Line::new(PlotPoints::new(adj_data.freq_spectrum.clone())).color(Color32::from_rgb(255,255,255));
                    plot_ui.line(line);
                });

                ui.label("Spin Echo vs Stimulated Echo");
                Plot::new("diff_plot").show_axes([true,true]).view_aspect(1.0).show(ui, |plot_ui| {
                    let adj_data = adjustment::AdjustmentResults::from_file(Path::new("./test_data/adj_data/adjustment_results.json"));
                    let line = Line::new(PlotPoints::new(adj_data.rf_cal_spin_vs_stim.clone())).color(Color32::from_rgb(255,255,255));
                    plot_ui.line(line);
                });

            });


            ui.label("Working Directory");
            ui.horizontal(|ui| {
                if ui.text_edit_singleline(&mut self.work_dir_buff).ctx.input().key_pressed(egui::Key::Enter) {
                    let path = Path::new(&self.work_dir_buff);
                    match path.exists() && path.is_dir() {
                        true => self.work_dir = Some(path.to_owned()),
                        false => self.work_dir = None
                    }
                }
                if ui.button("create").clicked() {
                    let path = Path::new(&self.work_dir_buff);
                    match fs::create_dir_all(&self.work_dir_buff).is_ok() {
                        true => self.work_dir = Some(path.to_owned()),
                        false => self.work_dir = None
                    }
                }
                match self.work_dir.is_some() {
                    true => ui.label("ok"),
                    false => ui.label("not a directory")
                }
            });
            ui.button("run setup").clicked();
            ui.button("start scan").clicked();
            if ui.button("compile sequence").clicked() {
                build_directory(Path::new("d:/dev/221011"));
            }
        });
    }
}

struct MyImage {
    texture: Option<egui::TextureHandle>,
}

impl MyImage {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
            // Load the texture only once.
            ui.ctx().load_texture(
                "my-image",
                egui::ColorImage::example(),
                egui::TextureFilter::Linear
            )
        });

        // Show the image:
        //ui.add(egui::Image::new(texture, texture.size_vec2()));

        // Shorter version:
        ui.image(texture, texture.size_vec2());
    }
}