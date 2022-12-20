use std::path::{Path, PathBuf};
use eframe::egui;
use eframe::egui::Ui;
use utils;

const BASE_STUDY_DIR:&str = "/Users/Wyatt/scratch";
//const BASE_STUDY_DIR:&str = "d:/dev/studies";
const STUDY_PREFIX:&str = "N";


enum StudyType {
    New,
    Recalled,
}


pub struct StudyPanel {
    work_dir:Option<PathBuf>,
    new_study_buffer:Option<String>,
    sub_menu_open:bool,
    study_type:Option<StudyType>,
    creation_message:String,
    selected_study_buffer:Option<String>,
}

impl StudyPanel {
    pub fn default() -> Self {
        Self {
            work_dir: None,
            new_study_buffer: None,
            sub_menu_open: false,
            study_type: None,
            creation_message:String::from(""),
            selected_study_buffer:None,
        }
    }
    pub fn study_dir(&self) -> Option<&Path> {
        match &self.work_dir {
            Some(dir) => Some(dir),
            None => None
        }
    }
}

pub fn study_panel(_ctx: &egui::Context,ui:&mut Ui,sp:&mut StudyPanel) {

    if sp.work_dir.is_some() {
        let dir = sp.work_dir.clone().unwrap();
        ui.label(format!("current study {}", dir.to_str().unwrap()));
    }

    match sp.sub_menu_open {
        false => {
            if ui.button("new study").clicked() {
                sp.study_type = Some(StudyType::New);
                sp.work_dir = None;
                sp.sub_menu_open = true;
            }
            if ui.button("recall study").clicked() {
                sp.study_type = Some(StudyType::Recalled);
                sp.work_dir = None;
                sp.sub_menu_open = true;
            }
        }
        true => {

        }
    }

    if sp.work_dir.is_none() {
        // new study -> directory prompt -> dir validation/creation -> set work dir
        // recall study -> listbox -> dir selection -> set work dir

        match &sp.study_type {
            None => {

            }
            Some(study_type) => {
                match study_type {
                    StudyType::New => {
                        ui.label("Create a new study");
                        create_study(ui, sp);
                        if ui.button("cancel").clicked() {
                            sp.study_type = None;
                            sp.sub_menu_open = false;
                        }

                    }
                    StudyType::Recalled => {
                        ui.label("Select a previous study");
                        select_study(ui, sp);
                        if ui.button("cancel").clicked() {
                            sp.study_type = None;
                            sp.sub_menu_open = false;
                        }
                    }
                }
            }
        }
    }
}

fn select_study(ui:&mut Ui,sp:&mut StudyPanel) {
    let base_dir = Path::new(BASE_STUDY_DIR);


    match utils::get_all_matches(base_dir,&format!("{}*",STUDY_PREFIX)) {
        Some(items) => {
            let dirs:Vec<&PathBuf> = items.iter().filter(|file| file.is_dir()).collect();
            let dirnames:Vec<String> = dirs.iter().map(|dir| dir.file_name().unwrap().to_str().unwrap().to_string()).collect();

            egui::ComboBox::from_label("")
                .selected_text(format!("{}", sp.selected_study_buffer.clone().unwrap_or(String::from("None"))))
                .show_ui(ui, |ui| {
                    for study in dirnames {
                        if ui.selectable_value(&mut sp.selected_study_buffer,Some(study.clone()),study.as_str()).clicked(){
                            sp.work_dir = Some(base_dir.join(study));
                            sp.sub_menu_open = false;
                        };
                    }
                });
        }
        None => {
            ui.label("dir empty");
        }
    }

}

fn create_study(ui:&mut Ui,sp:&mut StudyPanel) {
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
                        sp.sub_menu_open = false;
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