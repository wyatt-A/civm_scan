//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use eframe::egui;
use scan_ui::basic_adjustment::{basic_adjustemnt, BasicAdjustmentPanel};
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
    scout_view_port:ScoutViewPort,
    sequence_editor:SequenceEditor,
    adjustment_panel:BasicAdjustmentPanel,
    sequence_viwer:SequenceViewer,
    study_panel:StudyPanel,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
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
        });
    }
}