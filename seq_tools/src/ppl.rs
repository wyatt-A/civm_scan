use std::path::{Path};
use serde::{Deserialize, Serialize};
use crate::command_string::CommandString;
use crate::event_block::{EventQueue, EventQueueError};
use crate::ppl_function;
use crate::ppl_constants::{
    AVERAGES_LOOP_COUNTER_VAR, CALC_MATRIX, CIVM_INCLUDE, GRAD_FN_INCLUDE, GRAD_STRENGTH_VAR,
    LONG_TEMPVAL_VAR_NAME, LUT_INCLUDE, LUT_INDEX_VAR_NAME, LUT_TEMPVAL_VAR_NAME_1,
    LUT_TEMPVAL_VAR_NAME_2, NO_AVERAGES_MAX, NO_AVERAGES_MIN, NO_AVERAGES_VAR, NO_DISCARD_MAX,
    NO_DISCARD_MIN, NO_DISCARD_VAR, NO_ECHOES_MAX, NO_ECHOES_MIN, NO_ECHOES_VAR, NO_SAMPLES_MAX,
    NO_SAMPLES_MIN, NO_SAMPLES_VAR, NO_VIEWS_MAX, NO_VIEWS_MIN, NO_VIEWS_VAR, RECEIVER_MASK_MAX,
    RECEIVER_MASK_MIN, RECEIVER_MASK_VAR, RF_FN_INCLUDE, SPECTRAL_WIDTH_VAR, STD_FN_INCLUDE,
    STD_GRAD_SEQ, STD_RF_SEQ, VIEW_LOOP_COUNTER_VAR
};
use crate::grad_cal;
use crate::hardware_constants::{GRAD_SEQ_FILE_LABEL, RF_SEQ_FILE_LABEL};
use crate::loop_structure::FlatLoopStructure;
use crate::ppl_header::{BaseFrequency, DspRoutine, Header};
use crate::seqframe::FrameType;


#[derive(Clone,Serialize,Deserialize)]
pub enum Orientation {
    CivmStandard,
    Ortho1,
    Ortho2,
    Simulation,
    Scout0,
    Scout1,
    Scout2,
}

#[derive(Clone,Serialize,Deserialize)]
pub enum GradClock {
    CPS20
}

#[derive(Clone,Serialize,Deserialize)]
pub enum PhaseUnit {
    PU90,
    Min
}

impl GradClock {
    pub fn clocks_per_sample(&self) -> i16 {
        match self {
            GradClock::CPS20 => 20
        }
    }
    pub fn print(&self) -> String {
        vec![
            ppl_function::grad_deglitch(),
            ppl_function::grad_clock(self.clocks_per_sample()),
        ].join("\n")
    }
}

impl Orientation {
    pub fn base_matrix(&self) -> (i16,i16,i16) {
        match self {
            Orientation::CivmStandard => (-900, 0, 0),
            Orientation::Simulation => (0, 0, 0),
            Orientation::Ortho1 => (0, -900, 0),
            Orientation::Ortho2 => (0, 0, -900),
            Orientation::Scout0 => (0, 0, 900),
            Orientation::Scout1 => (-900, 0, 0),
            Orientation::Scout2 => (-900, -900, 0),
        }
    }
    pub fn print(&self) -> String {
        let mat = self.base_matrix();
        vec![
            format!("MR3040_SelectMatrix( {} );",CALC_MATRIX),
            ppl_function::base_matrix(mat),
            ppl_function::delay_us(100)
        ].join("\n")
    }
}


impl PhaseUnit{
    pub fn value(&self) -> i16 {
        match self {
            PhaseUnit::PU90 => 400,
            PhaseUnit::Min => 1,
        }
    }
    pub fn print(&self) -> String {
        format!("phase_increment({});",self.value())
    }
}





pub enum Import {
    Use(FrameType,String,String),
    Include(String),
    Function(String)
}

impl Import {
    pub fn print(&self) -> String {
        match self {
            Import::Use(kind,path,label) => {
                match kind {
                    FrameType::Rf | FrameType::RfPhase =>
                        format!("#use RF1 \"{}\" {}",path,label),
                    FrameType::Grad =>
                        format!("#use GRAD \"{}\" {}",path,label)
                }
            }
            Import::Include(path) => format!("#include \"{}\"",path),
            Import::Function(func_declaration) => func_declaration.clone()
        }
    }
}

//void systemout(int);
// void delay32(long);

pub struct Includes {
    civm_grad:Import,
    civm_rf:Import,
    civm_include:Import,
    std_fn:Import,
    grad_fn:Import,
    rf_fn:Import,
    std_grad:Import,
    std_rf:Import,
    sys_out:Import,
    delay32:Import,
}

impl Includes {
    pub fn new_default(grad_seqfile:String,rf_seqfile:String) -> Self {
        use FrameType::*;
        Self {
            civm_grad:Import::Use(Grad,grad_seqfile,String::from(GRAD_SEQ_FILE_LABEL)),
            civm_rf:Import::Use(Rf,rf_seqfile,String::from(RF_SEQ_FILE_LABEL)),
            civm_include:Import::Include(String::from(CIVM_INCLUDE)),
            std_fn:Import::Include(String::from(STD_FN_INCLUDE)),
            grad_fn:Import::Include(String::from(GRAD_FN_INCLUDE)),
            rf_fn:Import::Include(String::from(RF_FN_INCLUDE)),
            std_grad:Import::Use(Grad,String::from(STD_GRAD_SEQ),String::from("grad")),
            std_rf:Import::Use(Rf,String::from(STD_RF_SEQ),String::from("pf1")),
            sys_out:Import::Function(String::from("void systemout(int);")),
            delay32:Import::Function(String::from("void delay32(long);")),
        }
    }
    pub fn print(&self,simulator_mode:bool) -> String {
        let sim_define = match simulator_mode {
            true => String::from("#define SIMULATOR on"),
            false => String::from("")
        };
        vec![
            self.civm_grad.print(),
            self.civm_rf.print(),
            sim_define,
            self.std_fn.print(),
            self.civm_include.print(),
            self.rf_fn.print(),
            self.grad_fn.print(),
            self.std_rf.print(),
            self.std_grad.print(),
            self.sys_out.print(),
            self.delay32.print(),
        ].join("\n")
    }
}

pub struct Constants {
    block:Vec<String>,
    required:Vec<String>,
}

impl Constants {
    pub fn default(event_queue:&EventQueue) -> Self {

        let bc = match event_queue.ppl_constants() {
            Some(code) => code,
            None => Vec::<CommandString>::new()
        };

        Self {
            block:bc.iter().map(|cmd| cmd.commands.clone()).collect(),
            required:vec![
                format!("const {} 1;",CALC_MATRIX),
            ]
        }
    }
    pub fn print(&self) -> String {
        let mut strvec = Vec::<String>::new();
        strvec.extend(self.required.clone());
        strvec.extend(self.block.clone());
        strvec.join("\n")
    }
}

pub struct Declarations {
    block:Vec<String>,
    temp_vars:Vec<String>
}

impl Declarations {
    pub fn default(event_queue:&EventQueue) -> Self {
        Self {
            block:event_queue.ppl_declarations().iter().map(|cmd| cmd.commands.clone()).collect(),
            temp_vars:vec![
                format!("int {};",LUT_TEMPVAL_VAR_NAME_1),
                format!("int {};",LUT_TEMPVAL_VAR_NAME_2),
                format!("long {};",LONG_TEMPVAL_VAR_NAME),
                format!("long {};",LUT_INDEX_VAR_NAME),
                String::from("common int pts_mask;"),
                format!("long {};",VIEW_LOOP_COUNTER_VAR),
                format!("int {};",AVERAGES_LOOP_COUNTER_VAR),
                Import::Include(String::from(LUT_INCLUDE)).print(),
                format!("int is16bit;"),
                format!("is16bit = 1;"),
            ]
        }
    }
    pub fn print(&self) -> String {
        let mut strvec = Vec::<String>::new();
        strvec.extend(self.block.clone());
        strvec.extend(self.temp_vars.clone());
        strvec.join("\n")
    }
}

pub struct Initializations {
    block:Vec<String>,
}

impl Initializations {
    pub fn default(event_queue:&EventQueue) -> Self {
        Self {
            block:event_queue.ppl_initializations().iter().map(|cmd| cmd.commands.clone()).collect(),
        }
    }
    pub fn print(&self) -> String {
        let mut strvec = Vec::<String>::new();
        strvec.extend(self.block.clone());
        strvec.join("\n")
    }
}

pub struct PPL {
    pub header:Header,
    pub includes:Includes,
    pub constants:Constants,
    pub declarations:Declarations,
    pub initializations:Initializations,
    pub setup:Setup,
    pub loop_structure:FlatLoopStructure,
    pub simulate:bool
}

impl PPL {
    pub fn new(
        event_queue:&mut EventQueue,
        repetitions:u32,
        averages:u16,
        rep_time:f32,base_freq:BaseFrequency,
        grad_seq_file:&str,
        rf_seq_file:&str,
        orientation:Orientation,
        grad_clock:GradClock,
        phase_unit:PhaseUnit,
        acceleration:u16,
        simulate:bool
    ) -> Result<Self,EventQueueError> {
        // the simulator cannot handle orientations that are not (0,0,0) for the base matrix
        let orientation = match simulate {
            true => Orientation::Simulation,
            false => orientation
        };
        let acq = event_queue.ppl_acquisition()?;
        Ok(Self {
            header:Header {
            dsp_routine:DspRoutine::Dsp,
            receiver_mask:1,
            base_frequency:base_freq,
            samples:acq.n_samples,
            spectral_width: acq.sample_rate,
            sample_discards:acq.n_discards,
            repetitions,
            echos:acq.n_echos,
            echo_divisor:1,
            averages,
            user_adjustments:event_queue.ppl_user_adjustments()
            },
            includes:Includes::new_default(String::from(grad_seq_file),String::from(rf_seq_file)),
            constants:Constants::default(&event_queue),
            declarations:Declarations::default(&event_queue),
            initializations:Initializations::default(&event_queue),
            setup:Setup{grad_clock,orientation,phase_unit},
            loop_structure:event_queue.flat_loop_structure(repetitions,averages,rep_time,acceleration).unwrap(),
            simulate
        })
    }
    pub fn print(&self) -> String {
        vec![
            self.header.print(),
            self.includes.print(self.simulate),
            String::from("main(){"),
            self.constants.print(),
            self.declarations.print(),
            self.setup.print(&self.header),
            self.initializations.print(),
            String::from("sync();"),
            self.loop_structure.print(),
            String::from("end:\n}"),
        ].join("\n")
    }
    pub fn print_ppr(&self,path_to_ppl:&Path) -> String {
        self.header.print_ppr(path_to_ppl)
    }
}

pub struct Setup {
    orientation:Orientation,
    grad_clock:GradClock,
    phase_unit:PhaseUnit,
}

impl Setup {
    pub fn print(&self,header:&Header) -> String {
        vec![
            self.orientation.print(),
            self.grad_clock.print(),
            ppl_function::set_discard_samples(NO_DISCARD_VAR),
            header.base_frequency.set_freq_buffer(),
            self.phase_unit.print()
        ].join("\n")
    }
}


struct PPLNumeric {
    keyword:String,
    var:String,
    min:u32,
    max:u32,
    divisor:Option<u32>,
    value:u32
}

impl PPLNumeric {
    fn new(keyword:&str,var:&str,min:u32,max:u32,divisor:Option<u32>,value:u32) -> Self {
        Self{
            keyword:keyword.to_owned(),
            var:var.to_owned(),
            min,
            max,
            divisor,
            value,
        }
    }
    fn print(&self) -> String{
        match self.divisor{
            Some(div) =>
                format!("{} {},{},{},{},{};",self.keyword,self.min,self.max,self.value,div,self.var),
            None =>
                format!("{} {},{},{},{};",self.keyword,self.min,self.max,self.value,self.var)
        }
    }
    pub fn print_ppr(&self) -> String {
        format!(":{} {}, {}",self.keyword,self.var,self.value)
    }
}

impl Header {
    pub fn print(&self) -> String {
        let mut out = vec![
            String::from("/* PARAMLIST"),
            self.dsp_routine.print(),
            PPLNumeric::new(
                "RECEIVER_MASK",
                RECEIVER_MASK_VAR,
                RECEIVER_MASK_MIN,
                RECEIVER_MASK_MAX,
                None,
                self.receiver_mask as u32
            ).print(),
            format!("GRADIENT_STRENGTH {};",GRAD_STRENGTH_VAR),
            self.base_frequency.print(),
            PPLNumeric::new(
                "SPECTRAL_WIDTH",
                SPECTRAL_WIDTH_VAR,
                self.spectral_width.hertz() as u32,
                self.spectral_width.hertz() as u32,
                None,
                self.spectral_width.hertz() as u32
            ).print(),
            PPLNumeric::new(
                "NO_VIEWS",
                NO_VIEWS_VAR,
                NO_VIEWS_MIN,
                NO_VIEWS_MAX,
                None,
                self.repetitions
            ).print(),
            PPLNumeric::new(
                "NO_ECHOES",
                NO_ECHOES_VAR,
                NO_ECHOES_MIN,
                NO_ECHOES_MAX,
                Some(self.echo_divisor as u32),
                self.echos as u32
            ).print(),
            PPLNumeric::new(
                "NO_AVERAGES",
                NO_AVERAGES_VAR,
                NO_AVERAGES_MIN,
                NO_AVERAGES_MAX,
                None,
                self.averages as u32
            ).print(),
            PPLNumeric::new(
                "NO_SAMPLES",
                NO_SAMPLES_VAR,
                NO_SAMPLES_MIN,
                NO_SAMPLES_MAX,
                None,
                self.samples as u32
            ).print(),
            PPLNumeric::new(
                "DISCARD",
                NO_DISCARD_VAR,
                NO_DISCARD_MIN,
                NO_DISCARD_MAX,
                None,
                self.sample_discards as u32
            ).print(),
        ];
        match &self.user_adjustments {
            Some(list) =>{
                let strvec:Vec<String> = list.iter().map(|item| item.print()).collect();
                out.extend(strvec);
            }
            None => {}
        }
        out.push(String::from(format!("END\n*/")));
        out.join("\n")
    }
    pub fn print_ppr(&self,path_to_ppl:&Path) -> String {
        let mut out = vec![
            format!(":PPL {}",path_to_ppl.to_str().unwrap().to_owned()),
            self.dsp_routine.print_ppr(),
            PPLNumeric::new(
                "RECEIVER_MASK",
                RECEIVER_MASK_VAR,
                RECEIVER_MASK_MIN,
                RECEIVER_MASK_MAX,
                None,
                self.receiver_mask as u32
            ).print_ppr(),
            format!(":GRADIENT_STRENGTH {}, 4, {}, {}, {}, {}",
                    GRAD_STRENGTH_VAR,grad_cal::GRAD_MIN,grad_cal::GRAD_MAX_READ,
                    grad_cal::GRAD_MAX_PHASE,grad_cal::GRAD_MAX_SLICE),
            self.base_frequency.print_ppr(),
            format!(":SAMPLE_PERIOD {}, {}",SPECTRAL_WIDTH_VAR,self.spectral_width.ppr_string()),
            PPLNumeric::new(
                "NO_VIEWS",
                NO_VIEWS_VAR,
                NO_VIEWS_MIN,
                NO_VIEWS_MAX,
                None,
                self.repetitions
            ).print_ppr(),
            PPLNumeric::new(
                "NO_ECHOES",
                NO_ECHOES_VAR,
                NO_ECHOES_MIN,
                NO_ECHOES_MAX,
                Some(self.echo_divisor as u32),
                self.echos as u32
            ).print_ppr(),
            PPLNumeric::new(
                "NO_AVERAGES",
                NO_AVERAGES_VAR,
                NO_AVERAGES_MIN,
                NO_AVERAGES_MAX,
                None,
                self.averages as u32
            ).print_ppr(),
            PPLNumeric::new(
                "NO_SAMPLES",
                NO_SAMPLES_VAR,
                NO_SAMPLES_MIN,
                NO_SAMPLES_MAX,
                None,
                self.samples as u32
            ).print_ppr(),
            PPLNumeric::new(
                "DISCARD",
                NO_DISCARD_VAR,
                NO_DISCARD_MIN,
                NO_DISCARD_MAX,
                None,
                self.sample_discards as u32
            ).print_ppr(),
        ];
        match &self.user_adjustments {
            Some(list) =>{
                let strvec:Vec<String> = list.iter().map(|item| item.print_ppr()).collect();
                out.extend(strvec);
            }
            None => {}
        }
        let mut o = out.join("\n");
        o.push('\n');
        o
    }
}

