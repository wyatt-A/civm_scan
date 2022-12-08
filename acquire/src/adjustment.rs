/*
    Here we are implementing adjustment calculations
 */

use std::path::{Path, PathBuf};
use mr_data::mrd::MRData;
use seq_lib::one_pulse::OnePulseParams;
use seq_lib::pulse_sequence::Initialize;
use utils;
use ndarray::{s,Array6,Order};
use seq_lib::rfcal::RfCalParams;
use serde::{Serialize,Deserialize};
use crate::build;
use crate::args::NewAdjArgs;
use scan_control;

pub struct Adjustment {
    rf_cal_config:PathBuf,
    freq_cal_config:PathBuf,
    rf_cal_dir:PathBuf, // these point to directories where calibration data has been collected
    freq_cal_dir:PathBuf,
    results_file:PathBuf,
}

impl Adjustment {
    pub fn new(freq_cal_config:&Path,rf_cal_config:&Path,results_dir:&Path) -> Self {
        Self {
            rf_cal_config: rf_cal_config.to_owned(),
            freq_cal_config: freq_cal_config.to_owned(),
            rf_cal_dir: results_dir.join("rfcal"),
            freq_cal_dir: results_dir.join("freqcal"),
            results_file: results_dir.join("adjustment_results"),
        }
    }
}



#[derive(Serialize,Deserialize)]
pub struct AdjustmentResults {
    pub obs_freq_offset:f32,
    pub rf_dac_seconds:f32,
    pub freq_spectrum:Vec<[f64;2]>,
    pub rf_cal_spin_vs_stim:Vec<[f64;2]>,
}

impl AdjustmentResults {
    pub fn to_file(&self,filename:&Path) {
        let s = serde_json::to_string_pretty(&self).expect("cannot serialize struct");
        utils::write_to_file(filename,"json",&s);
    }
    pub fn from_file(file_path:&Path) -> Self {
        let s = utils::read_to_string(file_path,"json");
        serde_json::from_str(&s).expect("unable to parse json")
    }
}


impl Adjustment {
    pub fn calc_freq_offset(&self) -> (Vec<[f64;2]>,f32) {
        // first, load up adjustment params
        let cfg = utils::get_first_match(&self.freq_cal_dir, "one_pulse.json").expect("one pulse file not found!");

        // check that the acq completed
        utils::get_first_match(&self.freq_cal_dir, "one_pulse.ac").expect("ac file not found. Did the scan complete?");

        // load up the mrd file
        let mrd = utils::get_first_match(&self.freq_cal_dir, "one_pulse.mrd").expect("mrd file not found!");

        // load up parameter file for some meta data
        let params = OnePulseParams::load(&cfg);

        // load
        let raw = MRData::new(&mrd);
        let arr = raw.complex_array();

        // get number of samples per view
        let n_samples = arr.shape()[5];
        // grab the last repetition
        let rep = arr.shape()[4] - 1;

        // index into array and convert to a plain vector
        let slice = arr.slice(s![0,0,0,0,rep,..]);
        let view = slice.to_shape((n_samples,Order::RowMajor)).expect("incorrect shape for array").to_vec();

        // get peak location of the freq spectrum
        let mut ft = utils::complex_abs(&utils::fft_shift(&utils::fft(&view,view.len())));
        let idx = utils::arg_max(&ft);
        // convert to freq offset
        let hz_per_sample = params.spectral_width.hertz() as f32/n_samples as f32;
        let dc_sample = n_samples/2 + 1;
        let obs_offset = (idx as f32 - dc_sample as f32) * hz_per_sample;

        let mut freq_axis:Vec<f64> = (0..ft.len()).map(|idx|hz_per_sample as f64 * idx as f64).collect();
        //freq_axis.rotate_right(ft.len()/2);

        let plot_points:Vec<[f64;2]> = ft.iter().enumerate().map(|(idx,y)|{
            [freq_axis[idx],*y as f64]
        }).collect();
        (plot_points,obs_offset)
    }
    /// calculate rf dac scale for 90 deg pulse in dac_per_sec
    /// this needs to be divided by the normalized magnitude of other pulses to get a dac value
    pub fn calc_rf_dac_seconds(&self) -> (Vec<[f64;2]>,f32) {
        // first, load up adjustment params
        let cfg = utils::get_first_match(&self.rf_cal_dir, "rf_cal.json").expect("one pulse file not found!");
        // check that the acq completed
        utils::get_first_match(&self.rf_cal_dir, "rf_cal.ac").expect("ac file not found. Did the scan complete?");
        // load up the mrd file
        let mrd = utils::get_first_match(&self.rf_cal_dir, "rf_cal.mrd").expect("mrd file not found!");
        // load up parameter file for some meta data
        let params = RfCalParams::load(&cfg);

        let hardpulse_length = params.rf_duration;

        let dacs_per_rep = (params.end_rf_dac - params.start_rf_dac)/(params.n_repetitions as i16 - 1);
        let dac_offset = params.start_rf_dac;

        let raw = MRData::new(&mrd).complex_array();

        // we are expecting two echos
        let spin_echo = raw.slice(s![0,0,0,0,..,..]);
        let stim_echo = raw.slice(s![0,1,0,0,..,..]);

        let spin_echo_max:Vec<f32> = spin_echo.outer_iter().map(|rep|{
            utils::max(&utils::complex_abs(&rep.to_vec()))
        }).collect();
        let stim_echo_max:Vec<f32> = stim_echo.outer_iter().map(|rep|{
            utils::max(&utils::complex_abs(&rep.to_vec()))
        }).collect();

        let diff:Vec<f32> = (0..spin_echo_max.len()).map(|idx|{
            spin_echo_max[idx] - stim_echo_max[idx]
        }).collect();

        let midx = utils::arg_min(&diff);

        // starting at midx, find the zero-crossing point
        let mut lower_idx = 0;
        for idx in midx..(diff.len()-1) {
            if diff[idx] < 0.0 && diff[idx+1] >= 0.0 {
                lower_idx = idx;
                break
            }
        }

        let rep_interp = lower_idx as f32 - diff[lower_idx]/(diff[lower_idx+1] - diff[lower_idx]);

        let dac = rep_interp*dacs_per_rep as f32 + dac_offset as f32;

        let dac_seconds = dac*hardpulse_length;

        let dac_vs_signal_difference = diff.iter().enumerate().map(|(idx,d)|{
            [(idx as f64)*(dacs_per_rep as f64) + dac_offset as f64,*d as f64]
        }).collect();

        (dac_vs_signal_difference,dac_seconds)
    }


    /// this will run a frequency and rf_calibration adjustment
    pub fn run(&self) {


        // proper rf calibration depends on a frequency calibration being performed

        // run the frequency calibration routine

        let params = build::load_adj_params(&self.freq_cal_config);
        build::build_adj(params,&self.freq_cal_dir,false);

        // scan_control::command::run_directory(scan_control::args::RunDirectoryArgs{
        //     path: fcal_dir.clone(),
        //     cs_table: None,
        //     depth_to_search: Some(0)
        // });

        // analyze the results
        let (freq_spec,freq_offset) = self.calc_freq_offset();

        // run rf calibration with the found frequency offset
        let mut params = build::load_adj_params(&self.rf_cal_config);
        params.set_freq_offset(freq_offset);
        build::build_adj(params,&self.rf_cal_dir,false);

        // scan_control::command::run_directory(scan_control::args::RunDirectoryArgs{
        //     path: rfcal_dir.clone(),
        //     cs_table: None,
        //     depth_to_search: Some(0)
        // });

        let (signal_difference,rf_dac_secs) = self.calc_rf_dac_seconds();

        AdjustmentResults {
            obs_freq_offset: freq_offset,
            rf_dac_seconds:rf_dac_secs ,
            freq_spectrum:freq_spec,
            rf_cal_spin_vs_stim:signal_difference
        }.to_file(&self.results_file);

    }
}


#[test]
fn adj_test(){
    let adj = Adjustment {
        rf_cal_config:Path::new("/Users/Wyatt/sequence_library/rf_cal.json").to_owned(),
        freq_cal_config: Path::new("/Users/Wyatt/sequence_library/1p.json").to_owned(),
        rf_cal_dir: Path::new("/Users/Wyatt/adj_data/adj_01/rfadjustments").to_owned(),
        freq_cal_dir: Path::new("/Users/Wyatt/adj_data/adj_01/adjustments").to_owned(),
        results_file: Path::new("/Users/Wyatt/adj_data/adjustments/results.json").to_owned()
    };

    adj.run();


}