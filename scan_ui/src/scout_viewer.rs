use std::path::{Path, PathBuf};
use eframe::egui;
use eframe::egui::{TextureHandle, Ui};
use crate::image_utilities;

pub struct ScoutViewPort {
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

pub fn scout_viewer(ctx: &egui::Context,ui:&mut Ui,scout_view:&mut ScoutViewPort){
    egui::Window::new("Scout View").collapsible(true).show(ctx, |ui| {

        let textures = scout_view.textures(ui);
        ui.label("How'd you do?");
        ui.horizontal(|ui|{
            ui.image(&textures[0],textures[0].size_vec2());
            ui.image(&textures[1],textures[1].size_vec2());
            ui.image(&textures[2],textures[2].size_vec2());
        });
        if ui.button("reload").clicked(){
            scout_view.clear_textures();
        }
    });
}