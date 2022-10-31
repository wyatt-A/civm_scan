use std::cell::RefCell;
use std::rc::Rc;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use seq_tools::grad_cal;
use seq_tools::acq_event::{AcqEvent, SpectralWidth};
use seq_tools::event_block::{Event, EventQueue, GradEventType};
use seq_tools::event_block::EventPlacementType::{After, Before, ExactFromOrigin, Origin};
use seq_tools::execution::ExecutionBlock;
use seq_tools::gradient_event::GradEvent;
use seq_tools::gradient_matrix::{DacValues, Dimension, DriverVar, EncodeStrategy, LinTransform, Matrix, MatrixDriver, MatrixDriverType};
use seq_tools::ppl::{BaseFrequency, GradClock, Orientation, PhaseUnit, PPL};
use seq_tools::pulse::{CompositeHardpulse, Hardpulse, Pulse, Trapezoid};
use seq_tools::rf_event::RfEvent;
use seq_tools::rf_state::{PhaseCycleStrategy, RfDriver, RfDriverType, RfStateType};
use seq_tools::seqframe::SeqFrame;
use seq_tools::utils;
use seq_tools::utils::{ms_to_clock, sec_to_clock, us_to_clock};

#[test]
fn test(){
    let mep = RfCal::default_params();
    let sim_mode = false;
    let acceleration = 1;
    let output_dir = Path::new("/mnt/d/dev/rf_cal");
    let me = RfCal::new(mep);
    let ppl = me.ppl_export(BaseFrequency::civm9p4t(0.0),Orientation::CivmStandard,acceleration,sim_mode);
    let filename = output_dir.join("rf_cal.ppl");
    let mut outfile = File::create(&filename).expect("cannot create file");
    outfile.write_all(ppl.print().as_bytes()).expect("cannot write to file");
    me.seq_export(2,output_dir.to_str().unwrap());
    let ppr_filename = output_dir.join("setup.ppr");
    let mut outfile = File::create(ppr_filename).expect("cannot create file");
    outfile.write_all(ppl.print_ppr(Path::new("d:/dev/rf_cal/rf_cal.ppl")).as_bytes()).expect("cannot write to file");

    //me.seq_export(4,".");
}

#[derive(Clone)]
pub struct RfCalParams {
    samples: u16,
    sample_discards: u16,
    spectral_width: SpectralWidth,
    tau_1:f32,
    tau_2:f32,
    rep_time:f32,
}

pub struct RfCal {
    params: RfCalParams,
    events: RfCalEvents,
}

pub struct RfCalEvents {
    excitation:RfEvent<Hardpulse>,
    slice_select:GradEvent<Trapezoid>,
    acquire:AcqEvent,
}

impl RfCal {

    pub fn default_params() -> RfCalParams {
        RfCalParams {
            samples:256,
            sample_discards:0,
            spectral_width:SpectralWidth::SW100kH,
            tau_1:2.5E-3,
            tau_2:2.5E-3,
            rep_time:1.0,
        }
    }

    pub fn new(params: RfCalParams) -> RfCal {
        let events = Self::build_events(params.clone());
        Self {
            events,
            params
        }
    }

    pub fn build_events(params: RfCalParams) -> RfCalEvents {
        let n_read = params.samples;
        let n_discards = params.sample_discards;
        let spectral_width = params.spectral_width;

        let mat_count = Matrix::new_tracker();

        /* ACQUISITION */
        /* define the acq event with a static phase of 0 */
        let acquire = AcqEvent::new(
            "acquire",
            spectral_width,
            n_read,
            n_discards,
            RfStateType::Static(0)
        );

        /* EXCITATION */
        /* excitation pulse is a single hard pulse with a user-adjustable power and static phase set
        to 0
        */
        let excite_waveform = Hardpulse::new(100E-6);
        let excite_power = RfStateType::Adjustable(513,None);
        let excite_phase = RfStateType::Static(0);
        let excitation = RfEvent::new(
            "excitation",
            1,
            excite_waveform,
            excite_power,
            excite_phase
        );

       /* SLICE SELECT */

        let duration = params.tau_1*3.0 + params.tau_2 + 3.0E-3;
        let slice_select_waveform = Trapezoid::new(400E-6,duration);
        let slice_select_dac = 500;
        let slice_select_mat = Matrix::new_static("slice_select_mat",DacValues::new(Some(0),Some(0),Some(slice_select_dac)),
                                                  (false,false,false),false,
                                                  &mat_count);
        let slice_select = GradEvent::new(
            (None,
             None,
             Some(slice_select_waveform)),
            &slice_select_mat,
            GradEventType::NonBlocking,
            "slice_select"
        );

        /* export events for placement */
        RfCalEvents {
            excitation,
            acquire,
            slice_select
        }
    }
    fn place_events(&self) -> EventQueue{

        let tau = self.params.tau_1;
        let tau_2 = self.params.tau_2;

        let excite1 = Event::new(self.events.excitation.as_reference(),Origin);
        let excite2 = Event::new(self.events.excitation.as_reference(),ExactFromOrigin(sec_to_clock(tau)));
        let acq1 = Event::new(self.events.acquire.as_reference(),ExactFromOrigin(sec_to_clock(2.0*tau)));
        let excite3 = Event::new(self.events.excitation.as_reference(),ExactFromOrigin(sec_to_clock(2.0*tau+tau_2)));
        let acq2 = Event::new(self.events.acquire.as_reference(),ExactFromOrigin(sec_to_clock(3.0*tau + tau_2)));

        let ss = Event::new(self.events.slice_select.as_reference(),ExactFromOrigin(sec_to_clock(2.0*tau + 1.5E-3)));

        EventQueue::new(&vec![
            excite1,
            excite2,
            excite3,
            acq1,
            acq2,
            ss
            ])
    }
    // pub fn plot_export(&self,sample_period_us:usize,driver_val:u32,filename:&str){
    //     let file = Path::new(filename);
    //     let graphs = self.place_events().graphs_dynamic(sample_period_us,driver_val);
    //     let s = serde_json::to_string_pretty(&graphs).expect("cannot serialize");
    //     let mut f = File::create(file).expect("cannot create file");
    //     f.write_all(&s.as_bytes()).expect("trouble writing to file");
    // }
    pub fn ppl_export(&self,base_frequency:BaseFrequency,orientation:Orientation,acceleration:u16,simulation_mode:bool) -> PPL {
        let averages = 1;
        //let repetitions = (self.params.samples.1 as u32*self.params.samples.2 as u32);
        let repetitions = 2;
        PPL::new(
            &mut self.place_events(),repetitions,averages,self.params.rep_time,base_frequency,
            r"d:\dev\rf_cal\civm_grad.seq",r"d:\dev\rf_cal\civm_rf.seq",
            orientation,GradClock::CPS20,PhaseUnit::Min,acceleration,simulation_mode)
    }

    pub fn seq_export(&self,sample_period_us:usize,filepath:&str){
        let q = self.place_events();
        let (grad_params,rf_params) = q.ppl_seq_params(sample_period_us);
        //let path = std::env::current_dir().expect("cannot get current dir");
        let path = Path::new(filepath);
        let grad_param = Path::new("civm_grad_params").with_extension("txt");
        let grad_param_path = path.join(grad_param);
        let rf_param = Path::new("civm_rf_params").with_extension("txt");
        let rf_param_path = path.join(rf_param);
        let mut rf_seq_file = File::create(rf_param_path).expect("cannot create file");
        rf_seq_file.write_all(&SeqFrame::format_as_bytes(&rf_params.unwrap())).expect("trouble writing to file");
        let mut grad_seq_file = File::create(grad_param_path).expect("cannot create file");
        grad_seq_file.write_all(&SeqFrame::format_as_bytes(&grad_params.unwrap_or(String::from("")))).expect("trouble writing to file");
    }
}
