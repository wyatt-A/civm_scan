use std::path::{Path};
use serde::{Deserialize, Serialize};
use crate::command_string::CommandString;
use crate::acq_event::SpectralWidth;
use crate::event_block::{EventQueue, EventQueueError};
use crate::ppl_function;
use crate::_utils;
use crate::rf_frame::RF_MAX_DAC;
use crate::seqframe::{FrameType, RF_SEQ_FILE_LABEL, GRAD_SEQ_FILE_LABEL};
use crate::gradient_matrix::{LUT_TEMPVAL_VAR_NAME_1, LUT_TEMPVAL_VAR_NAME_2, LONG_TEMPVAL_VAR_NAME, LUT_INDEX_VAR_NAME};
use crate::grad_cal;

const CIVM_INCLUDE:&str = r"C:\workstation\SequenceTools\CivmSequenceTools_v1.0\civm_var_20_long.PPH";
const STD_FN_INCLUDE:&str = r"stdfn_15.pph";
const GRAD_FN_INCLUDE:&str = r"m3040_20.pph";
const RF_FN_INCLUDE:&str = r"m3031_15.pph";
const STD_RF_SEQ:&str = r"c:\smis\seqlib\RFstd.seq";
const STD_GRAD_SEQ:&str = r"c:\smis\seqlib\g3040_15.seq";
const LUT_INCLUDE:&str = r"C:\smis\include\lututils.pph";

const CALC_MATRIX:&str = "c_calc_mat";

const SPECTRAL_WIDTH_VAR:&str = "sample_period";
const GRAD_STRENGTH_VAR:&str = "grad_var";
const RECEIVER_MASK_VAR:&str = "rec_sel";
const RECEIVER_MASK_MIN:u32 = 1;
const RECEIVER_MASK_MAX:u32 = 65535;

const NO_SAMPLES_VAR:&str = "no_samples";
const NO_SAMPLES_MIN:u32 = 8;
const NO_SAMPLES_MAX:u32 = 65535;

const NO_DISCARD_VAR:&str = "no_discard";
const NO_DISCARD_MIN:u32 = 0;
const NO_DISCARD_MAX:u32 = 64;

const NO_ECHOES_VAR:&str = "no_echoes";
const NO_ECHOES_MIN:u32 = 1;
const NO_ECHOES_MAX:u32 = 64;

const NO_VIEWS_VAR:&str = "no_views";
const VIEW_LOOP_NAME:&str = "views_loop";
pub const VIEW_LOOP_COUNTER_VAR:&str = "no_completed_views";

const NO_VIEWS_MIN:u32 = 1;
const NO_VIEWS_MAX:u32 = 500_000;

const NO_AVERAGES_VAR:&str = "no_averages";
const AVERAGES_LOOP_NAME:&str = "averages_loop";
pub const AVERAGES_LOOP_COUNTER_VAR:&str = "no_completed_averages";
const NO_AVERAGES_MIN:u32 = 1;
const NO_AVERAGES_MAX:u32 = 65535;

const FREQ_OFFSET_MIN:i32 = -40000000;
const FREQ_OFFSET_MAX:i32 = 40000000;

pub enum DspRoutine {
    Dsp
}

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

impl DspRoutine {
    fn print(&self) -> String {
        match self {
            DspRoutine::Dsp =>
                String::from("DSP_ROUTINE \"dsp\";")
        }
    }
    pub fn print_ppr(&self) -> String {
        match self {
            DspRoutine::Dsp =>
                String::from(":DSP_ROUTINE dsp")
        }
    }
}


// pub enum BaseFrequency {
//     Civm9p4T(f32)
// }

#[derive(Clone,Serialize,Deserialize)]
pub struct BaseFrequency {
    base_freq:f32,
    obs_offset:f32
}


impl BaseFrequency {

    pub fn civm9p4t(offset:f32) -> Self {
        Self {
            base_freq:30171576.0,
            obs_offset:offset
        }
    }
    fn print(&self) -> String {
                format!("OBSERVE_FREQUENCY \"9.4T 1H\",{},{},{},MHz, kHz, Hz, rx1MHz;",
                        FREQ_OFFSET_MIN,FREQ_OFFSET_MAX,self.obs_offset)
    }
    fn print_ppr(&self) -> String {
                format!(":OBSERVE_FREQUENCY \"9.4T 1H\", {:.1}, MHz, kHz, Hz, rx1MHz"
                        ,self.base_freq+self.obs_offset)
    }
    pub fn set_freq_buffer(&self) -> String {
        ppl_function::set_base_freq()
    }
}

pub struct Header {
    pub dsp_routine:DspRoutine,
    pub receiver_mask:u16,
    pub base_frequency:BaseFrequency,
    pub samples:u16,
    pub spectral_width: SpectralWidth,
    pub sample_discards:u16,
    pub repetitions:u32,
    pub echos:u16,
    pub echo_divisor:u16,
    pub averages:u16,
    pub user_adjustments:Option<Vec<Adjustment>>
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
    block_constants:Vec<String>,
    miscellaneous:Vec<String>,
}

impl Constants {
    pub fn default(event_queue:&EventQueue) -> Self {

        let bc = match event_queue.ppl_constants() {
            Some(code) => code,
            None => Vec::<CommandString>::new()
        };

        Self {
            block_constants:bc.iter().map(|cmd| cmd.commands.clone()).collect(),
            miscellaneous:vec![
                format!("const {} 1;",CALC_MATRIX),
            ]
        }
    }
    pub fn print(&self) -> String {
        let mut strvec = Vec::<String>::new();
        strvec.extend(self.miscellaneous.clone());
        strvec.extend(self.block_constants.clone());
        strvec.join("\n")
    }
}

pub struct Declarations {
    block_declarations:Vec<String>,
    temp_vars:Vec<String>
}

impl Declarations {
    pub fn default(event_queue:&EventQueue) -> Self {
        Self {
            block_declarations:event_queue.ppl_declarations().iter().map(|cmd| cmd.commands.clone()).collect(),
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
        strvec.extend(self.block_declarations.clone());
        strvec.extend(self.temp_vars.clone());
        strvec.join("\n")
    }
}

pub struct Initializations {
    block_initializations:Vec<String>,
}

impl Initializations {
    pub fn default(event_queue:&EventQueue) -> Self {
        Self {
            block_initializations:event_queue.ppl_initializations().iter().map(|cmd| cmd.commands.clone()).collect(),
        }
    }
    pub fn print(&self) -> String {
        let mut strvec = Vec::<String>::new();
        strvec.extend(self.block_initializations.clone());
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



pub enum AdjustmentInterface {
    Scrollbar,
    Text,
}

pub struct Adjustment {
    interface:AdjustmentInterface,
    title:String,
    title_hint:String,
    target_var:String,
    min:i16,
    max:i16,
    scale:f32,
    default:i16,
}

impl Adjustment {
    pub fn new_rf_pow_adj(label:&str,target_var:&str,default_val:i16) -> Self {
        Self {
            title:format!("{} dac percent",label),
            title_hint:String::from("%"),
            target_var:String::from(target_var),
            min:0,
            max:RF_MAX_DAC,
            scale:RF_MAX_DAC as f32/100.0,
            default:default_val,
            interface:AdjustmentInterface::Scrollbar
        }
    }
    pub fn new_rf_phase_adj(label:&str,target_var:&str,default_val:i16) -> Self {
        Self {
            title:format!("{} phase adjustment",label),
            title_hint:String::from("400=90deg"),
            target_var:String::from(target_var),
            min:-800,
            max:800,
            scale:1.0,
            default:default_val,
            interface:AdjustmentInterface::Scrollbar
        }
    }
    pub fn new_grad_adj(label:&str,target_var:&str,half_range:i16) -> Self {
        Self {
            title:format!("{}",label),
            title_hint:String::from("dac"),
            target_var:String::from(target_var),
            min:-half_range,
            max:half_range,
            scale:1.0,
            default:0,
            interface:AdjustmentInterface::Text
        }
    }
    fn print(&self) -> String {
        match self.interface {
            AdjustmentInterface::Text => {
                format!("EDITTEXT \"{}\",\"{}\",\"%.2f\",{},{},{},{},{};",
                        self.title,self.title_hint,self.min,self.max,self.default,self.scale,self.target_var,)
            },
            AdjustmentInterface::Scrollbar => {
                format!("SCROLLBAR \"{}\",\"{}\",\"%.2f\",{},{},{},{},{};",
                        self.title,self.title_hint,self.min,self.max,self.default,self.scale,self.target_var,)
            }
        }

    }
    fn print_ppr(&self) -> String {
        format!(":VAR {}, {}",self.target_var,self.default)
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



impl Loop {
    pub fn start(&self) -> String {
        match self {
            Loop::Repetition(_,_) =>
                Loop::_start(VIEW_LOOP_NAME),
            Loop::Average(_) =>
                Loop::_start(AVERAGES_LOOP_NAME)
        }
    }
    pub fn end(&self) -> String {
        match self{
            Loop::Repetition(_,step) =>
                Loop::_end(VIEW_LOOP_NAME,VIEW_LOOP_COUNTER_VAR,NO_VIEWS_VAR,*step),
            Loop::Average(_) =>
                Loop::_end(AVERAGES_LOOP_NAME,AVERAGES_LOOP_COUNTER_VAR,NO_AVERAGES_VAR,1),
        }
    }

    pub fn init_counter(&self) -> String {
        match self {
            Loop::Repetition(_,_) =>
                format!("{} = 0;",VIEW_LOOP_COUNTER_VAR),
            Loop::Average(_) =>
                format!("{} = 0;",AVERAGES_LOOP_COUNTER_VAR)
        }
    }

    fn _start(loop_name:&str) -> String {
        format!("{}:",loop_name)
    }
    fn _end(loop_name:&str,counter_name:&str,varname:&str,step:u16) -> String {
        vec![
            format!("{} = {} + {};",counter_name,counter_name,step),
            format!("if ({} < {}*{})",counter_name,step,varname),
            format!("goto {};",loop_name),
        ].join("\n")
    }

}

pub enum Loop {
    Repetition(u32,u16),
    Average(u16)
}

pub struct CalcBlock {
    body:Vec<CommandString>,
}

impl CalcBlock {
    pub fn new(calc_commands:Vec<CommandString>) -> Self {
        Self {
            body:calc_commands
        }
    }
    fn header(&self) -> Vec<CommandString>{
        vec![
            CommandString::new_calculation(&ppl_function::start_timer()),
            CommandString::new_calculation(&ppl_function::host_request()),
            CommandString::new_calculation(&ppl_function::system_out())
        ]
    }
    pub fn footer(&self) -> Vec<CommandString> {
        vec![CommandString::new_calculation(&ppl_function::wait_timer(self.duration_clocks()))]
    }
    pub fn duration_clocks(&self) -> i32 {
        // todo! estimate required duration from body
        _utils::ms_to_clock(3)
    }
    pub fn print(&self) -> String {
        let h:Vec<String> = self.header().iter().map(|cmds| cmds.commands.clone()).collect();
        let b:Vec<String> = self.body.iter().map(|cmds| cmds.commands.clone()).collect();
        let f:Vec<String> = self.footer().iter().map(|cmds| cmds.commands.clone()).collect();
        vec![
            h.join("\n"),
            b.join("\n"),
            f.join("\n")
        ].join("\n")
    }
}

pub struct FlatLoopStructure {
    outer:Loop,
    inner:Loop,
    calc_block:CalcBlock,
    exec_block:Vec<CommandString>
}

impl FlatLoopStructure {
    pub fn new(repetitions:u32, averages:u16, rep_time:f32, event_queue:&mut EventQueue,acceleration:u16) -> Result<Self,EventQueueError> {
        // lock in events in the event queue so timing is accurate
        let calc_block = CalcBlock::new(event_queue.export_calc_blocks());
        let calc_time = calc_block.duration_clocks();
        let loop_time = FlatLoopStructure::loop_waittimer();
        event_queue.set_rep_time(rep_time,loop_time,calc_time)?;
        Ok(Self {
            outer:Loop::Repetition(repetitions,acceleration),
            inner:Loop::Average(averages),
            calc_block,
            exec_block:event_queue.export_exec_blocks() // rep time must be set before exporting execs
        })
    }
    fn loop_waittimer() -> i32 {
        500
    }

    pub fn print(&self) -> String {

        let exec_string_vec:Vec<String> = self.exec_block.iter().map(|block| block.commands.clone()).collect();
        let exec_string = exec_string_vec.join("\n");

        vec![
            ppl_function::start_timer(),
            self.outer.init_counter(),
            self.outer.start(),
            self.inner.init_counter(),
            self.inner.start(),
            ppl_function::wait_timer(FlatLoopStructure::loop_waittimer()),
            self.calc_block.print(),
            exec_string,
            ppl_function::start_timer(),
            self.inner.end(),
            self.outer.end()
        ].join("\n")
    }
    pub fn n_reps(&self) -> u32 {
        match self.outer {
            Loop::Repetition(n,_) => n,
            _=> panic!("this loop structure must have an outer repetition loop")
        }
    }
    pub fn n_averages(&self) -> u32 {
        match self.outer {
            Loop::Average(n) => n as u32,
            _=> panic!("this loop structure must have an inner averages loop")
        }
    }
}


#[test]
fn test(){
    let h = Header{
        dsp_routine:DspRoutine::Dsp,
        receiver_mask:1,
        base_frequency:BaseFrequency::civm9p4t(0.0),
        samples:788,
        spectral_width: SpectralWidth::SW200kH,
        sample_discards:0,
        repetitions:28000,
        echos:4,
        echo_divisor:1,
        averages:1,
        user_adjustments:None
    };

    println!("{}",h.print())
}



