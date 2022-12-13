//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::fs;
use std::path::{Path, PathBuf};
use eframe::egui;
use eframe::egui::{Color32, TextureHandle, Ui};
use mr_data;

use build_sequence::build_directory::build_directory;
use crate::egui::plot::{Line, Plot, PlotPoints};
use acquire::adjustment;
use acquire::adjustment::Adjustment;
use scan_ui::basic_adjustment::{basic_adjustemnt, BasicAdjustmentPanel};
use scan_ui::image_utilities;
use utils;
use scan_ui::sequence_editor::{sequence_editor, SequenceEditor};
use scan_ui::scout_viewer::{scout_viewer,ScoutViewPort};
use scan_ui::sequence_viewer::{sequence_viewer, SequenceViewer};
use scan_ui::study_panel::{study_panel,StudyPanel};

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
    sequence_editor:SequenceEditor,
    adjustment_panel:BasicAdjustmentPanel,
    sequence_viwer:SequenceViewer,
    study_panel:StudyPanel,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "How motivated are you?".to_string(),
            age: 42,
            work_dir_buff: "dummy_path".to_string(),
            work_dir:None,
            texture:None,
            scout_view_port:ScoutViewPort::default(),
            sequence_editor:SequenceEditor::default(),
            adjustment_panel:BasicAdjustmentPanel::default(),
            sequence_viwer:SequenceViewer::default(),
            study_panel:StudyPanel::default(),
        }
    }
}


impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
            ui.label("Window Selector");

            egui::CollapsingHeader::new("Study Panel").show(ui, |ui| {
                study_panel(ctx,ui,&mut self.study_panel);
            });

            egui::CollapsingHeader::new("Basic Adjustment Panel").show(ui, |ui| {
                basic_adjustemnt(ctx,ui,&mut self.adjustment_panel,&self.study_panel);
            });

            egui::CollapsingHeader::new("Sequence Selector").show(ui, |ui| {
                sequence_editor(ctx,ui,&mut self.sequence_editor);
            });

            egui::CollapsingHeader::new("Scout Viewer").show(ui, |ui| {
                scout_viewer(ctx,ui,&mut self.scout_view_port,&self.adjustment_panel,&self.study_panel);
            });

            egui::CollapsingHeader::new("Sequence Viewer").show(ui, |ui| {
                sequence_viewer(ctx,ui,&mut self.sequence_viwer);
            });

        });


        egui::CentralPanel::default().show(ctx, |ui| {

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

            if ui.button("run adjustment").clicked(){
                Adjustment::new(
                    Path::new("./test_env/sequence_library/1p.json"),
                    Path::new("./test_env/sequence_library/rf_cal.json"),
                    Path::new("./test_data/adj_data")
                ).run();
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