use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use eframe::egui;
use eframe::egui::Ui;
use acquire::protocol::Protocol;
use utils;
use crate::basic_adjustment::BasicAdjustmentPanel;
use crate::study_panel::StudyPanel;
use scan_control;

//const PROTOCOL_DIR:&str = "/Users/Wyatt/IdeaProjects/civm_scan/test_env/protocols";
const PROTOCOL_DIR:&str = r"C:\workstation\dev\civm_scan\test_env\protocols";



pub struct ProtocolPanel {
    selected_protocol:Option<String>,
    loaded_protocol:Option<Protocol>,
    setup_pprs:Option<Vec<PathBuf>>,
    setup_complete:bool,
    build_listener:Option<Receiver<bool>>,
    build_done:bool,
}

impl ProtocolPanel {
    pub fn default() -> Self {
        Self {
            selected_protocol:None,
            loaded_protocol: None,
            setup_pprs: None,
            setup_complete: false,
            build_listener: None,
            build_done: false,
        }
    }
}

pub fn protocol_panel(_ctx: &egui::Context,ui:&mut Ui,pp:&mut ProtocolPanel,sp:&mut StudyPanel,ba:&mut BasicAdjustmentPanel) {
    ui.label("protocol panel");
    if sp.study_dir().is_none() || ba.adjustment_file().is_none() {

        ui.label("create a study and run adjustments!");

    }else {

        let study_dir = sp.study_dir().clone().unwrap();
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
                            pp.setup_complete = false;
                        }

                        if pp.setup_pprs.is_some() && !pp.setup_complete {
                            // render all buttons to run the setup procedures
                            for ppr in pp.setup_pprs.clone().unwrap() {
                                let name = ppr.file_stem().unwrap().to_str().unwrap();
                                if ui.button(&format!("run {}", name)).clicked() {
                                    let ppr_for_thread = ppr.clone();
                                    std::thread::spawn(move ||{
                                        scan_control::command::setup_ppr(scan_control::args::RunDirectoryArgs {
                                            path: ppr_for_thread,
                                            cs_table: None,
                                            depth_to_search: None
                                        }).unwrap();
                                    });
                                }
                            }

                            if ui.button("setup complete").clicked(){
                                pp.setup_complete = true;
                            }
                        }

                        if pp.setup_complete {
                            if ui.button("build protocol").clicked(){

                                let setup_pprs = match p.require_adjustments {
                                    true => Some(pp.setup_pprs.clone().unwrap()),
                                    false => None
                                };

                                let adj_file = ba.adjustment_file();

                                let thread_protocol = (*p).clone();
                                let thread_study_dir = (*study_dir).to_owned();

                                let (tx, rx) = std::sync::mpsc::channel();
                                pp.build_listener = Some(rx);

                                std::thread::spawn(move||{
                                    thread_protocol.build_acquisition(&thread_study_dir,setup_pprs,adj_file);
                                    tx.send(true).unwrap();
                                });

                            }
                        }
                        // every frame, check that the build is done
                        match &pp.build_listener {
                            Some(listener) => {
                                match listener.try_recv() {
                                    Ok(_) => {
                                        // the build is done and the protocol is ready to run
                                        pp.build_done = true;
                                    }
                                    Err(_) => {
                                        if !pp.build_done{
                                            ui.label("building protocol ...");
                                        }
                                    }
                                }
                            }
                            None => {}
                        }

                        if pp.build_done && ui.button("run protocol").clicked(){
                            let thread_protocol = (*p).clone();
                            let thread_study_dir = (*study_dir).to_owned();
                            std::thread::spawn(move||{
                                thread_protocol.run_acquisition(&thread_study_dir);
                            });
                        }

                    }
                    None => {}
                }
            }
            None => {}
        }
}



}
