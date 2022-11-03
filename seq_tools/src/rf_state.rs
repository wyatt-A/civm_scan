use crate::gradient_matrix::{LUT_INDEX_VAR_NAME, LUT_TEMPVAL_VAR_NAME_1, LUT_TEMPVAL_VAR_NAME_2, LONG_TEMPVAL_VAR_NAME, DriverVar};
use crate::command_string::{CommandString,Command};
use crate::ppl::{ScrollBar, VIEW_LOOP_COUNTER_VAR};

#[derive(Clone,Debug)]
pub struct RfState {
    label:String,// power and phase vars are derived from label
    power:Option<RfStateType>,
    phase:RfStateType
}

#[derive(Clone,Debug)]
pub struct RfDriver{
    kind:RfDriverType,
    driver_var:String,
    // this is added to the driver var to account for echos with different phase encodes
    // starts at 0
    echo_index:usize
}

#[derive(Clone,Debug)]
pub enum RfDriverType{
    PhaseCycle3D(PhaseCycleStrategy),
    PhaseCycle2D(PhaseCycleStrategy),
    PowerRamp(f32)
}

#[derive(Clone,Debug)]
pub enum PhaseCycleStrategy{
    LUTNinetyTwoSeventy(usize,Option<usize>),
    FullySampledNinetyTwoSeventy(usize,Option<usize>),
    CycleCPMG(usize)
}

#[derive(Clone,Debug)]
pub enum RfStateType {
    Static(i16),
    Adjustable(i16,Option<PhaseCycleStrategy>),
    Driven(RfDriver)
}

impl RfDriver {
    pub fn new(driver_variable:DriverVar,driver_type:RfDriverType,echo_index:Option<usize>) -> RfDriver {
        RfDriver{
            kind:driver_type,
            driver_var:driver_variable.varname(),
            echo_index:echo_index.unwrap_or(0)
        }
    }
    pub fn render(&self,state:RfState) -> String {
        match &self.kind {
            RfDriverType::PowerRamp(scale) => {
                panic!("not yet implemented")
            }
            _=> "not yet implemented".to_owned()
        }
    }
}

impl RfState {
    pub fn new(label:&str,power_type:RfStateType,phase_type:RfStateType) -> RfState{
        RfState{
            label:label.to_owned(),
            power:Some(power_type),
            phase:phase_type,
        }
    }
    pub fn new_phase_only(label:&str,phase_type:RfStateType) -> RfState {
        RfState{
            label:label.to_owned(),
            power:None,
            phase:phase_type,
        }
    }
    pub fn header_declaration(&self) -> Option<Vec<ScrollBar>> {
        let mut scrollbars = Vec::<ScrollBar>::new();
        match &self.power {
            Some(RfStateType::Adjustable(init_dac,_)) => {
                let scrollbar = ScrollBar::new_rf_pow_adj(&self.label,&self.adjust_power_var(),*init_dac);
                scrollbars.push(scrollbar);
            }
            _=> {}
        };
        match &self.phase {
            RfStateType::Adjustable(init_dac,_) => {
                let scrollbar = ScrollBar::new_rf_phase_adj(&self.label,&self.adjust_phase_var(),*init_dac);
                scrollbars.push(scrollbar);
            }
            _=> {}
        };
        return if scrollbars.len() > 0 { Some(scrollbars) } else { None };
    }
    pub fn declare_phase_var(&self) -> String {
        match &self.phase {
            RfStateType::Static(_) | RfStateType::Driven(_) => {
                format!("int {};",self.phase_var())
            }
            RfStateType::Adjustable(_,_) => {
                vec![
                    format!("common int {};",self.adjust_phase_var()),
                    format!("int {};",self.phase_var())
                ].join("\n")
            }
        }
    }
    pub fn declare_power_var(&self) -> Option<String> {
        match &self.power {
            Some(RfStateType::Static(_)) | Some(RfStateType::Driven(_)) => {
                Some(format!("int {};",self.power_var()))
            }
            Some(RfStateType::Adjustable(_,_)) => {
                Some(vec![
                    format!("common int {};",self.adjust_power_var()),
                    format!("int {};",self.power_var()),
                ].join("\n"))
            },
            None => None
        }
    }
    pub fn init_power_var(&self) -> Option<String> {
        if self.power.is_none(){
            return None;
        }
        return match &self.power {
            Some(RfStateType::Static(dac_val)) => Some(format!("{} = {};", self.power_var(), dac_val)),
            _ => Some(format!("{} = {};", self.power_var(), 0))
        }
    }
    pub fn init_phase_var(&self) -> String {
        match &self.phase {
            RfStateType::Static(dac_val) => {
                format!("{} = {};",self.phase_var(),dac_val)
            }
            _=> format!("{} = {};",self.phase_var(),0)
        }
    }
    pub fn phase_var(&self) -> String {
        format!("{}_phase",self.label)
    }
    pub fn power_var(&self) -> String {
        format!("{}_power",self.label)
    }
    pub fn set_phase(&self) -> String {
        match &self.phase {
            RfStateType::Static(phase) => {
                format!("{} = {};",self.phase_var(),phase)
            }
            RfStateType::Adjustable(_,strategy) => {
                match strategy {
                    Some(strat) => {
                        match strat {
                            PhaseCycleStrategy::CycleCPMG(acceleration) => {
                                format!("{} = {} + 800*(({}/{})%2);", self.phase_var(),self.adjust_phase_var(), VIEW_LOOP_COUNTER_VAR, acceleration)
                            }
                            _=> panic!("phase cycle strategy no yet implemented for user adjustments")
                        }
                    }
                    None => format!("{} = {};", self.phase_var(), self.adjust_phase_var())
                }
            }
            RfStateType::Driven(driver) => {
                match &driver.kind {
                    RfDriverType::PhaseCycle3D(strategy) => {
                        match &strategy{
                            PhaseCycleStrategy::LUTNinetyTwoSeventy(size_1,size_2) => {
                                let size_2 = size_2.unwrap_or(*size_1);
                                let out_str = vec![
                                    format!("{} = 2L*({}+{});",LUT_INDEX_VAR_NAME,&driver.driver_var,driver.echo_index),
                                    format!("GETLUTENTRY({},{})",LUT_INDEX_VAR_NAME,LUT_TEMPVAL_VAR_NAME_1),
                                    format!("{} = {} + 1L;",LUT_INDEX_VAR_NAME,LUT_INDEX_VAR_NAME),
                                    format!("GETLUTENTRY({},{})",LUT_INDEX_VAR_NAME,LUT_TEMPVAL_VAR_NAME_2),
                                    format!("{} = 2*(({}+{}+{})%2)+1;",self.phase_var(),LUT_TEMPVAL_VAR_NAME_1,LUT_TEMPVAL_VAR_NAME_2,(size_1/2)+(size_2/2)+2)
                                ];
                                out_str.join("\n")
                            }
                            PhaseCycleStrategy::FullySampledNinetyTwoSeventy(size_1,size_2) => {
                                let size_2 = size_2.unwrap_or(*size_1);
                                let out_str = vec![
                                    format!("{}=(({}+{})%{}) - {};",LUT_TEMPVAL_VAR_NAME_1,&driver.driver_var,driver.echo_index,size_1,size_1/2),
                                    format!("{}=(({}+{})/{}) - {};",LUT_TEMPVAL_VAR_NAME_2,&driver.driver_var,driver.echo_index,size_2,size_2/2),
                                    format!("{} = 2*(({}+{}+{})%2)+1;",self.phase_var(),LUT_TEMPVAL_VAR_NAME_1,LUT_TEMPVAL_VAR_NAME_2,(size_1/2)+(size_2/2)+2),
                                ];
                                out_str.join("\n")
                            }
                            PhaseCycleStrategy::CycleCPMG(acceleration) => {
                                format!("{} = 400*(2*(({}/{}+{})%2)+1);",self.phase_var(),&driver.driver_var,acceleration,driver.echo_index)
                            }
                            _=> "phase cycle strategy not implemented yet".to_owned()
                        }
                    }
                    _=> "driver not implemented yet".to_owned()
                }
            }
        }
    }
    pub fn set_power(&self) -> Option<String> {
        match &self.power {
            Some(RfStateType::Static(dac)) => {
                Some(format!("{} = {};",self.power_var(),dac))
            }
            Some(RfStateType::Adjustable(_,_)) => {
                Some(format!("{} = {};",self.power_var(),self.adjust_power_var()))
            }
            Some(RfStateType::Driven(driver)) => {
                match &driver.kind {
                    RfDriverType::PowerRamp(scale) => {
                        Some(format!("{} = {}*{}",self.power_var(),driver.driver_var,scale))
                    }
                    _=> Some("".to_owned())
                }
            }
            None => None
        }
    }
    pub fn adjust_power_var(&self) -> String {
        format!("{}_adj",self.power_var())
    }
    pub fn adjust_phase_var(&self) -> String {
        format!("{}_adj",self.phase_var())
    }
    pub fn power(&self) -> RfStateType {
        self.power.clone().expect("rf event must have a power field. What happened??")
    }
}