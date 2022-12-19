/*
 UI window object used to view, edit, and save pulse sequence settings
*/

use std::path::Path;
use std::str::FromStr;
use eframe::egui;
use eframe::egui::{Ui};
use eframe::epaint::ahash::{HashMap, HashMapExt};
//use serde::{Serialize,Deserialize};
use serde_json::{Result, Value};
use acquire::build;
use utils;


pub struct SequenceEditor {
    config_names:Option<Vec<String>>,
    selected:Option<String>,
    json_buffer:Option<String>,
    save_as_buffer:String,
    save_success:Option<bool>,
    validation_response:Option<String>,
}

impl SequenceEditor {
    pub fn default() -> Self {
        Self {
            config_names:None,
            selected:None,
            json_buffer:None,
            save_as_buffer:String::new(),
            save_success:None,
            validation_response:None,
        }
    }
}


pub fn sequence_editor(ctx: &egui::Context,ui:&mut Ui,se:&mut SequenceEditor){

    let seq_config = Path::new("./test_env/sequence_library/5xfad_fse.json");

    let sequence_library = Path::new("./test_env/sequence_library");

    let config_names = se.config_names.get_or_insert_with(||{
        let files = utils::find_files(sequence_library,"json",true).unwrap();
        files.iter().map(|f| {
            let s = f.file_stem().unwrap();
            s.to_str().unwrap().to_string()
        }).collect()
    });


    egui::Window::new("Sequence Editor").scroll2([true,true]).show(ctx, |ui| {

        ui.label("Sequence Editor");
        egui::ComboBox::from_label("")
            .selected_text(format!("{}", se.selected.clone().unwrap_or(String::from("None"))))
            .show_ui(ui, |ui| {
                for name in config_names {
                    if ui.selectable_value(&mut se.selected,Some(name.clone()),name.as_str()).clicked(){
                        // clear json editor buffer to refresh later
                        se.json_buffer = None;
                        se.save_success = None;
                    };
                }
            });

        let str_buff = se.json_buffer.get_or_insert_with(||{
            match &se.selected {
                Some(sequence_name) => {
                    utils::read_to_string(&sequence_library.join(sequence_name),"json")
                }
                None => String::new()
            }
        });

        egui::CollapsingHeader::new("save as").show(ui, |ui| {
            ui.horizontal(|ui|{
                if ui.text_edit_singleline(&mut se.save_as_buffer).ctx.input().key_pressed(egui::Key::Enter){
                    se.save_success = None;

                    let new_config = sequence_library.join(&se.save_as_buffer).with_extension("json");

                    if new_config.exists(){
                        se.save_success = Some(false)
                    }
                    else{
                        utils::write_to_file(&new_config,"json",&str_buff);
                        se.save_success = Some(true)
                    }
                }
            });
            if se.save_success.is_some() {
                let _ = match se.save_success.unwrap(){
                    true => ui.label(format!("saved successfully")),
                    false => ui.label("sequence name already exists!")
                };
            }
        });


        egui::CollapsingHeader::new("validate").show(ui, |ui| {
            if ui.button("validate").clicked(){

                // write to temp file and clean up
                let f = utils::write_to_file(&sequence_library.join("temp_config"),"json",&str_buff);

                match &se.selected {
                    Some(config) => {
                        let valid = build::validate(&f);
                        let _ = match valid.0 {
                            true => {
                                se.validation_response = Some(String::from("sequence is valid!"));
                            }
                            false =>{
                                se.validation_response = Some(valid.1.unwrap());
                            }
                        };
                    }
                    None => {}
                }

                std::fs::remove_file(&f).expect("cannot remove temp file");
            }
            if se.validation_response.is_some(){
                ui.label(se.validation_response.clone().unwrap());
            }

        });

        ui.code_editor(str_buff);
    });

}

#[derive(Debug,PartialEq)]
enum Enum {
    First,
    Second,
    Third,
}