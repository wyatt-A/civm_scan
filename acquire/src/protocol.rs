use std::path::{Path, PathBuf};
use crate::build::SEQUENCE_LIB;
use crate::build;
use crate::args;
use serde::{Serialize,Deserialize};
use utils;


#[derive(Serialize,Deserialize)]
pub enum ProtocolState {
    Idle,
    Setup,
    Running,
}

#[derive(Serialize,Deserialize)]
pub struct Protocol {
    pub name:String,
    pub require_adjustments:bool,
    pub items:Vec<ProtocolItem>,
    pub state:ProtocolState,
}

impl Protocol {
    pub fn advance(&self) {
        match &self.state {
            ProtocolState::Idle => {
                //check for basic adjustments
            }
            ProtocolState::Setup => {
                // loop thru items, export setup protocols, save setup ppr paths
                // display buttons for running setups for different protocols, show user after they have been completed
                // on completion, transfer parameters to protocol that will run
                // when all setups are complete, advance state to running

            }
            ProtocolState::Running => {
                // show button for run protocol. If clicked, run protocol
            }
        }
    }

    pub fn build_setups(&self,study_dir:&Path,adj_file:&Path) -> Vec<PathBuf> {

        let mut setup_pprs = Vec::<PathBuf>::new();

        for item in &self.items {
            if item.require_setup {
                build::new_setup(&args::NewArgs {
                    alias: item.alias.clone(),
                    destination: item.setup_dir(study_dir).unwrap(),
                    adjustment_file: Some(adj_file.to_owned())
                });
                setup_pprs.push(utils::get_first_match(&item.setup_dir(study_dir).unwrap(),"*.ppr").unwrap());
            }
        }
        setup_pprs
    }

    pub fn load(filename:&Path) -> Self {
        let s = utils::read_to_string(filename,"json");
        serde_json::from_str(&s).unwrap()
    }
}


// some sequences require a setup stage, some do not

#[derive(Serialize,Deserialize)]
pub struct ProtocolItem {
    pub alias:String,
    pub require_setup:bool,
}

impl ProtocolItem {
    pub fn acquire_dir(&self,study_dir:&Path) -> PathBuf {
        study_dir.join(&self.alias)
    }
    pub fn setup_dir(&self,study_dir:&Path) -> Option<PathBuf> {
        match self.require_setup {
            true => Some(study_dir.join(format!("{}_setup",&self.alias))),
            false => None
        }
    }
}



#[test]
fn test(){
    let p = Protocol{
        name: String::from("5xfad"),
        require_adjustments: true,
        items: vec![
            ProtocolItem{
                alias: String::from("5xfad_fse"),
                require_setup: true
            },
            ProtocolItem{
                alias: String::from("5xfad_se"),
                require_setup: true
            },
        ],
        state: ProtocolState::Idle
    };

    let s = serde_json::to_string_pretty(&p).unwrap();
    utils::write_to_file(&Path::new("../test_env/protocols").join(p.name),"json",&s);

}