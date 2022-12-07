use spin_sim::spin::Spin;
use serde_json::{to_string, from_str};
use seq_lib::pulse_sequence::{AdjustmentParameters, Initialize};
use seq_lib::rfcal;
use seq_tools::execution::WaveformData;

#[test]
fn test1(){

    let seq = rfcal::RfCalParams::default().instantiate();
    let graphs = seq.place_events().graphs_dynamic(2,0);

    //Event{gradient:Vector::new(0.0,0.0,1.0),rf:Vector::new(0.0,0.0,0.0),duration:1.0E-6}


    // synchronize overlapping waveform graphs

    // get all the start and stop times of the events and group them by overlap

    for g in graphs.iter(){
        g.
    }





    match &graphs[0].wave_data {
        WaveformData::Rf(_, _) => {}
        WaveformData::Grad(_, _, _) => {}
        WaveformData::Acq(_) => {}
    }





    println!("test 1");


}