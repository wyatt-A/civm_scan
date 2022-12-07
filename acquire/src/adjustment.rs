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

pub struct Adjustment {
    rf_cal:PathBuf, // these point to directories where calibration data has been collected
    freq_cal:PathBuf,
}


impl Adjustment {
    pub fn calc_freq_offset(&self) -> f32 {

        // first, load up adjustment params
        let cfg = utils::get_first_match(&self.freq_cal,"one_pulse.json").expect("one pulse file not found!");

        // check that the acq completed
        utils::get_first_match(&self.freq_cal,"one_pulse.ac").expect("ac file not found. Did the scan complete?");

        // load up the mrd file
        let mrd = utils::get_first_match(&self.freq_cal,"one_pulse.mrd").expect("mrd file not found!");

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
        let ft = utils::complex_abs(&utils::fft_shift(&utils::fft(&view,view.len())));
        let idx = utils::arg_max(&ft);
        // convert to freq offset
        let hz_per_sample = params.spectral_width.hertz() as f32/n_samples as f32;
        let dc_sample = n_samples/2 + 1;
        let obs_offset = (idx as f32 - dc_sample as f32) * hz_per_sample;
        obs_offset
    }

    /// calculate rf dac scale for 90 deg pulse in dac_per_sec
    /// this needs to be divided by the normalized magnitude of other pulses to get a dac value
    pub fn calc_rf_dac_per_sec(&self) -> f32 {
        // first, load up adjustment params
        let cfg = utils::get_first_match(&self.rf_cal,"rf_cal.json").expect("one pulse file not found!");
        // check that the acq completed
        utils::get_first_match(&self.rf_cal,"rf_cal.ac").expect("ac file not found. Did the scan complete?");
        // load up the mrd file
        let mrd = utils::get_first_match(&self.rf_cal,"rf_cal.mrd").expect("mrd file not found!");
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
            if diff[idx] < 0.0 && diff[idx+1] >- 0.0 {
                lower_idx = idx;
                break
            }
        }

        let rep_interp = lower_idx as f32 - diff[lower_idx]/(diff[lower_idx+1] - diff[lower_idx]);

        let dac = (rep_interp*dacs_per_rep as f32 + dac_offset as f32);

        let dac_seconds = dac*hardpulse_length;

        dac_seconds
    }
}


#[test]
fn adj_test(){
    let adj = Adjustment {
        rf_cal: Path::new("/Users/Wyatt/adj_data/rfadjustments").to_owned(),
        freq_cal: Path::new("/Users/Wyatt/adj_data/adjustments").to_owned(),
    };

    let offset = adj.calc_freq_offset();
    let rf_dac = adj.calc_rf_dac_per_sec();

    println!("rf_dac = {}",rf_dac);
}