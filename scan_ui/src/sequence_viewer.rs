use std::path::Path;
use eframe::egui;
use eframe::egui::plot::{Line, LinkedAxisGroup, Plot, PlotPoints};
use eframe::egui::{Color32, Ui};
use acquire::build;
use seq_tools::event_block::EventGraph;
use seq_tools::execution::WaveformData;


pub struct SequenceViewer {
    selected_sequence:Option<String>,
    events:Option<Vec<EventGraph>>,
    grad_data:Option<Vec<(Vec<[f64; 2]>,(u8,u8,u8))>>,
    rf_data:Option<Vec<Vec<[f64;2]>>>,
    acq_data:Option<Vec<Vec<[f64;2]>>>,
    link_axis:LinkedAxisGroup,
    config_names:Option<Vec<String>>,
    selected_config:Option<String>,
}

impl SequenceViewer {
    pub fn default() -> Self {
        Self {
            selected_sequence:None,
            events:None,
            grad_data:None,
            rf_data:None,
            acq_data:None,
            link_axis:LinkedAxisGroup::new(true,false),
            config_names:None,
            selected_config:None,
        }
    }
}
fn render_grad(eg:&Vec<EventGraph>) -> Vec<(Vec<[f64; 2]>,(u8,u8,u8))> {
    let mut plot_objects = Vec::<(Vec<[f64; 2]>,(u8,u8,u8))>::new();
    eg.iter().for_each(|event|{
        match &event.wave_data {
            WaveformData::Grad(r, p, s) => {
                match r {
                    Some(data) => plot_objects.push((data.f64_pair(event.waveform_start as f64),(3, 252, 102))),
                    None => {}
                }
                match p {
                    Some(data) => plot_objects.push((data.f64_pair(event.waveform_start as f64),(3, 136, 252))),
                    None => {}
                }
                match s {
                    Some(data) => plot_objects.push((data.f64_pair(event.waveform_start as f64),(252, 3, 45))),
                    None => {}
                }
            }
            _=> {}
        }
    });
    plot_objects
}

fn render_rf(eg:&Vec<EventGraph>) -> Vec<Vec<[f64; 2]>> {
    let mut plot_objects = Vec::<Vec<[f64; 2]>>::new();
    eg.iter().for_each(|event| {
        match &event.wave_data {
            WaveformData::Rf(amp,_) => plot_objects.push(amp.f64_pair(event.waveform_start as f64)),
            _=> {}
        }
    });
    plot_objects
}

fn render_acq(eg:&Vec<EventGraph>) -> Vec<Vec<[f64; 2]>> {
    let mut plot_objects = Vec::<Vec<[f64; 2]>>::new();
    eg.iter().for_each(|event| {
        match &event.wave_data {
            WaveformData::Acq(amp) => plot_objects.push(amp.f64_pair(event.waveform_start as f64)),
            _=> {}
        }
    });
    plot_objects
}


// fn test_render(config_file:&Path) -> Vec<Vec<[f64; 2]>>{
//
//     let events = build::load_build_params(config_file).expect("cannot load parameters").instatiate().place_events().graphs_dynamic(2,0);
//
//     //let events = b.place_events().graphs_dynamic(2,0);
//
//     let mut plot_objects = Vec::<Vec<[f64; 2]>>::new();
//
//     events.iter().for_each(|event|{
//         match &event.wave_data {
//             WaveformData::Rf(amp,phase) => {
//                 plot_objects.push(amp.f64_pair(event.waveform_start as f64));
//                 //plot_objects.push(phase.f64_pair());
//             }
//             WaveformData::Grad(r,p,s) => {
//                 match r {
//                     Some(data) => plot_objects.push(data.f64_pair(event.waveform_start as f64)),
//                     None => {}
//                 }
//                 match p {
//                     Some(data) => plot_objects.push(data.f64_pair(event.waveform_start as f64)),
//                     None => {}
//                 }
//                 match s {
//                     Some(data) => plot_objects.push(data.f64_pair(event.waveform_start as f64)),
//                     None => {}
//                 }
//             }
//             WaveformData::Acq(data) => {
//                 plot_objects.push(data.f64_pair(event.waveform_start as f64))
//             }
//         }
//     });
//     plot_objects
// }

//pub enum WaveformData {
//     Rf(PlotTrace,PlotTrace),
//     Grad(Option<PlotTrace>,Option<PlotTrace>,Option<PlotTrace>),
//     Acq(PlotTrace),
// }


pub fn sequence_viewer(ctx: &egui::Context,ui:&mut Ui,sv:&mut SequenceViewer){
    egui::Window::new("Sequence Viewer").show(ctx, |ui| {

        let sequence_library = Path::new("./test_env/sequence_library");

        if ui.button("refresh").clicked(){
            sv.config_names = None;
        }

        let config_names = sv.config_names.get_or_insert_with(||{
            let files = utils::find_files(sequence_library,"json",true).unwrap();
            files.iter().map(|f| {
                let s = f.file_stem().unwrap();
                s.to_str().unwrap().to_string()
            }).collect()
        });



        egui::ComboBox::from_label("")
            .selected_text(format!("{}", sv.selected_sequence.clone().unwrap_or(String::from("None"))))
            .show_ui(ui, |ui| {
                for name in config_names {
                    if ui.selectable_value(&mut sv.selected_sequence,Some(name.clone()),name.as_str()).clicked(){
                        sv.events = None;
                        sv.grad_data = None;
                        sv.rf_data = None;
                        sv.acq_data = None;
                    };
                }
            });


        match &sv.selected_sequence {
            Some(sequence) => {
                let config = sequence_library.join(sequence).with_extension("json");
                let events = sv.events.get_or_insert_with(||{
                    let params = build::load_build_params(&config).expect("cannot load sequence");
                    params.place_events().graphs_dynamic(2,4)
                });

                let rf_data = sv.rf_data.get_or_insert_with(||render_rf(events));
                let acq_data = sv.acq_data.get_or_insert_with(||render_acq(events));
                let grad_data = sv.grad_data.get_or_insert_with(||render_grad(events));


                let link = LinkedAxisGroup::new(true,false);


                Plot::new("rf plot").show_axes([true,false]).link_axis(sv.link_axis.clone()).view_aspect(4.0).show(ui, |plot_ui| {
                    rf_data.iter().for_each(|p|{
                        let line = Line::new(PlotPoints::new(p.clone())).color(Color32::from_rgb(255,255,255));
                        plot_ui.line(line);
                    });
                });

                Plot::new("grad plot").show_axes([true,false]).link_axis(sv.link_axis.clone()).view_aspect(4.0).show(ui, |plot_ui| {
                    grad_data.iter().for_each(|p|{
                        let line = Line::new(PlotPoints::new(p.0.clone())).color(Color32::from_rgb(p.1.0,p.1.1,p.1.2));
                        plot_ui.line(line);
                    });
                });

                Plot::new("acq plot").show_axes([true,false]).link_axis(sv.link_axis.clone()).view_aspect(4.0).show(ui, |plot_ui| {
                    acq_data.iter().for_each(|p|{
                        let line = Line::new(PlotPoints::new(p.clone())).color(Color32::from_rgb(255,255,255));
                        plot_ui.line(line);
                    });
                });
            }
            None => {}
        }
    });
}
