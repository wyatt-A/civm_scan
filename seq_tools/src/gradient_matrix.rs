/*
    A Gradient Matrix is list of magnitudes that determine the peak strength of a gradient waveform
    A gradient matrix comes in 3 flavors:
        Static: it gets a single state that doesn't change over the pulse program
        Driven: it's state changes every time a set of variables is updated
        Derived: it's state is driven by another matrix down to some linear scaling

        Note* calling these things matrices is a bit of a misnomer because they only hold 3 values
        and do not make use of matrix operations. That is handled internally in the pulse program
 */

use std::cell::RefCell;
use std::rc::Rc;
use crate::ppl_constants::{AVERAGES_LOOP_COUNTER_VAR, LONG_TEMPVAL_VAR_NAME, LUT_INDEX_VAR_NAME, LUT_TEMPVAL_VAR_NAME_1, LUT_TEMPVAL_VAR_NAME_2, VIEW_LOOP_COUNTER_VAR};
use crate::ppl_header::Adjustment;


#[derive(Clone,Copy,Debug)]
struct TransCoeffs {
    scale:f32,
    offset:i16
}

impl TransCoeffs {
    fn transform(&self,dac:Option<i16>) -> Option<i16> {
        if dac.is_none(){return None}
        let d = (dac.unwrap() as f32)*self.scale + self.offset as f32;
        if d > (i16::MAX as f32) || d < (i16::MIN as f32) {panic!("gradient dac overflow!");}
        Some(d as i16)
    }
    pub fn transform_string(&self,parent_var:&str,target_var:&str,temp_long_var:&str) -> String {
        let d = 100.0;
        let n = (self.scale*d) as i32;
        format!("{}={};\n{}=({}*{}L)/{}L + {}L;\n{}={};",
                temp_long_var,parent_var,
                temp_long_var,temp_long_var,n,d as i32,self.offset,
                target_var,temp_long_var
        )
    }
}

#[derive(Clone,Copy,Debug)]
pub struct LinTransform {
    read:TransCoeffs,
    phase:TransCoeffs,
    slice:TransCoeffs,
}

impl LinTransform {
    pub fn new(
        scale:(Option<f32>,Option<f32>,Option<f32>),
        offset:(Option<i16>,Option<i16>,Option<i16>))-> LinTransform{
            LinTransform{
                read:TransCoeffs{
                    scale:scale.0.unwrap_or(1.0),
                    offset:offset.0.unwrap_or(0)
                },
                phase:TransCoeffs{
                    scale:scale.1.unwrap_or(1.0),
                    offset:offset.1.unwrap_or(0)
                },
                slice:TransCoeffs{
                    scale:scale.2.unwrap_or(1.0),
                    offset:offset.2.unwrap_or(0)
                },
            }
    }
}

#[derive(Clone,Copy,Debug)]
pub struct DacValues {
    pub read:Option<i16>,
    pub phase:Option<i16>,
    pub slice:Option<i16>,
}

impl DacValues {
    pub fn new(read:Option<i16>,phase:Option<i16>,slice:Option<i16>) -> DacValues{
        DacValues{
            read,phase,slice
        }
    }
    pub fn transform(&self,trans:LinTransform) -> DacValues {
        DacValues {
            read:trans.read.transform(self.read),
            phase:trans.phase.transform(self.phase),
            slice:trans.slice.transform(self.slice),
        }
    }
}

#[derive(Clone,Debug)]
pub enum MatrixType {
    Static(DacValues),
    Driven(MatrixDriver,LinTransform,DacValues),
    Derived(Rc<Matrix>,LinTransform),
}

#[derive(Clone,Debug)]
pub enum EncodeStrategy {
    FullySampled(Dimension,usize,Option<usize>), // derive k-space coordinates
    LUT(Dimension,Vec<i16>),
}

#[derive(Clone,Debug)]
pub enum Dimension{
    _3D,
    _2D
}

impl EncodeStrategy {
    pub fn dac_value(&self,driver_val:u32,trans:LinTransform,default_dac:DacValues,echo_index:usize) -> DacValues {
        match self {
            EncodeStrategy::FullySampled(dim,size1,size2) => {
                match dim {
                    Dimension::_3D => { // 3-d case
                        let size2 = size2.unwrap_or(*size1);
                        let coord1 = (driver_val%(*size1 as u32))as i32 - *size1 as i32/2;
                        let dac_phase = trans.phase.transform(Some(coord1 as i16));
                        let coord2 = (driver_val/(size2 as u32))as i32 - size2 as i32/2;
                        let dac_slice = trans.slice.transform(Some(coord2 as i16));
                        let dac_read = trans.read.transform(default_dac.read);
                        println!("dac_phase = {}",dac_phase.unwrap());
                        println!("dac_slice = {}",dac_slice.unwrap());
                        DacValues::new(dac_read,dac_phase,dac_slice)
                    }
                    Dimension::_2D => { // 2-d case
                        let coord = driver_val as i32 - (*size1 as i32/2);
                        let dac_phase = trans.phase.transform(Some(coord as i16));
                        let dac_read = trans.read.transform(default_dac.read);
                        let dac_slice = trans.slice.transform(default_dac.slice);
                        DacValues::new(dac_read,dac_phase,dac_slice)
                    }
                }
            }
            EncodeStrategy::LUT(dim,lut) => {
                match dim {
                    Dimension::_2D => {
                        let lut_idx = driver_val as usize + echo_index;
                        let lut_tempval = lut[lut_idx];
                        let dac_phase = trans.phase.transform(Some(lut_tempval));
                        let dac_read = trans.read.transform(default_dac.read);
                        let dac_slice = trans.slice.transform(default_dac.slice);
                        DacValues::new(dac_read,dac_phase,dac_slice)
                    }
                    Dimension::_3D => {
                        let lut_idx1 = 2*(driver_val as usize + echo_index);
                        let lut_idx2 = lut_idx1 + 1;
                        let lut_tempval1 = lut[lut_idx1];
                        let lut_tempval2 = lut[lut_idx2];
                        let dac_phase = trans.phase.transform(Some(lut_tempval1));
                        let dac_slice = trans.slice.transform(Some(lut_tempval2));
                        let dac_read = trans.read.transform(default_dac.read);
                        DacValues::new(dac_read,dac_phase,dac_slice)
                    }
                }
            }
        }
    }
    pub fn print(&self,driver_var:&str,matrix:&Matrix,trans:LinTransform,default_dac:DacValues,echo_index:usize) -> String {
        match self {
            EncodeStrategy::FullySampled(dim,size1,size2) => {
                match dim {
                    Dimension::_3D => { // 3-d case
                        let size2 = size2.unwrap_or(*size1);
                        let vars = matrix.var_names();
                        let out_str = vec![
                            format!("{}",format!("{}=({}%{}) - {};",vars.1,driver_var,size1,size1/2)),
                            trans.phase.transform_string(&vars.1,&vars.1,LONG_TEMPVAL_VAR_NAME),
                            format!("{}",format!("{}=({}/{}) - {};",vars.2,driver_var,size2,size2/2)),
                            trans.slice.transform_string(&vars.2,&vars.2,LONG_TEMPVAL_VAR_NAME),
                            format!("{} = {};",&vars.0,default_dac.read.unwrap_or(0)),
                            trans.read.transform_string(&vars.0,&vars.0,LONG_TEMPVAL_VAR_NAME)
                        ];
                        out_str.join("\n")
                    }
                    Dimension::_2D => { // 2-d case
                        let vars = matrix.var_names();
                        let out_str = vec![
                            format!("{}",format!("{}={} - {};",vars.1,driver_var,size1/2)),
                            trans.phase.transform_string(&vars.1,&vars.1,LONG_TEMPVAL_VAR_NAME),
                            // set default values to read and slice
                            format!("{} = {};",&vars.0,default_dac.read.unwrap_or(0)),
                            trans.read.transform_string(&vars.0,&vars.0,LONG_TEMPVAL_VAR_NAME),
                            format!("{} = {};",&vars.2,default_dac.slice.unwrap_or(0)),
                            trans.slice.transform_string(&vars.2,&vars.2,LONG_TEMPVAL_VAR_NAME)
                        ];
                        out_str.join("\n")
                    }
                }
            }
            EncodeStrategy::LUT(dim,_) => {
                match dim {
                    Dimension::_2D => {
                        let vars = matrix.var_names();
                        let out_str = vec![
                            format!("{} = {}+{};",LUT_INDEX_VAR_NAME,driver_var,echo_index),
                            format!("GETLUTENTRY({},{})",LUT_INDEX_VAR_NAME,LUT_TEMPVAL_VAR_NAME_1),
                            trans.phase.transform_string(LUT_TEMPVAL_VAR_NAME_1,&vars.1,LONG_TEMPVAL_VAR_NAME),
                            // set default values to read and slice
                            format!("{} = {};",&vars.0,default_dac.read.unwrap_or(0)),
                            trans.read.transform_string(&vars.0,&vars.0,LONG_TEMPVAL_VAR_NAME),
                            format!("{} = {};",&vars.2,default_dac.slice.unwrap_or(0)),
                            trans.slice.transform_string(&vars.2,&vars.2,LONG_TEMPVAL_VAR_NAME)
                        ];
                        out_str.join("\n")
                    }
                    Dimension::_3D => {
                        let vars = matrix.var_names();
                        let out_str = vec![
                            format!("{} = ({}+{})*2L;",LUT_INDEX_VAR_NAME,driver_var,echo_index),
                            format!("GETLUTENTRY({},{})",LUT_INDEX_VAR_NAME,LUT_TEMPVAL_VAR_NAME_1),
                            trans.phase.transform_string(LUT_TEMPVAL_VAR_NAME_1,&vars.1,LONG_TEMPVAL_VAR_NAME),
                            format!("{} = {} + 1L;",LUT_INDEX_VAR_NAME,LUT_INDEX_VAR_NAME),
                            format!("GETLUTENTRY({},{})",LUT_INDEX_VAR_NAME,LUT_TEMPVAL_VAR_NAME_2),
                            trans.slice.transform_string(LUT_TEMPVAL_VAR_NAME_2,&vars.2,LONG_TEMPVAL_VAR_NAME),
                            format!("{} = {};",&vars.0,default_dac.read.unwrap_or(0)),
                            trans.read.transform_string(&vars.0,&vars.0,LONG_TEMPVAL_VAR_NAME)
                        ];
                        out_str.join("\n")
                    }
                }
            }
        }
    }

}

#[derive(Clone,Debug)]
pub enum MatrixDriverType{
    PhaseEncode(EncodeStrategy),
}

#[derive(Clone,Debug)]
pub struct MatrixDriver{
    kind:MatrixDriverType,
    driver_var:String,
    echo_index:usize
}

pub enum DriverVar {
    Repetition,
    Average
}

impl DriverVar {
    pub fn varname(&self) -> String {
        match self {
            DriverVar::Repetition => String::from(VIEW_LOOP_COUNTER_VAR),
            DriverVar::Average => String::from(AVERAGES_LOOP_COUNTER_VAR)
        }
    }
}

impl MatrixDriver {
    pub fn new(driver_variable:DriverVar,driver_type:MatrixDriverType,echo_index:Option<usize>) -> MatrixDriver {
        MatrixDriver{
            kind:driver_type,
            driver_var:driver_variable.varname(),
            echo_index:echo_index.unwrap_or(0)
        }
    }
    fn render(&self,trans:LinTransform,matrix:&Matrix,default_dac:DacValues) -> String {
        match &self.kind {
            MatrixDriverType::PhaseEncode(strategy) => strategy.print(&self.driver_var,matrix,trans,default_dac,self.echo_index)
        }
    }
}

#[derive(Clone,Debug)]
pub struct Matrix {
    kind:MatrixType,
    label:String,
    uid:u8,
    pub adjustable:(bool,bool,bool),
    pub disabled:bool
}

impl Matrix {
    pub fn new_tracker() -> Rc<RefCell<u8>> {
        Rc::new(RefCell::<u8>::new(1))
    }
    pub fn new_static(label: &str, dac_values: DacValues, adjustable:(bool,bool,bool),disabled:bool,uid_tracker: &Rc<RefCell<u8>>) -> Matrix {
        let uid_tracker = uid_tracker.clone();
        let mut uid = uid_tracker.borrow_mut();
        *uid += 1;
        Matrix {
            kind: MatrixType::Static(dac_values),
            label: label.to_owned(),
            uid: (*uid).clone(),
            adjustable,
            disabled
        }
    }
    pub fn new_derived(label: &str, parent: &Rc<Matrix>, trans: LinTransform, adjustable:(bool,bool,bool),disabled:bool,uid_tracker: &Rc<RefCell<u8>>) -> Matrix {
        let parent = Rc::clone(parent);
        let uid_tracker = uid_tracker.clone();
        let mut uid = uid_tracker.borrow_mut();
        *uid += 1;
        Matrix {
            kind: MatrixType::Derived(parent, trans),
            label: label.to_owned(),
            uid: (*uid).clone(),
            adjustable,
            disabled
        }
    }
    pub fn new_driven(label: &str, driver: MatrixDriver, trans: LinTransform, default_dac: DacValues,adjustable:(bool,bool,bool),disabled:bool, uid_tracker: &Rc<RefCell<u8>>) -> Matrix {
        let uid_tracker = uid_tracker.clone();
        let mut uid = uid_tracker.borrow_mut();
        *uid += 1;
        Matrix {
            kind: MatrixType::Driven(driver.clone(), trans, default_dac),
            label: label.to_owned(),
            uid: (*uid).clone(),
            adjustable,
            disabled
        }
    }
    pub fn derive(&self, label: &str, trans: LinTransform, adjustable:(bool,bool,bool),disabled:bool,uid_tracker: &Rc<RefCell<u8>>) -> Matrix {
        let uid_tracker = uid_tracker.clone();
        let mut uid = uid_tracker.borrow_mut();
        *uid += 1;
        Matrix {
            kind: MatrixType::Derived(Rc::new(self.clone()), trans),
            label: label.to_owned(),
            uid: (*uid).clone(),
            adjustable,
            disabled
        }
    }
    pub fn kind(&self) -> MatrixType {
        self.kind.clone()
    }
    pub fn var_names(&self) -> (String, String, String) {
        (format!("{}_read", self.label), format!("{}_phase", self.label), format!("{}_slice", self.label))
    }
    pub fn var_names_adj(&self) -> (String, String, String) {
        (format!("{}_read_adj", self.label), format!("{}_phase_adj", self.label), format!("{}_slice_adj", self.label))
    }
    pub fn create_matrix(&self) -> String {

        match self.disabled {
            true => {
                vec![
                    format!("CREATE_MATRIX({},0,0,0)", self.label),
                    String::from("delay(100,us);")
                ].join("\n")
            }
            false => {
                let var_names = self.var_names();
                let var_names_adj = self.var_names_adj();
                let arg1 = match self.adjustable.0 {
                    true => format!("{}+{}",var_names.0,var_names_adj.0),
                    false => format!("{}",var_names.0)
                };
                let arg2 = match self.adjustable.1 {
                    true => format!("{}+{}",var_names.1,var_names_adj.1),
                    false => format!("{}",var_names.1)
                };
                let arg3 = match self.adjustable.2 {
                    true => format!("{}+{}",var_names.2,var_names_adj.2),
                    false => format!("{}",var_names.2)
                };
                vec![
                    format!("CREATE_MATRIX({},{},{},{})", self.label, arg3, arg2, arg1),
                    String::from("delay(100,us);")
                ].join("\n")
            }
        }


    }
    pub fn parent_var_names(&self) -> Option<(String, String, String)> {
        match &self.kind {
            MatrixType::Derived(parent, _) => {
                let label = parent.label.clone();
                Some((format!("{}_read", &label), format!("{}_phase", &label), format!("{}_slice", &label)))
            }
            _ => None
        }
    }
    pub fn set_vars(&self) -> String {
        let vars = self.var_names();
        match &self.kind {
            // static matrices get set to their literal dac values
            MatrixType::Static(dac) => {
                let out = vec![
                    format!("{} = {};", vars.0, dac.read.unwrap_or(0)),
                    format!("{} = {};", vars.1, dac.phase.unwrap_or(0)),
                    format!("{} = {};", vars.2, dac.slice.unwrap_or(0))
                ];
                out.join("\n")
            },
            MatrixType::Derived(_, transform) => {
                let vars = self.var_names();
                let parent_vars = self.parent_var_names().unwrap();
                let r = transform.read.transform_string(&parent_vars.0, &vars.0, LONG_TEMPVAL_VAR_NAME);
                let p = transform.phase.transform_string(&parent_vars.1, &vars.1, LONG_TEMPVAL_VAR_NAME);
                let s = transform.slice.transform_string(&parent_vars.2, &vars.2, LONG_TEMPVAL_VAR_NAME);
                format!("{}\n{}\n{}", r, p, s)
            },
            MatrixType::Driven(driver, transform, default_dac) => {
                driver.render(transform.clone(), &self, *default_dac)
            }
        }
    }
    pub fn label(&self) -> String {
        self.label.clone()
    }
    pub fn declaration(&self) -> String {
        // this will need to be set properly once we know about the existence of other matrices
        format!("const {} {};", self.label(), self.uid)
    }
    pub fn vars_declaration(&self) -> String {
        let vars = self.var_names();
        vec![
            format!("int {};", vars.0),
            format!("int {};", vars.1),
            format!("int {};", vars.2),
        ].join("\n")
    }

    pub fn vars_adj_declaration(&self) -> Option<String> {
        let vars = self.var_names_adj();
        let mut decs = Vec::<String>::new();
        match self.adjustable.0 {
            true => decs.push(format!("common int {};", vars.0)),
            false => {}
        }
        match self.adjustable.1 {
            true => decs.push(format!("common int {};", vars.1)),
            false => {}
        }
        match self.adjustable.2 {
            true => decs.push(format!("common int {};", vars.2)),
            false => {}
        }
        match decs.len() > 0 {
            true => Some(decs.join("\n")),
            false => None
        }
    }
    pub fn header_declaration(&self) -> Option<Vec<Adjustment>> {
        let mut scrollbars = Vec::<Adjustment>::new();
        let vars = self.var_names_adj();

        match &self.adjustable.0 {
            true => {
                scrollbars.push(Adjustment::new_grad_adj(&self.label, &vars.0, 20000));
            }
            false => {}
        }
        match &self.adjustable.1 {
            true => {
                scrollbars.push(Adjustment::new_grad_adj(&self.label, &vars.1, 20000));
            }
            false => {}
        }
        match &self.adjustable.2 {
            true => {
                scrollbars.push(Adjustment::new_grad_adj(&self.label, &vars.2, 20000));
            }
            false => {}
        }
        return if scrollbars.len() > 0 { Some(scrollbars) } else { None };
    }
    pub fn dac_vals(&self, driver_value: u32) -> DacValues {
        match &self.kind {
            MatrixType::Static(dac_values) => *dac_values,
            MatrixType::Driven(driver, transform, dac_values) => {
                match &driver.kind {
                    MatrixDriverType::PhaseEncode(strategy) => {
                        strategy.dac_value(driver_value, *transform, *dac_values, driver.echo_index)
                    }
                }
            }
            MatrixType::Derived(parent, transform) => {
                println!("getting parent dac values ...");
                let dac = parent.dac_vals(driver_value);
                let dac_read = transform.read.transform(dac.read);
                let dac_phase = transform.phase.transform(dac.phase);
                let dac_slice = transform.slice.transform(dac.slice);
                DacValues::new(dac_read, dac_phase, dac_slice)
            }
        }
    }
}

// #[test]
// fn test(){
//     // pointer that keeps track of incrementing matrix uids
//     let m_tracker = Matrix::new_tracker();
//
//     /*
//         Create a static matrix with some dac values. This matrix state will
//         never change after it's set. Useful for readout, spoiler, crusher gradients
//      */
//     let dac_vals = DacValues::new(None,Some(500),Some(500));
//     let static_mat = Matrix::new_static("static_mat",dac_vals,&m_tracker);
//     /*
//         Create a driven matrix with a special matrix driver and a set of
//         transforms. The driver has a LUT encoding strategy where LUT values are read
//         and transformed with the supplied linear transform. Table lookup is driven by
//         the completed views variable. If a dac channel is not set by the driver, it will
//         inherit a supplied default dac value that will also be transformed
//      */
//     let driver_type = MatrixDriverType::PhaseEncode(EncodeStrategy::LUT(Dimension::_3D,vec![0,0]));
//     let driver = MatrixDriver::new(DriverVar::Repetition,driver_type,None);
//     let default_dacs = DacValues::new(None,None,None);
//     let fov_transform = LinTransform{
//         read:TransCoeffs{scale:0.0,offset:0},
//         phase:TransCoeffs{scale:30.0,offset:0},
//         slice:TransCoeffs{scale:30.0,offset:0},
//     };
//     let phase_encode_mat = Matrix::new_driven("pe3_mat",driver,fov_transform,default_dacs,&m_tracker);
//     /*
//         Create a derived matrix from a pre-existing one. It is not recommended to derive a matrix
//         from already derived matrix. I may force this to be an error condition. All we need is a parent
//         matrix and a set of transforms. Lets create a re-winder from the previous phase encode
//         matrix
//      */
//     let phase_reverse_transform = LinTransform{
//         read:TransCoeffs{scale:0.0,offset:0},
//         phase:TransCoeffs{scale:-1.0,offset:0},
//         slice:TransCoeffs{scale:-1.0,offset:0},
//     };
//     let rewinder_mat = phase_encode_mat.derive("phase_rewind",phase_reverse_transform,&m_tracker);
//
// }
