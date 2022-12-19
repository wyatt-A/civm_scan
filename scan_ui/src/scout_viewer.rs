use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use eframe::egui;
use eframe::egui::{TextureHandle, Ui};
use acquire::build::ContextParams;
use crate::basic_adjustment::BasicAdjustmentPanel;
use crate::image_utilities;
use crate::study_panel::StudyPanel;


pub const SCOUT_DIR_NAME:&str = "scout";

pub struct ScoutViewPort {
    image_textures:Option<[TextureHandle;3]>,
    process_handle:Option<JoinHandle<()>>,
}

impl ScoutViewPort {
    pub fn default() -> Self {
        Self {
            image_textures:None,
            process_handle:None,
        }
    }

    pub fn is_running(&self) -> bool {
        match &self.process_handle {
            Some(handle) => !handle.is_finished(),
            None => false
        }
    }

    pub fn run_scout(&mut self,study_dir:&Path,ba:&BasicAdjustmentPanel) {
        let scout_params = Path::new(r"C:\workstation\dev\civm_scan\test_env\sequence_library/scout.json");


        let scout_dir = study_dir.join(SCOUT_DIR_NAME);

        let ctx = match ba.adjustment_file() {
            Some(file) => ContextParams::from_adjustments(&file,&scout_dir,true),
            None => ContextParams::without_adjustments(&scout_dir,true)
        };

        self.process_handle = Some(std::thread::spawn(move ||{
            println!("launching new thread");
            acquire::scout::Scout::new(scout_params,&ctx).run().unwrap();
        }));

    }

    pub fn find_raw_data(&mut self,scout_data_dir:&Path) -> Option<[PathBuf;3]>{
        let raw_files = utils::find_files(scout_data_dir,"mrd",true);
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

    pub fn textures(&mut self, ui: &mut Ui,raw_files:[PathBuf;3]) -> &[TextureHandle;3] {

                self.image_textures.get_or_insert_with(||{
                    let image_data = mr_data::mrd::mrd_to_2d_image(&raw_files[0]);
                    let texture1:TextureHandle = ui.ctx().load_texture(
                        "view-0",
                        image_utilities::array_to_image(&image_data),
                        egui::TextureFilter::Linear
                    );
                    let image_data = mr_data::mrd::mrd_to_2d_image(&raw_files[1]);
                    let texture2:TextureHandle = ui.ctx().load_texture(
                        "view-1",
                        image_utilities::array_to_image(&image_data),
                        egui::TextureFilter::Linear
                    );
                    let image_data = mr_data::mrd::mrd_to_2d_image(&raw_files[2]);
                    let texture3:TextureHandle = ui.ctx().load_texture(
                        "view-2",
                        image_utilities::array_to_image(&image_data),
                        egui::TextureFilter::Linear
                    );
                    [texture1,texture2,texture3]
                })

    }
}

pub fn scout_viewer(ctx: &egui::Context,_ui:&mut Ui,scout_view:&mut ScoutViewPort,ba:&BasicAdjustmentPanel,study_panel:&StudyPanel){
    egui::Window::new("Scout View").collapsible(false).show(ctx, |ui| {

        match study_panel.study_dir() {
            Some(dir) => {
                if ui.button("run scout").clicked() {
                    scout_view.clear_textures();
                    scout_view.run_scout(dir,ba);
                }

                // attempt to load up some scout images
                let textures = match scout_view.find_raw_data(&dir.join(SCOUT_DIR_NAME)) {
                    Some(files) => Some(scout_view.textures(ui,files)),
                    None => None
                };

                // attempt to display them
                match textures {
                    Some(textures) => {
                        ui.label("How'd you do?");
                        ui.horizontal(|ui|{
                            ui.image(&textures[0],textures[0].size_vec2());
                            ui.image(&textures[1],textures[1].size_vec2());
                            ui.image(&textures[2],textures[2].size_vec2());
                        });
                        if ui.button("refresh").clicked(){
                            scout_view.clear_textures();
                        }
                    }
                    None => {}
                }
            },
            None => {
                ui.label("you must create or recall a study");
            }
        }

    });
}