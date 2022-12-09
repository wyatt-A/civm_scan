use std::path::{Path, PathBuf};
use seq_tools::ppl::Orientation;
use crate::build;
use crate::args;
use serde::{Serialize,Deserialize};
use scan_control::args::RunDirectoryArgs;


pub struct Scout {
    scout_config:PathBuf,
    result_dir:PathBuf,
    view_settings:ScoutViewSettings
}


#[derive(Serialize,Deserialize)]
pub struct ScoutViewSettings {
    pub orientations:[Orientation;3],
    pub fields_of_view:[(f32,f32);3],
    pub samples:[(u16,u16);3]
}

impl ScoutViewSettings {
    pub fn default() -> Self {
        Self {
            orientations: [Orientation::Scout0,Orientation::Scout1,Orientation::Scout2],
            fields_of_view: [(12.0,12.0),(19.7,12.0),(19.7,12.0)],
            samples: [(128,128),(210,128),(210,128)],
        }
    }
    pub fn to_file(&self,filepath:&Path) {
        let s = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        utils::write_to_file(filepath,"json",&s);
    }
    pub fn from_file(filepath:&Path) -> Self {
        let s = utils::read_to_string(filepath,"json");
        serde_json::from_str(&s).expect("cannot parse json")
    }
}



impl Scout {
    pub fn new(scout_config:&Path,result_dir:&Path) -> Self {
        Self {
            scout_config:scout_config.to_owned(),
            result_dir:result_dir.to_owned(),
            view_settings:ScoutViewSettings::default()
        }
    }
    pub fn run(&self){
        &self.view_settings.to_file(&self.result_dir.join("view_settings"));
        let params = build::load_scout_params(&self.scout_config);
        build::build_scout_experiment(params,&self.view_settings,&self.result_dir, false);
        scan_control::command::run_directory(RunDirectoryArgs{
            path: self.result_dir.clone(),
            cs_table: None,
            depth_to_search: Some(1),
        });
    }

    //pub fn view(&self) ->


}