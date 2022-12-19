use crate::gradient_frame::GradFrame;
use crate::gradient_matrix::{Matrix, MatrixType};
use crate::command_string::CommandString;
use crate::event_block::GradEventType;
use crate::execution::{ExecutionBlock, WaveformData, BlockExecution, EventType};
use crate::ppl_function;
use crate::_utils;
use crate::ppl::Adjustment;
use crate::seqframe::{GRAD_SEQ_FILE_LABEL, SeqFrame};

const TIME_BLOCK_1:i32 = 300;
const TIME_BLOCK_2:i32 = 300;

//const SINGLE_CHANNEL_START_DELAY:i32 = 100; // cost of 1 channel start
const SINGLE_CHANNEL_START_DELAY:i32 = 0; // cost of 1 channel start
const EXTRA_CHANNEL_START_DELAY:i32 = 30; // added cost per 1 channel more (81 + 31 for 2 channels)

const READ_MASK:&str = "0x0002";
const PHASE_MASK:&str = "0x0020";
const SLICE_MASK:&str = "0x0200";

#[derive(Clone,Copy)]
pub enum Channel{
    Read,
    Phase,
    Slice
}

impl Channel {
    pub fn mask(&self) -> &str {
        match &self {
            Channel::Read => READ_MASK,
            Channel::Phase => PHASE_MASK,
            Channel::Slice => SLICE_MASK,
        }
    }
}

#[derive(Clone)]
pub struct GradEvent<GF> where GF:GradFrame{
    read_frame:Option<GF>,
    phase_frame:Option<GF>,
    slice_frame:Option<GF>,
    kind: GradEventType,
    matrix:Matrix,
    label:String,
}

impl<GF> GradEvent<GF> where GF:GradFrame + Copy {
    pub fn new(grad_frames:(Option<GF>,Option<GF>,Option<GF>), matrix:&Matrix, event_type: GradEventType, label:&str) -> GradEvent<GF> {
        GradEvent{
            read_frame:grad_frames.0,
            phase_frame:grad_frames.1,
            slice_frame:grad_frames.2,
            kind:event_type,
            matrix:matrix.clone(),
            label:label.to_string()
        }
    }
    fn list_label(&self,channel:Channel) -> Option<String>{
        match channel {
            Channel::Read => if self.read_frame.is_some() {Some(format!("{}_read",self.label))} else {None},
            Channel::Phase => if self.phase_frame.is_some() {Some(format!("{}_phase",self.label))} else {None},
            Channel::Slice => if self.slice_frame.is_some() {Some(format!("{}_slice",self.label))} else {None},
        }
    }
    fn channel_seq_frame(&self,channel:Channel,sample_period_us:usize) -> Option<SeqFrame>{
        match channel {
            Channel::Read => match self.read_frame {
                Some(frame) => Some(frame.grad_seq_frame(&self.list_label(channel).unwrap(),sample_period_us)),
                None => None
            }
            Channel::Phase => match self.phase_frame {
                Some(frame) => Some(frame.grad_seq_frame(&self.list_label(channel).unwrap(),sample_period_us)),
                None => None
            }
            Channel::Slice => match self.slice_frame {
                Some(frame) => Some(frame.grad_seq_frame(&self.list_label(channel).unwrap(),sample_period_us)),
                None => None
            }
        }
    }
    pub fn seq_params(&self,sample_period_us:usize) -> Option<String> {
        let mut txt = Vec::<String>::new();
        match self.channel_seq_frame(Channel::Read,sample_period_us) {
            None => {},
            Some(seq_frame) => txt.push(seq_frame.serialize())
        }
        match self.channel_seq_frame(Channel::Phase,sample_period_us) {
            None => {},
            Some(seq_frame) => txt.push(seq_frame.serialize())
        }
        match self.channel_seq_frame(Channel::Slice,sample_period_us) {
            None => {},
            Some(seq_frame) => txt.push(seq_frame.serialize())
        }
        return if txt.len() > 0 {Some(txt.join("\n"))} else {None}
    }

    pub fn list(&self,channel:Channel) -> Option<String> {
        match self.list_label(channel) {
            Some(label) => Some(format!("MR3040_SetList({},{});",label,&channel.mask())),
            None => None
        }
    }
    pub fn declare_list(&self,channel:Channel) -> Option<String> {
        match self.list_label(channel){
            Some(label) => Some(format!("int {};",label)),
            None => None
        }
    }
    pub fn init_list(&self,channel:Channel) -> Option<String> {
        match self.list_label(channel){
            Some(label) => {
                Some(vec![
                    ppl_function::init_list_var(&label),
                    ppl_function::init_list(&GRAD_SEQ_FILE_LABEL,&label)
                ].join("\n"))
            }
            None => None
        }
    }
    pub fn select_matrix(&self) -> String {
        format!("MR3040_SelectMatrix({});",self.matrix.label())
    }
    pub fn set_list(&self) -> String {
        let mut out_str = Vec::<String>::new();
        let r = self.list(Channel::Read);
        let p = self.list(Channel::Phase);
        let s = self.list(Channel::Slice);
        match r {
            Some(list) => out_str.push(list),
            None => {}
        }
        match p {
            Some(list) => out_str.push(list),
            None => {}
        }
        match s {
            Some(list) => out_str.push(list),
            None => {}
        }
        out_str.join("\n")
    }
    pub fn channel_mask(&self) -> String {
        let mut s = String::from("0x0");
        if self.slice_frame.is_some(){s.push('2')} else {s.push('0')}
        if self.phase_frame.is_some(){s.push('2')} else {s.push('0')}
        if self.read_frame.is_some(){s.push('2')} else {s.push('0')}
        s
    }
    pub fn n_active_channels(&self) -> i32 {
        let mut count = 0;
        if self.read_frame.is_some(){count = count + 1}
        if self.phase_frame.is_some(){count = count + 1}
        if self.slice_frame.is_some(){count = count + 1}
        count
    }
    pub fn hardware_start_delay(&self) -> i32 {
        (self.n_active_channels()-1)*EXTRA_CHANNEL_START_DELAY + SINGLE_CHANNEL_START_DELAY
    }
    pub fn frame_duration_clocks(&self,chann:Channel) -> Option<i32> {
        match chann {
            Channel::Read => match self.read_frame {
                Some(frame) => Some(_utils::sec_to_clock(frame.duration())),
                None => None
            }
            Channel::Phase => match self.phase_frame {
                Some(frame) => Some(_utils::sec_to_clock(frame.duration())),
                None => None
            }
            Channel::Slice => match self.slice_frame {
                Some(frame) => Some(_utils::sec_to_clock(frame.duration())),
                None => None
            }
        }
    }
    pub fn max_waveform_duration(&self) -> i32 {
        use std::cmp::max;
        let rd = self.frame_duration_clocks(Channel::Read).unwrap_or(0);
        let pd = self.frame_duration_clocks(Channel::Phase).unwrap_or(0);
        let sd = self.frame_duration_clocks(Channel::Slice).unwrap_or(0);
        max(max(rd, pd),sd) // i actually looked up if the max operation is associative haha
    }
    pub fn derive_with_matrix(&self, label:&str, matrix:&Matrix) -> GradEvent<GF> {
        let mut derived = self.clone();
        derived.matrix = matrix.clone();
        derived.label = label.to_string();
        return derived
    }
    pub fn matrix(&self) -> Matrix {
        self.matrix.clone()
    }
}

impl<GF: 'static> ExecutionBlock for GradEvent<GF> where GF:GradFrame + Copy{
    fn block_duration(&self) -> i32 {
        TIME_BLOCK_1 + TIME_BLOCK_2
    }
    fn time_to_start(&self) -> i32 {
        self.hardware_start_delay() + TIME_BLOCK_1
    }
    fn time_to_end(&self) -> i32 {
        self.time_to_start() + self.max_waveform_duration()
    }
    fn time_to_center(&self) -> i32 {
        self.time_to_start() + self.max_waveform_duration()/2
    }
    fn block_execution(&self,post_delay_clocks:i32) -> BlockExecution{
        let cmd = CommandString::new_hardware_exec(
            &vec![
                ppl_function::start_timer(),
                self.select_matrix(),
                self.set_list(),
                ppl_function::wait_timer(TIME_BLOCK_1),
                ppl_function::start_timer(),
                ppl_function::grad_start(&self.channel_mask()),
                ppl_function::wait_timer(TIME_BLOCK_2),
            ].join("\n")
        );
        BlockExecution::new(cmd,post_delay_clocks)
    }
    fn block_header_adjustments(&self) -> Option<Vec<Adjustment>> {
        self.matrix.header_declaration()
    }
    fn block_constant_initialization(&self) -> Option<CommandString> {
        Some(CommandString::new_constant(&self.matrix.declaration()))
    }
    fn block_initialization(&self) -> CommandString {
        let mut cmds = Vec::<String>::new();
        // match self.matrix.kind(){
        //     MatrixType::Static(_) => cmds.push(self.matrix.set_vars()),
        //     _ => {}
        // };
        // match self.matrix.kind(){
        //     MatrixType::Static(_) => cmds.push(self.matrix.create_matrix()),
        //     _ => {}
        // };
        match self.init_list(Channel::Read) {
            Some(cmd) => cmds.push(cmd),
            None => {}
        }
        match self.init_list(Channel::Phase) {
            Some(cmd) => cmds.push(cmd),
            None => {}
        }
        match self.init_list(Channel::Slice) {
            Some(cmd) => cmds.push(cmd),
            None => {}
        }
        CommandString::new_init(&cmds.join("\n"))
    }
    fn block_declaration(&self) -> CommandString {
        let mut cmds = Vec::<String>::new();
        match self.declare_list(Channel::Read) {
            Some(cmd) => cmds.push(cmd),
            None => {}
        }
        match self.declare_list(Channel::Phase) {
            Some(cmd) => cmds.push(cmd),
            None => {}
        }
        match self.declare_list(Channel::Slice) {
            Some(cmd) => cmds.push(cmd),
            None => {}
        }
        cmds.push(self.matrix.vars_declaration());
        let adj = self.matrix.vars_adj_declaration();
        if adj.is_some() {
            cmds.push(adj.unwrap());
        }
        CommandString::new_declare(&cmds.join("\n"))
    }
    fn block_calculation(&self) -> Option<CommandString> {
        // only applies to matrices of type driven and derived
        let set_matrix_vars = match self.matrix.kind(){
            MatrixType::Driven(_,_,_) | MatrixType::Derived(_,_) | MatrixType::Static(_) => Some(self.matrix.set_vars()),
        };
        let create_matrix = match self.matrix.kind(){
            MatrixType::Driven(_,_,_) | MatrixType::Derived(_,_) | MatrixType::Static(_) => Some(self.matrix.create_matrix()),
        };
        let mut cmds = Vec::<String>::new();

        match set_matrix_vars {
            Some(code) => {
                cmds.push(code)
            }
            None => {}
        }
        match create_matrix {
            Some(code) => {
                cmds.push(code)
            }
            None => {}
        }
        return if cmds.len() == 0 {
            None
        } else {
            Some(CommandString::new_calculation(&cmds.join("\n")))
        }
    }
    fn as_reference(&self) -> Box<dyn ExecutionBlock> {
        Box::new(self.clone())
    }
    fn label(&self) -> String {
        self.label.clone()
    }
    fn render_normalized(&self, time_step_us:usize) -> WaveformData {
        let r = match self.read_frame {
            Some(frame) =>{
                Some(frame.render_normalized(time_step_us))
            }
            None => None
        };
        let p = match self.phase_frame {
            Some(frame) =>{
                Some(frame.render_normalized(time_step_us))
            }
            None => None
        };
        let s = match self.slice_frame {
            Some(frame) =>{
                Some(frame.render_normalized(time_step_us))
            }
            None => None
        };
        WaveformData::Grad(r,p,s)
    }
    fn kind(&self) -> EventType {
        EventType::Grad
    }
    fn blocking(&self) -> bool {
        match self.kind {
            GradEventType::Blocking => true,
            GradEventType::NonBlocking => false
        }
    }
    fn seq_params(&self, sample_period_us: usize) -> Option<String> {
        self.seq_params(sample_period_us)
    }
    fn render_magnitude(&self,time_step_us:usize,driver_value:u32) -> WaveformData {
        let dac = self.matrix.dac_vals(driver_value);
        let r = match self.read_frame {
            Some(frame) =>{
                Some(frame.render_magnitude(time_step_us,dac.read.unwrap_or(0)))
            }
            None => None
        };
        let p = match self.phase_frame {
            Some(frame) =>{
                Some(frame.render_magnitude(time_step_us,dac.phase.unwrap_or(0)))
            }
            None => None
        };
        let s = match self.slice_frame {
            Some(frame) =>{
                Some(frame.render_magnitude(time_step_us,dac.slice.unwrap_or(0)))
            }
            None => None
        };
        WaveformData::Grad(r,p,s)
    }
}

/*
starttimer();
MR3040_SelectMatrix( c_spoil_mat );
MR3040_SetList( c_spoil_read_list, 0x0002);
MR3040_SetList( c_spoil_phase_list, 0x0020);
MR3040_SetList( c_spoil_slice_list, 0x0200);
waittimer(300);
MR3040_Start(0x0222);
waittimer(600);
*/


// #[test]
// fn test(){
//
//     let m_tracker = Rc::new(RefCell::<u8>::new(1));
//
//     let t = Trapezoid::new(100E-6,1E-3);
//     let t2 = Trapezoid::new(100E-6,2E-3);
//
//     let pe_strategy = EncodeStrategy::LUT;
//     let driver = MatrixDriver::new("no_completed_views",MatrixDriverType::PhaseEncode3D(pe_strategy));
//
//     let transform = LinTransform::new((Some(5.0),Some(5.0),Some(5.0)),(None,None,None));
//     let d = DacValues::new(None,None,None);
//
//     let m = Matrix::new_static("mat1",DacValues::new(Some(300),Some(300),None),&m_tracker);
//     let m1 = Matrix::new_driven("driven",driver,transform,d,&m_tracker);
//
//     let ge = GradEvent::new((Some(t),Some(t),None), &m1, GradEventType::Blocking, "phase_encode");
//
//     //println!("{}",t2.grad_seq_frame("phase_encode_read",2).serialize());
//     //println!("{}",t.grad_seq_frame("phase_encode_read",2).serialize());
//
//     let dec = ge.block_declaration();
//     let consts = ge.block_constant_initialization().unwrap();
//     let init = ge.block_initialization();
//     let calc = ge.block_calculation();
//     let exec = ge.block_execution(64).cmd_string();
//
//     println!("{}",consts.commands);
//     println!("{}",dec.commands);
//     println!("{}",init.commands);
//     println!("{}",calc.unwrap().commands);
//     println!("{}",exec.commands);
// }