use std::path::{Path, PathBuf};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use eframe::egui;
use eframe::egui::{Color32, Ui};
use eframe::egui::Key::P;
use eframe::egui::plot::{Legend, Line, Plot, PlotPoints};
use acquire::adjustment::ADJ_FILE_NAME;
use acquire::build::ContextParams;
use seq_lib::pulse_sequence::AdjustmentResults;
use seq_tools::ppl::Adjustment;
use crate::study_panel::StudyPanel;

const ADJ_DATA_DIR_NAME:&str = "adj_data";

pub struct BasicAdjustmentPanel {
    current_results:Option<PathBuf>,
    adj_data:Option<AdjustmentResults>,
    status_message:String,
    adjustments_running:bool,
    process_handle:Option<JoinHandle<()>>
}

impl BasicAdjustmentPanel{
    pub fn default() -> Self{
        BasicAdjustmentPanel{
            current_results:None,
            adj_data:None,
            status_message:String::from(""),
            adjustments_running:false,
            process_handle:None,
        }
    }
}

impl BasicAdjustmentPanel {

    pub fn set_results_dir(&mut self,results:&Path) {
        self.current_results = Some(results.to_owned());
    }

    fn get_adj_data(&mut self) -> Option<&AdjustmentResults> {
        match &self.current_results {
            Some(results) => {
                let filename = results.join(ADJ_FILE_NAME).with_extension("json");
                if !filename.exists(){None}
                else {
                    Some(self.adj_data.get_or_insert(AdjustmentResults::from_file(&filename)))
                }
            }
            None => None
        }
    }

    pub fn adjustment_file(&self) -> Option<PathBuf> {
        match &self.current_results {
            Some(results) => {
                let filename = results.join(ADJ_FILE_NAME).with_extension("json");
                if !filename.exists(){None}
                else {
                    Some(filename)
                }
            }
            None => None
        }
    }

    pub fn adjustment_is_running(&self) -> bool {
        match &self.process_handle {
            Some(handle) => {
                !handle.is_finished()
            }
            None => false
        }
    }

}

pub fn run_adjustments(study_dir:&Path) {
    let freq_cal = Path::new(r"C:\workstation\dev\civm_scan\test_env\sequence_library/1p.json");
    let rf_cal = Path::new(r"C:\workstation\dev\civm_scan\test_env\sequence_library\rf_cal.json");
    let dir = study_dir.join(ADJ_DATA_DIR_NAME);
    acquire::adjustment::Adjustment::new(freq_cal,rf_cal,&dir).run();
}

pub fn basic_adjustemnt(ctx: &egui::Context,ui:&mut Ui,ba:&mut BasicAdjustmentPanel,sp:&StudyPanel){

    // run a check for adjustment data and load it up into memory if we find one


    egui::Window::new("Adjustments").collapsible(false).show(ctx, |ui| {

        match &sp.study_dir(){
            Some(dir) => {
                ba.current_results.get_or_insert((*dir.join(ADJ_DATA_DIR_NAME)).to_owned());

                let adj_data = ba.get_adj_data();

                if adj_data.is_some() {
                    egui::CollapsingHeader::new("FID").show(ui, |ui| {

                        Plot::new("frequency_plot").show_axes([true,true]).view_aspect(1.5).show(ui, |plot_ui| {
                            //let adj_data = AdjustmentResults::from_file(Path::new("./test_data/adj_data/adjustment_results.json"));
                            let line = Line::new(PlotPoints::new(adj_data.unwrap().freq_spectrum.clone())).color(Color32::from_rgb(255,255,255));
                            plot_ui.line(line);
                        });
                    });

                    egui::CollapsingHeader::new("RF Calibration").show(ui, |ui| {
                        Plot::new("diff_plot").legend(Legend::default()).show_axes([true,true]).view_aspect(1.5).show(ui, |plot_ui| {
                            //let adj_data = AdjustmentResults::from_file(Path::new("./test_data/adj_data/adjustment_results.json"));
                            let line = Line::new(PlotPoints::new(adj_data.unwrap().rf_cal_spin_vs_stim.clone())).color(Color32::from_rgb(255,255,255));
                            plot_ui.line(line);
                            //plot_ui.
                        });
                    });
                }else{
                    ui.label("no adjustments found!");


                    // are adjustments running?

                    match ba.adjustment_is_running() {
                        true => {
                            ui.label("running adjustments ...");
                        }
                        false => {
                            ba.process_handle = None;
                            if ui.button("run basic adjustments").clicked(){
                                match sp.study_dir(){
                                    Some(dir) =>{
                                        let dir_for_thread = dir.to_owned();
                                        // should go in a new thread
                                        ba.process_handle = Some(thread::spawn(move || {
                                            println!("spawning new thread");
                                            run_adjustments(&dir_for_thread)
                                        }));
                                    } ,
                                    None => ba.status_message = String::from("study directory is not specified")
                                }
                            }
                        }
                    }
                }
                ui.label(&ba.status_message);
            }
            None => {
                ui.label("use the study panel to create a new study before running adjustments.");
            }
        }

    });
}
