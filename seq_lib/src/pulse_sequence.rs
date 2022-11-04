use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use seq_tools::event_block::EventQueue;
use seq_tools::seqframe::SeqFrame;
use build_sequence::build_directory::{Config,build_directory};
use seq_tools::ppl::{BaseFrequency, GradClock, Orientation, PhaseUnit, PPL};
use serde_json;
use serde::{Serialize,Deserialize};

#[derive(Clone,Serialize,Deserialize)]
pub struct PPLBaseParams {
    pub n_averages:u16,
    pub n_repetitions:u32,
    pub rep_time:f32,
    pub base_frequency:BaseFrequency,
    pub orientation:Orientation,
    pub grad_clock:GradClock,
    pub phase_unit:PhaseUnit,
    pub view_acceleration:u16,
    pub waveform_sample_period_us:usize,
}

impl PPLBaseParams {
    pub fn to_file(&self,file_path:&Path) {
        let mut f = File::create(file_path).expect(&format!("cannot create file {:?}",file_path));
        let str = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        f.write_all(str.as_bytes()).expect("trouble writing to file");
    }
    pub fn from_file(file_path:&Path) -> Self {
        let mut f = File::open(file_path).expect(&format!("cannot open file {:?}",file_path));
        let mut in_str = String::new();
        f.read_to_string(&mut in_str).expect("trouble reading file");
        serde_json::from_str(&in_str).expect("cannot deserialize struct")
    }
}

pub trait PulseSequence {
    fn name() -> String;
    fn default() -> Self;
    fn from_file(filepath:&Path) -> Self;
    fn to_file(&self,filepath:&Path);
}

pub trait Setup {
    fn set_mode(&mut self);
    fn set_repetitions(&mut self,n_reps:u32);
}

pub trait Build {
    fn place_events(&self) -> EventQueue;
    fn base_params(&self) -> PPLBaseParams;
    fn seq_file_export(&self,sample_period_us:usize,filepath:&str) {
        let q = self.place_events();
        let (grad_params,rf_params) = q.ppl_seq_params(sample_period_us);
        let path = Path::new(filepath);
        let config = Config::load();
        let grad_param_path = path.join(config.grad_param_filename());
        let rf_param_path = path.join(config.rf_param_filename());
        let mut rf_seq_file = File::create(rf_param_path).expect("cannot create file");
        rf_seq_file.write_all(&SeqFrame::format_as_bytes(&rf_params.unwrap())).expect("trouble writing to file");
        let mut grad_seq_file = File::create(grad_param_path).expect("cannot create file");
        grad_seq_file.write_all(&SeqFrame::format_as_bytes(&grad_params.unwrap())).expect("trouble writing to file");
    }
    fn ppl_export(&mut self,filepath:&Path,ppr_name:&str,sim_mode:bool,build:bool) {
        let name = Path::new(ppr_name).with_extension("ppl");
        let base_params = self.base_params();
        let config = Config::load();
        let grad_seq_path = Path::new(filepath).join(config.grad_seq());
        let rf_seq_path = Path::new(filepath).join(config.rf_seq());
        self.seq_file_export(base_params.waveform_sample_period_us, filepath.as_os_str().to_str().unwrap());
        let ppl = PPL::new(
            &mut self.place_events(),
            base_params.n_repetitions,
            base_params.n_averages,
            base_params.rep_time,
            base_params.base_frequency.clone(),
            grad_seq_path.into_os_string().to_str().unwrap(),
            rf_seq_path.into_os_string().to_str().unwrap(),
            base_params.orientation.clone(),
            base_params.grad_clock.clone(),
            base_params.phase_unit.clone(),
            base_params.view_acceleration,
            sim_mode
        );
        let filename = filepath.join(name);
        let ppr_filename = filepath.join(ppr_name).with_extension("ppr");
        let ppr_str = ppl.print_ppr(&filename);
        let mut out_ppr = File::create(&ppr_filename).expect("cannot create file");
        out_ppr.write_all(ppr_str.as_bytes()).expect("cannot write to file");

        let mut outfile = File::create(&filename).expect("cannot create file");
        outfile.write_all(ppl.print().as_bytes()).expect("cannot write to file");
        if build {
            build_directory(filepath);
        }
    }
}
