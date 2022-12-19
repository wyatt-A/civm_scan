use std::path::{Path, PathBuf};
use eframe::egui;
use eframe::egui::Ui;

//const BASE_STUDY_DIR:&str = "/Users/Wyatt/scratch";
const BASE_STUDY_DIR:&str = "d:/dev/studies";
const STUDY_PREFIX:&str = "N";

pub struct StudyPanel {
    work_dir:Option<PathBuf>,
    new_study_buffer:Option<String>,
    new_study_open:bool,
    creation_message:String,
}

impl StudyPanel {
    pub fn default() -> Self {
        Self {
            work_dir: None,
            new_study_buffer: None,
            new_study_open: false,
            creation_message:String::from(""),
        }
    }
    pub fn study_dir(&self) -> Option<&Path> {
        match &self.work_dir {
            Some(dir) => Some(dir),
            None => None
        }
    }
}

pub fn study_panel(ctx: &egui::Context,_ui:&mut Ui,sp:&mut StudyPanel){

    // run a check for adjustment data and load it up into memory if we find one

    egui::Window::new("Study").collapsible(false).show(ctx, |ui| {

        match &sp.work_dir {
            Some(dir) => {
                ui.label(format!("current study {}",dir.clone().into_os_string().to_str().unwrap()));
            }
            None => {

                // open/cancel functionality
                match sp.new_study_open {
                    false => {
                        if ui.button("new study").clicked() {
                            sp.new_study_open = true
                        }
                    }
                    true => {
                        if ui.button("cancel").clicked() {
                            sp.new_study_open = false
                        }
                    }
                }

                // create new study:
                if sp.new_study_open {
                    let dirname = format!("{}{}",STUDY_PREFIX,utils::date_stamp());
                    let dir = Path::new(BASE_STUDY_DIR).join(dirname);

                    let buff = sp.new_study_buffer.get_or_insert_with(||{
                        dir.clone().into_os_string().to_str().unwrap().to_string()
                    });

                    ui.horizontal(|ui|{
                        ui.text_edit_singleline(buff);
                        let dir = Path::new(buff);
                        if dir.exists(){
                            ui.label("this study already exists!");
                        }
                        if ui.button("create").clicked(){
                            if !dir.exists(){
                                match std::fs::create_dir_all(&dir){
                                    Ok(_) => {
                                        sp.work_dir = Some(dir.to_owned());
                                        sp.creation_message = String::from("study creation succeeded");
                                    },
                                    Err(_) => {
                                        sp.creation_message = String::from("unable to create study dir!");
                                    }
                                }
                            }
                        }

                    });
                }
            }
        }
        ui.label(&sp.creation_message);
    });
}