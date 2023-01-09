use std::path::{Path, PathBuf};
use seq_tools::ppl::Orientation;
use crate::build;
use serde::{Serialize,Deserialize};
use scan_control::args::RunDirectoryArgs;
use scan_control::command::ScanControlError;
use crate::build::ContextParams;


pub struct Scout {
    scout_config:PathBuf,
    context:ContextParams,
    view_settings:ScoutViewSettings,
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
    pub fn new(scout_config:&Path,ctx:&ContextParams) -> Self {
        Self {
            scout_config:scout_config.to_owned(),
            context:ctx.clone(),
            view_settings:ScoutViewSettings::default()
        }
    }
    pub fn run(&self) -> Result<(),ScanControlError>{


        std::fs::create_dir_all(&self.context.export_dir).expect("unable to create directory!");
        self.view_settings.to_file(&self.context.export_dir.join("view_settings"));

        let params = build::load_scout_params(&self.scout_config).expect("cannot load parameters");

        build::build_scout_experiment(params,&self.context,&self.view_settings);

        scan_control::command::run_directory(RunDirectoryArgs{
            path: self.context.export_dir.clone(),
            cs_table: None,
            depth_to_search: Some(1),
            overwrite: Some(true)
        })?;
        Ok(())
    }
}