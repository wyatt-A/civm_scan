use std::borrow::Borrow;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use seq_tools::{_utils, grad_cal};
use seq_tools::event_block::{Event, EventGraph};
use seq_tools::execution::PlotTrace;
use seq_tools::grad_cal::GAMMA;
use seq_tools::gradient_event::Channel;
use utils::trapz;
use crate::pulse_sequence::{DiffusionWeighted, Initialize, SequenceParameters};
use crate::se_dti;

const EVENT_RENDER_SAMPLE_PERIOD_US:usize = 2;
const RENDER_SAMPLE_PERIOD:f32 = 1E-6;

struct StejskalTannerPrep {
    echo_time:f32,
    tau:f32,// time of 180 deg. pulse pulse relative to 90 deg. pulse
    diffusion_pulse1:Rc<RefCell<Event>>,
    diffusion_pulse2:Rc<RefCell<Event>>,
    other:Option<Vec<Rc<RefCell<Event>>>>,
}

impl StejskalTannerPrep {
    pub fn new(echo_time:f32,tau:f32,diff_pulse1:Rc<RefCell<Event>>,diff_pulse2:Rc<RefCell<Event>>,other:Option<Vec<Rc<RefCell<Event>>>>) -> Self {
        Self {
            echo_time,
            tau,
            diffusion_pulse1: diff_pulse1,
            diffusion_pulse2: diff_pulse2,
            other
        }
    }

    fn event_graphs(&self) -> Vec<EventGraph> {
        let mut out = Vec::<EventGraph>::new();

        let d1:&RefCell<Event> = self.diffusion_pulse1.borrow();
        let d1 = d1.borrow().event_graph_dynamic(EVENT_RENDER_SAMPLE_PERIOD_US,0);

        let d2:&RefCell<Event> = self.diffusion_pulse2.borrow();
        let d2 = d2.borrow().event_graph_dynamic(EVENT_RENDER_SAMPLE_PERIOD_US,0);

        out.push(d1);
        out.push(d2);
        match &self.other {
            Some(events) => {
                let event_graphs:Vec<EventGraph> = events.iter().map(|e|{
                    let g:&RefCell<Event> = e.borrow();
                    g.borrow().event_graph_dynamic(EVENT_RENDER_SAMPLE_PERIOD_US,0)
                }).collect();
                out.extend(event_graphs);
            },
            _=> {}
        }
        out
    }

    fn event_traces(&self,channel:Channel) -> Vec<PlotTrace> {
        let e = self.event_graphs();
        let mut plot_traces = Vec::<PlotTrace>::new();

        for event in e {
            match event.wave_data.grad_channel(channel) {
                Some(trace) => plot_traces.push(trace),
                None => {}
            }
        }
        plot_traces

        //e.iter().flat_map(|e| e.wave_data.grad_channel(channel).unwrap()).collect()
    }

    fn waveform(&self,time_axis:&Vec<f32>,channel:Channel,diffusion_pulse_dac:i16) -> Vec<f32> {

        let mut event_traces = Vec::<PlotTrace>::new();

        let d1:&RefCell<Event> = self.diffusion_pulse1.borrow();
        let graph = d1.borrow().event_graph_normalized(2);
        match graph.grad_channel(channel) {
            Some(g) => event_traces.push(g.scale_y(diffusion_pulse_dac as f32)),
            None => {},
        }

        let d2:&RefCell<Event> = self.diffusion_pulse2.borrow();
        let graph = d2.borrow().event_graph_normalized(2);
        match graph.grad_channel(channel) {
            Some(g) => event_traces.push(g.scale_y(diffusion_pulse_dac as f32)),
            None => {},
        }

        match &self.other {
            Some(other_events) => {
                for e in other_events {
                    let g:&RefCell<Event> = e.borrow();
                    let graph = g.borrow().event_graph_dynamic(2,0);
                    match graph.grad_channel(channel) {
                        Some(g) => event_traces.push(g),
                        None => {},
                    }
                }
            }
            _=> {}
        }

        let mut waveform = Vec::<f32>::with_capacity(time_axis.len());
        for t in time_axis {
            // sample each event in order until a valid value is found, or nothing is found
            let mut valid_value = true;
            for event in &event_traces {
                match utils::interp1(&event.x,&event.y,*t) {
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
        waveform
    }

    fn time_axis(&self) -> Vec<f32> {
        let n = (self.echo_time/RENDER_SAMPLE_PERIOD) as usize;
        _utils::linspace(0.0,self.echo_time,n)
    }

    pub fn waveforms(&self,diffusion_pulse_dacs:(i16,i16,i16)) -> (Vec<f32>,Vec<f32>,Vec<f32>){
        let t = self.time_axis();
        (
            self.waveform(&t,Channel::Read,diffusion_pulse_dacs.0),
            self.waveform(&t,Channel::Phase,diffusion_pulse_dacs.1),
            self.waveform(&t,Channel::Slice,diffusion_pulse_dacs.2),
        )
    }

    pub fn bvalue(&self,diffusion_pulse_dacs:(i16,i16,i16)) -> f32 {

        let t = self.time_axis();

        let wavs = self.waveforms(diffusion_pulse_dacs);

        let gr:Vec<f32> = wavs.0.iter().map(|val| grad_cal::dac_to_tesla_per_meter(*val as i16)).collect();
        let gp:Vec<f32> = wavs.1.iter().map(|val| grad_cal::dac_to_tesla_per_meter(*val as i16)).collect();
        let gs:Vec<f32> = wavs.2.iter().map(|val| grad_cal::dac_to_tesla_per_meter(*val as i16)).collect();

        let qr = utils::cumtrapz(&gr,Some(RENDER_SAMPLE_PERIOD));
        let qp = utils::cumtrapz(&gp,Some(RENDER_SAMPLE_PERIOD));
        let qs = utils::cumtrapz(&gs,Some(RENDER_SAMPLE_PERIOD));

        let idx_samples = (0..t.len()).map(|i| i as f32).collect();
        let tau_index = utils::interp1(&t,&idx_samples,self.tau).unwrap() as usize;

        let fr = qr[tau_index];
        let fp = qp[tau_index];
        let fs = qs[tau_index];

        let mut qr_half = vec![0.0;qr.len() - tau_index];
        qr_half.copy_from_slice(&qr[tau_index..qr.len()]);

        let mut qp_half = vec![0.0;qp.len() - tau_index];
        qp_half.copy_from_slice(&qp[tau_index..qp.len()]);

        let mut qs_half = vec![0.0;qs.len() - tau_index];
        qs_half.copy_from_slice(&qs[tau_index..qs.len()]);

        let q_squared:Vec<f32> = qr.iter().enumerate().map(|(idx,val)|{
            (val**val) + qp[idx]*qp[idx] + qs[idx]*qs[idx]
        }).collect();
        let term1 = utils::trapz(&q_squared,Some(RENDER_SAMPLE_PERIOD));
        let term2 = 4.0*(trapz(&qr_half,Some(RENDER_SAMPLE_PERIOD))*fr + trapz(&qp_half,Some(RENDER_SAMPLE_PERIOD))*fp + trapz(&qs_half,Some(RENDER_SAMPLE_PERIOD))*fs);
        let term3 = 4.0*(fr*fr + fp*fp + fs*fs)*(self.echo_time - self.tau);
        GAMMA*GAMMA*(term1 - term2 + term3)*1E-6 //s/mm^2
    }

}





pub enum DWMethod {
    StejskalTanner(f32) // time of 180 degree pulse (relative to 90)
}


pub trait Diffusion {
    fn grad_events(&self) -> Vec<RefCell<Event>>;
    fn calc_bvalue(&self,echo_time:f32,method:DWMethod);
}

fn render_events(events:&Vec<RefCell<Event>>) {

}

fn time_axis(echo_time:f32) -> Vec<f32> {
    let n = (echo_time/RENDER_SAMPLE_PERIOD) as usize;
    _utils::linspace(0.0,echo_time,n)
}


// lookup the index of specified time in time axis (rounded down)
fn time_index(time_axis:&Vec<f32>,tau:f32) -> Option<usize> {
    let idx_samples = (0..time_axis.len()).map(|i| i as f32).collect();
    let idx = utils::interp1(time_axis,&idx_samples,tau)?;
    Some(idx as usize)
}


#[test]
fn test(){
    let mut params = se_dti::SeDtiParams::default();
    params.set_b_value(3000.0);
    params.set_b_vec((1.0,0.0,0.0));
    let t = params.instantiate();
    let eq = t.place_events();

    let e = eq.events();

    let d1 = e[1].clone();
    let d2 = e[3].clone();
    let other = vec![e[4].clone(),e[5].clone()];

    let diffusion_model = StejskalTannerPrep::new(14E-3,7E-3,d1,d2,Some(other));


    let target_bval = 3000.0;
    let dir:(f32,f32,f32) = (1.0,1.0,1.0);

    // normalize direction
    let mag = (dir.0*dir.0 + dir.1*dir.1 + dir.2*dir.2).sqrt();
    let dir_n = (dir.0/mag,dir.1/mag,dir.2/mag);

    let dac_max = seq_tools::hardware_constants::GRAD_MAX_DAC as f32;

    let dacs =  _utils::linspace(0.0,dac_max,200);

    // loop thru dacs and calculate b-value for each. Use nearest neighbors to find the correct
    // dac value for the intended b-value
    // for d in dacs {
    //     dir_n.0*
    // }


    let bval = diffusion_model.bvalue((20,20,-20));


}

//[g,t] = Diffusion.get_grad_vs_time(this.diff_pulse_1,this.diff_pulse_2,dacs,this.echo_time);
//             tau = this.echo_time/2;
//             F = cumtrapz(t,g');
//             % F^2 = dot product with self
//             Fsq = dot(F,F,2);
//             % f = F(tau)
//             f = interp1(t,F,tau);
//             % evaluate first term of expression
//             tau_idx = interp1(t,1:numel(t),tau,'nearest');
//             t_half = t(tau_idx:end);
//             F_half = F(tau_idx:end,:);
//             term1 = trapz(t,Fsq); % T^2 S m^-2
//             term2 = dot(4*f,trapz(t_half,F_half));% T^2 S m^-2
//             term3 = 4*dot(f,f)*(this.echo_time - tau);% T^2 S m^-2
//             % T^-2 S^-2   (T^2 S m^-2)
//             bval = (GradCal.gamma*2*pi)^2*(term1 - term2 + term3)*1e-6; % s/mm^2;