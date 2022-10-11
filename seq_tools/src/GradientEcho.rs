use crate::execution::ExecutionBlock;
use crate::rf_event::RfEvent;
use crate::pulse::{Hardpulse, Trapezoid, Pulse};
use crate::rf_state::RfStateType;
use crate::gradient_event::GradEvent;
use crate::gradient_matrix::{Matrix, DacValues, MatrixDriver, MatrixDriverType, EncodeStrategy, LinTransform, Dimension, DriverVar};
use crate::event_block::{GradEventType, EventQueue, Event};
use std::cell::RefCell;
use std::rc::Rc;
use crate::ppl::{VIEW_LOOP_COUNTER_VAR, Orientation, GradClock, PhaseUnit, BaseFrequency, PPL};
use crate::acq_event::{AcqEvent, SpectralWidth};
use crate::acq_event::SpectralWidth::{SW200kH, SW133kH};
use crate::event_block::EventPlacementType::{Origin, ExactFromOrigin, Before, After};
use crate::utils::{sec_to_clock, clock_to_sec};
use std::path::Path;
use std::fs::File;
use std::io::Write;
use crate::grad_cal;
use crate::ppl::BaseFrequency::Civm9p4T;
use crate::seqframe::SeqFrame;

#[test]
fn test(){
    let mut gep = GradientEcho3D::default_params();
    gep.samples = (420,256,256);
    gep.rep_time = 100E-3;
    let ge = GradientEcho3D::new(gep);
    let sim_mode = false;
    let acceleration = 1;
    ge.plot_export(4,32,"output");
    let ppl = ge.ppl_export(Civm9p4T(0.0),Orientation::CivmStandard,acceleration,sim_mode);
    let mut outfile = File::create("/mnt/d/dev/220920/gradient_echo.ppl").expect("cannot create file");
    outfile.write_all(ppl.print().as_bytes()).expect("cannot write to file");
    ge.seq_export(4);
}

#[derive(Clone)]
pub struct GradientEcho3DParams {
    fov: (f32, f32, f32),
    samples: (u16, u16, u16),
    sample_discards: u16,
    spectral_width: SpectralWidth,
    ramp_time: f32,
    phase_encode_time:f32,
    echo_time:f32,
    rep_time:f32
}

pub struct GradientEcho3D {
    params:GradientEcho3DParams,
    events:GradientEcho3DEvents,
}

pub struct GradientEcho3DEvents{
    excitation:RfEvent<Hardpulse>,
    phase_encode:GradEvent<Trapezoid>,
    readout:GradEvent<Trapezoid>,
    acquire:AcqEvent,
    rewinder:GradEvent<Trapezoid>,
}

impl GradientEcho3D {

    pub fn default_params() -> GradientEcho3DParams {
        GradientEcho3DParams {
            fov:(20.0,20.0,20.0),
            samples:(420,256,256),
            sample_discards:0,
            spectral_width:SpectralWidth::SW100kH,
            ramp_time:500E-6,
            phase_encode_time:1E-3,
            echo_time:10.0E-3,
            rep_time:50.0E-3,
        }
    }

    pub fn new(params:GradientEcho3DParams) -> GradientEcho3D {
        let events = Self::build_events(params.clone());
        Self {
            events,
            params
        }
    }

    pub fn build_events(params:GradientEcho3DParams) -> GradientEcho3DEvents {
        let n_read = params.samples.0;
        let n_discards = params.sample_discards;
        let n_phase = params.samples.1;
        let n_slice = params.samples.2;
        let fov_read = params.fov.0;
        let fov_phase = params.fov.1;
        let fov_slice = params.fov.2;
        let spectral_width = params.spectral_width;
        let read_ramp_time = params.ramp_time;
        let phase_encode_ramp_time = params.ramp_time;
        let phase_encode_time = params.phase_encode_time;

        let mat_count = Matrix::new_tracker();
        /* Acquisition with no discards and static phase of 0 degrees */
        let read_sample_time_sec = spectral_width.sample_time(n_read + n_discards);
        let read_grad_dac = spectral_width.fov_to_dac(fov_read);
        let read_matrix = Matrix::new_static("read_mat",DacValues::new(Some(read_grad_dac),None,None),&mat_count);
        let read_waveform = Trapezoid::new(read_ramp_time,read_sample_time_sec);
        let phase_encode_waveform = Trapezoid::new(phase_encode_ramp_time,phase_encode_time);
        let phase_grad_step = phase_encode_waveform.magnitude_net(1.0/fov_phase);
        let slice_grad_step = phase_encode_waveform.magnitude_net(1.0/fov_slice);
        let read_pre_phase_dac = phase_encode_waveform.magnitude_net(0.5*read_waveform.power_net(read_grad_dac as f32)) as i16;
        let pe_driver = MatrixDriver::new(DriverVar::Repetition,MatrixDriverType::PhaseEncode(EncodeStrategy::FullySampled(Dimension::_3D,n_phase as usize,Some(n_slice as usize))),None);
        //let phase_grad_step = 0.0;
        //let slice_grad_step = 0.0;
        let phase_encode_matrix = Matrix::new_driven(
            "c_pe_mat",
            pe_driver,
            LinTransform::new((None,Some(grad_cal::grad_to_dac(phase_grad_step) as f32),Some(grad_cal::grad_to_dac(slice_grad_step) as f32)),(None,None,None)),
            DacValues::new(Some(-read_pre_phase_dac),None,None),
            &mat_count
        );
        let rewinder_matrix = Matrix::new_derived(
            "c_rewind_mat",
            &Rc::new(phase_encode_matrix.clone()),
            LinTransform::new((Some(-1.0),Some(-1.0),Some(-1.0)),(None,None,None)),
            &mat_count
        );
        let excitation = RfEvent::new(
            "excitation",
            1,
            Hardpulse::new(100E-6),
            RfStateType::Adjustable(400),
            RfStateType::Static(0)
        );
        let phase_encode = GradEvent::new(
            (Some(phase_encode_waveform),
             Some(phase_encode_waveform),
             Some(phase_encode_waveform)),
            &phase_encode_matrix,
            GradEventType::Blocking,
            "phase_encode"
        );
        let readout = GradEvent::new(
            (Some(read_waveform),
             None,
             None),
            &read_matrix,
            GradEventType::NonBlocking,
            "readout"
        );
        let acquire = AcqEvent::new("acquire",spectral_width,n_read,n_discards,RfStateType::Static(0));
        let rewinder = GradEvent::new(
            (Some(phase_encode_waveform),
             Some(phase_encode_waveform),
             Some(phase_encode_waveform)),
            &rewinder_matrix,
            GradEventType::Blocking,
            "rewinder"
        );
        GradientEcho3DEvents{
            excitation,phase_encode,readout,acquire,rewinder
        }
    }
    fn place_events(&self) -> EventQueue{
        let excite = Event::new(self.events.excitation.as_reference(),Origin);
        let read = Event::new(self.events.readout.as_reference(),ExactFromOrigin(sec_to_clock(self.params.echo_time)));
        let acq = Event::new(self.events.acquire.as_reference(),ExactFromOrigin(sec_to_clock(self.params.echo_time)));
        println!("sampling start = {}",acq.borrow().block_start() + acq.borrow().execution.time_to_end());
        let pe = Event::new(self.events.phase_encode.as_reference(),Before(read.clone(),0));
        let re = Event::new(self.events.rewinder.as_reference(),After(acq.clone(),1500));
        EventQueue::new(&vec![
            excite,read,acq,pe,re
        ])
    }
    pub fn plot_export(&self,sample_period_us:usize,driver_val:u32,filename:&str){
        let file = Path::new(filename);
        let graphs = self.place_events().graphs_dynamic(sample_period_us,driver_val);
        let s = serde_json::to_string_pretty(&graphs).expect("cannot serialize");
        let mut f = File::create(file).expect("cannot create file");
        f.write_all(&s.as_bytes()).expect("trouble writing to file");
    }
    pub fn ppl_export(&self,base_frequency:BaseFrequency,orientation:Orientation,acceleration:u16,simulation_mode:bool) -> PPL {
        let averages = 1;
        let repetitions = (self.params.samples.1 as u32*self.params.samples.2 as u32);
        //let repetitions = 4;
        PPL::new(
            &mut self.place_events(),repetitions,averages,self.params.rep_time,base_frequency,
            r"d:\dev\220920\civm_grad.seq",r"d:\dev\220920\civm_rf.seq",
            orientation,GradClock::CPS20,PhaseUnit::PU90,acceleration,simulation_mode)
    }

    pub fn seq_export(&self,sample_period_us:usize,){
        let q = self.place_events();
        let (grad_params,rf_params) = q.ppl_seq_params(sample_period_us);
        //let path = std::env::current_dir().expect("cannot get current dir");
        let path = Path::new("/mnt/d/dev/220920");
        let grad_param = Path::new("civm_grad_params").with_extension("txt");
        let grad_param_path = path.join(grad_param);
        let rf_param = Path::new("civm_rf_params").with_extension("txt");
        let rf_param_path = path.join(rf_param);
        let mut rf_seq_file = File::create(rf_param_path).expect("cannot create file");
        rf_seq_file.write_all(&SeqFrame::format_as_bytes(&rf_params.unwrap())).expect("trouble writing to file");
        let mut grad_seq_file = File::create(grad_param_path).expect("cannot create file");
        grad_seq_file.write_all(&SeqFrame::format_as_bytes(&grad_params.unwrap())).expect("trouble writing to file");
    }
}
