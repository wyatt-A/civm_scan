use std::path::{Path, PathBuf};
use eframe::egui;
use eframe::egui::Ui;
use acquire::protocol::Protocol;
use utils;
use crate::basic_adjustment::BasicAdjustmentPanel;
use crate::study_panel::StudyPanel;
use scan_control;

const PROTOCOL_DIR:&str = "/Users/Wyatt/IdeaProjects/civm_scan/test_env/protocols";



pub struct ProtocolPanel {
    selected_protocol:Option<String>,
    loaded_protocol:Option<Protocol>,
    setup_pprs:Option<Vec<PathBuf>>,
}

impl ProtocolPanel {
    pub fn default() -> Self {
        Self {
            selected_protocol:None,
            loaded_protocol: None,
            setup_pprs: None
        }
    }

}

pub fn protocol_panel(_ctx: &egui::Context,ui:&mut Ui,pp:&mut ProtocolPanel,sp:&mut StudyPanel,ba:&mut BasicAdjustmentPanel) {
    ui.label("protocol panel");
    if sp.study_dir().is_none() || ba.adjustment_file().is_none() {

        ui.label("create a study and run adjustments!");

    }else {

        let study_dir = sp.study_dir().unwrap();
        let adj_file = &ba.adjustment_file().unwrap();

        match utils::get_all_matches(Path::new(PROTOCOL_DIR), &format!("*.json")) {
            Some(items) => {
                let protocol_names: Vec<String> = items.iter().map(|file| file.file_stem().unwrap().to_str().unwrap().to_string()).collect();

                egui::ComboBox::from_label("")
                    .selected_text(format!("{}", pp.selected_protocol.clone().unwrap_or(String::from("None"))))
                    .show_ui(ui, |ui| {
                        for protocol in protocol_names {
                            if ui.selectable_value(&mut pp.selected_protocol, Some(protocol.clone()), protocol.as_str()).clicked() {
                                let p = Protocol::load(&Path::new(PROTOCOL_DIR).join(protocol.as_str()));
                                pp.loaded_protocol = Some(p);
                            };
                        }
                    });

                match &pp.loaded_protocol {
                    Some(p) => {
                        if ui.button("start setup").clicked() {
                            pp.setup_pprs = Some(p.build_setups(study_dir, adj_file));
                        }

                        if pp.setup_pprs.is_some() {
                            // render all buttons to run the setup procedures
                            for ppr in pp.setup_pprs.clone().unwrap() {
                                let name = ppr.file_stem().unwrap().to_str().unwrap();
                                if ui.button(&format!("run {}", name)).clicked() {
                                    scan_control::command::setup_ppr(scan_control::args::RunDirectoryArgs {
                                        path: ppr.clone(),
                                        cs_table: None,
                                        depth_to_search: None
                                    });
                                }
                            }
                        }
                    }
                    None => {}
                }
            }
            None => {}
        }
}



}
