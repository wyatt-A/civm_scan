use eframe::egui;
use eframe::egui::{Color32, Vec2, Widget};
use eframe::egui::plot::{Line, LinkedAxisGroup, Plot, PlotImage, PlotPoints, Points};
use eframe::glow::COLOR;
use seq_lib::pulse_sequence::{AdjustmentParameters, Initialize, SequenceParameters};
use seq_lib::{rfcal,fse_dti};
use seq_tools::event_block::EventGraph;
use seq_tools::execution::WaveformData;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Sequence Viewer",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

struct MyApp {
    message:String,
    events:Option<Vec<EventGraph>>,
    grad_data:Option<Vec<Vec<[f64;2]>>>,
    rf_data:Option<Vec<Vec<[f64;2]>>>,
    acq_data:Option<Vec<Vec<[f64;2]>>>,
    link_axis:LinkedAxisGroup
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            message:String::from("hello!"),
            events:None,
            grad_data:None,
            rf_data:None,
            acq_data:None,
            link_axis:LinkedAxisGroup::new(true,false),
        }
    }
}


fn render_grad(eg:&Vec<EventGraph>) -> Vec<Vec<[f64; 2]>> {
    let mut plot_objects = Vec::<Vec<[f64; 2]>>::new();
    eg.iter().for_each(|event|{
        match &event.wave_data {
            WaveformData::Grad(r, p, s) => {
                match r {
                    Some(data) => plot_objects.push(data.f64_pair(event.waveform_start as f64)),
                    None => {}
                }
                match p {
                    Some(data) => plot_objects.push(data.f64_pair(event.waveform_start as f64)),
                    None => {}
                }
                match s {
                    Some(data) => plot_objects.push(data.f64_pair(event.waveform_start as f64)),
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


fn test_render() -> Vec<Vec<[f64; 2]>>{
    let params = rfcal::RfCalParams::default();
    let b = params.instantiate();
    let events = b.place_events().graphs_dynamic(2,0);

    let mut plot_objects = Vec::<Vec<[f64; 2]>>::new();

    events.iter().for_each(|event|{
        match &event.wave_data {
            WaveformData::Rf(amp,phase) => {
                plot_objects.push(amp.f64_pair(event.waveform_start as f64));
                //plot_objects.push(phase.f64_pair());
            }
            WaveformData::Grad(r,p,s) => {
                match r {
                    Some(data) => plot_objects.push(data.f64_pair(event.waveform_start as f64)),
                    None => {}
                }
                match p {
                    Some(data) => plot_objects.push(data.f64_pair(event.waveform_start as f64)),
                    None => {}
                }
                match s {
                    Some(data) => plot_objects.push(data.f64_pair(event.waveform_start as f64)),
                    None => {}
                }
            }
            WaveformData::Acq(data) => {
                plot_objects.push(data.f64_pair(event.waveform_start as f64))
            }
        }
    });
    plot_objects
}

//pub enum WaveformData {
//     Rf(PlotTrace,PlotTrace),
//     Grad(Option<PlotTrace>,Option<PlotTrace>,Option<PlotTrace>),
//     Acq(PlotTrace),
// }

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {

            ui.heading(&self.message);

            let events = self.events.get_or_insert_with(||{
                //let params = rfcal::RfCalParams::default();
                let params = fse_dti::FseDtiParams::default();
                let b = params.instantiate();
                b.place_events().graphs_dynamic(2,4)
            });

            let rf_data = self.grad_data.get_or_insert_with(||render_rf(events));
            let acq_data = self.acq_data.get_or_insert_with(||render_acq(events));
            let grad_data = self.rf_data.get_or_insert_with(||render_grad(events));


            let link = LinkedAxisGroup::new(true,false);


            Plot::new("rf plot").show_axes([true,false]).link_axis(self.link_axis.clone()).view_aspect(4.0).show(ui, |plot_ui| {
                rf_data.iter().for_each(|p|{
                    let line = Line::new(PlotPoints::new(p.clone())).color(Color32::from_rgb(255,255,255));
                    plot_ui.line(line);
                });
            });

            Plot::new("grad plot").show_axes([true,false]).link_axis(self.link_axis.clone()).view_aspect(4.0).show(ui, |plot_ui| {
                grad_data.iter().for_each(|p|{
                    let line = Line::new(PlotPoints::new(p.clone())).color(Color32::from_rgb(255,255,255));
                    plot_ui.line(line);
                });
            });

            Plot::new("acq plot").show_axes([true,false]).link_axis(self.link_axis.clone()).view_aspect(4.0).show(ui, |plot_ui| {
                acq_data.iter().for_each(|p|{
                    let line = Line::new(PlotPoints::new(p.clone())).color(Color32::from_rgb(255,255,255));
                    plot_ui.line(line);
                });
            });
        });
    }
}