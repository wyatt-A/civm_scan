//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::fs;
use std::path::{Path, PathBuf};
use eframe::egui;
use eframe::egui::Color32;

use build_sequence::build_directory::build_directory;

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
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "How motivated are you?".to_string(),
            age: 42,
            work_dir_buff: "dummy_path".to_string(),
            work_dir:None,
            texture:None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {





        egui::CentralPanel::default().show(ctx, |ui| {

            egui::CollapsingHeader::new("Header").default_open(true).show(ui,|ui|{
                ui.label("Hello");
            });

            let texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
                // Load the texture only once.
                let p = Color32::from_gray(255).to_srgba_unmultiplied();
                let mut img = egui::ColorImage::from_rgba_unmultiplied([1,1],&p);
                img.size = [1,1];
                ui.ctx().load_texture(
                    "my-image",
                    img,
                    egui::TextureFilter::Linear
                )
            });



            ui.heading("Civm Scan");

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
            ui.image(texture, texture.size_vec2());
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