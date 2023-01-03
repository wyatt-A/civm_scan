use seq_lib::pulse_sequence::{DiffusionWeighted, Initialize, SequenceParameters};
use seq_lib::se_dti;


#[test]
fn b_val_test(){
    let mut params = se_dti::SeDtiParams::default();
    params.set_b_value(3000.0);
    params.set_b_vec((1.0,1.0,1.0));
    let t = params.instantiate();
    let eq = t.place_events();

    let e = eq.events();

    let g1 = e[1].borrow().event_graph_dynamic(2,0);

    let g2 = e[3].borrow().event_graph_dynamic(2,0);

    println!("{}",g1.waveform_start);

    println!("{:?}",g1.wave_data);


    // build g(t) for the 2 pulses


    g1.

    // clean the pulse endpoints (clamp to 0)
    let mut dac1 = g1.wave_data.read_channel().unwrap();

    // combine series of event graphs into a single event






}