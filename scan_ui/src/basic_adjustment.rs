use std::path::Path;
use eframe::egui;
use eframe::egui::{Color32, Ui};
use eframe::egui::plot::{Line, Plot, PlotPoints};
use acquire::adjustment;


pub struct BasicAdjustmentPanel {
}

impl BasicAdjustmentPanel{
    pub fn default() -> Self{
        BasicAdjustmentPanel{

        }
    }
}


pub fn basic_adjustemnt(ctx: &egui::Context,ui:&mut Ui,se:&mut BasicAdjustmentPanel){
    egui::Window::new("Adjustments").show(ctx, |ui| {
        ui.label("FID spectrum");
        Plot::new("frequency_plot").show_axes([true,true]).view_aspect(1.0).show(ui, |plot_ui| {
            let adj_data = adjustment::AdjustmentResults::from_file(Path::new("./test_data/adj_data/adjustment_results.json"));
            let line = Line::new(PlotPoints::new(adj_data.freq_spectrum.clone())).color(Color32::from_rgb(255,255,255));
            plot_ui.line(line);
        });

        ui.label("Spin Echo vs Stimulated Echo");
        Plot::new("diff_plot").show_axes([true,true]).view_aspect(1.0).show(ui, |plot_ui| {
            let adj_data = adjustment::AdjustmentResults::from_file(Path::new("./test_data/adj_data/adjustment_results.json"));
            let line = Line::new(PlotPoints::new(adj_data.rf_cal_spin_vs_stim.clone())).color(Color32::from_rgb(255,255,255));
            plot_ui.line(line);
        });
    });
}

