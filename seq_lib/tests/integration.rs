use std::path::Path;
use seq_lib::pulse_sequence::{DiffusionWeighted, Initialize, SequenceParameters};
use seq_lib::se_dti;
use seq_tools::_utils;
use utils;
use seq_tools::grad_cal;
use seq_tools::grad_cal::{GAMMA};
use seq_tools::gradient_event::Channel;


//#[test]
/*fn b_val_test(){
    //todo!()
    // decompose max b-value into read phase slice
    // solve dac for each channel individually
    // recombined bvalue should be the max bval

    let mut params = se_dti::SeDtiParams::default();
    params.set_b_value(3000.0);
    params.set_b_vec((1.0,0.0,0.0));
    let t = params.instantiate();
    let eq = t.place_events();

    let e = eq.events();

    let g1 = e[1].borrow().event_graph_dynamic(2,0);
    let g2 = e[3].borrow().event_graph_dynamic(2,0);
    let g3 = e[4].borrow().event_graph_dynamic(2,0);
    let g4 = e[5].borrow().event_graph_dynamic(2,0);

    let echo_time = 14E-3;
    let tau = 7E-3;

    // determine start, stop, and step of waveform resampling
    // start at t = 0
    let start = 0.0;
    let end = echo_time;
    let sample_period = 1E-6;
    let n = ((end-start)/sample_period) as usize;
    let time_resample = _utils::linspace(start,end,n);

    let idx_samples = (0..n).map(|i| i as f32).collect();

    let tau_index = utils::interp1(&time_resample,&idx_samples,tau).unwrap() as usize;

    // sample each event, expecting only one or none to return a valid value

    let trace1 = g1.grad_channel(Channel::Read).unwrap();
    let trace2 = g2.grad_channel(Channel::Read).unwrap();
    let trace3 = g3.grad_channel(Channel::Read).unwrap();
    let trace4 = g4.grad_channel(Channel::Read).unwrap();

    let event_traces = vec![&trace1,&trace2,&trace3,&trace4];
    //let event_traces = vec![&trace1,&trace2];


    let mut waveform = Vec::<f32>::with_capacity(n);


    for t in time_resample {
        // sample each event in order until a valid value is found, or nothing is found
        let mut valid_value = true;
        for event in &event_traces {
            match utils::interp1(&event.x,&event.y,t) {
                Some(value) =>{
                    valid_value = true;
                    waveform.push(value);
                    break;
                }
                _=> {
                    valid_value = false;
                }
            }
        }
        if !valid_value {
            waveform.push(0.0);
        }
    }

    // convert grad dac to T/m


    let g:Vec<f32> = waveform.iter().map(|val| grad_cal::dac_to_tesla_per_meter(*val as i16)).collect();

    // transform to q-space
    let q = utils::cumtrapz(&g,Some(sample_period));

    let f = q[tau_index];
    let mut q_half = vec![0.0;q.len() - tau_index];
    q_half.copy_from_slice(&q[tau_index..q.len()]);

    // term1 is time integral of q^2
    let term1 = utils::trapz(&q.iter().map(|val| (*val)*(*val)).collect(),Some(sample_period));

    // second half of q-space

    let term2 = utils::trapz(&q_half,Some(sample_period))*4.0*f;

    let term3 = 4.0*f*f*(echo_time - tau);

    let bval = GAMMA*GAMMA*(term1 - term2 + term3)*1E-6; //s/mm^2


    //println!("term1 = {}",term1);
    //println!("term2 = {}",term2);
    //println!("term3 = {}",term3);


    // let arr = format!("{:?}",waveform);
    // let arr = arr.replace("[","");
    // let arr = arr.replace("]","");
    // utils::write_to_file(Path::new("./out"),"csv",&arr);
    //println!("bval = {}",bval);

}*/