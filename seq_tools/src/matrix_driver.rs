// use crate::lut_driver::{LUT,FullySampled};
//
// pub trait Matrix {
//     fn label(&self) -> String;
//     fn render(&self) -> String;
//     fn magnitude_vars(&self) -> (String,String,String);
//     fn create_matrix(&self) -> String;
// }
//
// trait Derivable {
//     fn derive(&self,label:&str,linear_coeffs:&DerivedLinearMatrixCoeffs) -> DerivedMatrix;
// }
//
// // something that determines the matrix values for phase and slice
// pub trait PhaseSliceDriver {
//     fn render(&self,phase_var:&str,slice_var:&str) -> String;
// }
//
// pub trait PhaseDriver {
//     fn render(&self,phase_var:&str) -> String;
// }
//
// #[derive(Clone)]
// pub struct DerivedMatrix {
//     label:String,
//     parent:String,
//     linear_coeffs:LinearMatixCoeffs
// }
//
// #[derive(Clone)]
// pub struct DacMatrix {
//     label:String,
//     linear_coeffs:LinearMatixCoeffs
// }
//
// #[derive(Clone)]
// pub struct PhaseEncode2Matrix<D> where D:PhaseSliceDriver + Clone {
//     label:String,
//     driver:D,
//     linear_coeffs:LinearMatixCoeffs
// }
//
// #[derive(Clone)]
// pub struct PhaseEncode1Matrix<D> where D:PhaseDriver {
//     label:String,
//     driver:D,
//     linear_coeffs:LinearMatixCoeffs
// }
//
// #[derive(Clone)]
// pub struct LinearMatixCoeffs {
//     pub read:(i16, i16),
//     pub phase:(i16, i16),
//     pub slice:(i16, i16)
// }
//
// #[derive(Clone)]
// pub struct DerivedLinearMatrixCoeffs {
//     read:(f32,i16),
//     phase:(f32,i16),
//     slice:(f32,i16)
// }
//
// impl LinearMatixCoeffs {
//     pub fn new_composite(&self,matrix:&DerivedLinearMatrixCoeffs) -> LinearMatixCoeffs {
//         LinearMatixCoeffs{
//             read:LinearMatixCoeffs::derive_coeffs(self.read,matrix.read),
//             phase:LinearMatixCoeffs::derive_coeffs(self.phase,matrix.phase),
//             slice:LinearMatixCoeffs::derive_coeffs(self.slice,matrix.slice),
//         }
//     }
//     fn derive_coeffs(lhs:(i16,i16),rhs:(f32,i16)) -> (i16,i16) {
//         //a3 = a1*a2
//         //b3 = b1*a2 + b2
//         let c0 = (lhs.0 as f32)*rhs.0;
//         let c1 = (lhs.1 as f32)*rhs.0 + rhs.1 as f32;
//         (c0 as i16,c1 as i16)
//     }
// }
//
// impl DacMatrix {
//     pub fn new(label:&str,linear_coeffs:&LinearMatixCoeffs) -> DacMatrix {
//         DacMatrix{
//             label:label.to_string(),
//             linear_coeffs:linear_coeffs.clone()
//         }
//     }
// }
//
// impl Matrix for DacMatrix {
//     fn label(&self) -> String {
//         self.label.clone()
//     }
//     fn render(&self) -> String{
//         self.create_matrix()
//     }
//     fn magnitude_vars(&self) -> (String,String,String) {
//         (
//             format!("{}_read",&self.label),
//             format!("{}_phase",&self.label),
//             format!("{}_slice",&self.label)
//         )
//     }
//     fn create_matrix(&self) -> String {
//         let vars = self.magnitude_vars();
//         let mut args = Vec::<String>::new();
//         args.push(format!("{}*{}+{}",vars.0,self.linear_coeffs.read.0,self.linear_coeffs.read.1));
//         args.push(format!("{}*{}+{}",vars.1,self.linear_coeffs.phase.0,self.linear_coeffs.phase.1));
//         args.push(format!("{}*{}+{}",vars.2,self.linear_coeffs.slice.0,self.linear_coeffs.slice.1));
//         args.reverse();
//         let arg = args.join(",");
//         format!("CREATE_MATRIX({},{})",self.label,arg)
//     }
// }
// impl Derivable for DacMatrix {
//     fn derive(&self,label:&str,linear_coeffs:&DerivedLinearMatrixCoeffs) -> DerivedMatrix {
//         let composite_coeffs = self.linear_coeffs.new_composite(linear_coeffs);
//         DerivedMatrix{
//             label:label.to_string(),
//             parent:self.label.clone(),
//             linear_coeffs:composite_coeffs
//         }
//     }
// }
//
// impl Matrix for DerivedMatrix {
//     fn label(&self) -> String {
//         self.label.clone()
//     }
//     fn render(&self) -> String{
//         self.create_matrix()
//     }
//     fn create_matrix(&self) -> String {
//         let vars = self.magnitude_vars();
//         let mut args = Vec::<String>::new();
//         args.push(format!("{}*{}+{}",vars.0,self.linear_coeffs.read.0,self.linear_coeffs.read.1));
//         args.push(format!("{}*{}+{}",vars.1,self.linear_coeffs.phase.0,self.linear_coeffs.phase.1));
//         args.push(format!("{}*{}+{}",vars.2,self.linear_coeffs.slice.0,self.linear_coeffs.slice.1));
//         args.reverse();
//         let arg = args.join(",");
//         format!("CREATE_MATRIX({},{})",self.label,arg)
//     }
//     fn magnitude_vars(&self) -> (String,String,String) {
//         (
//             format!("{}_read",&self.parent),
//             format!("{}_phase",&self.parent),
//             format!("{}_slice",&self.parent)
//         )
//     }
// }
//
// impl<D> PhaseEncode2Matrix<D> where D:PhaseSliceDriver + Clone {
//     pub fn new(label:&str,driver:&D,linear_coeffs:&LinearMatixCoeffs) -> PhaseEncode2Matrix<D> {
//         PhaseEncode2Matrix{
//             label:label.to_string(),
//             driver:driver.clone(),
//             linear_coeffs:linear_coeffs.clone()
//         }
//     }
// }
//
// impl<D> Matrix for PhaseEncode2Matrix<D> where D:PhaseSliceDriver + Clone {
//     fn label(&self) -> String {
//         self.label.clone()
//     }
//     fn magnitude_vars(&self) -> (String,String,String) {
//         (
//             format!("{}_read",&self.label),
//             format!("{}_phase",&self.label),
//             format!("{}_slice",&self.label)
//         )
//     }
//     fn create_matrix(&self) -> String {
//         let vars = self.magnitude_vars();
//         let mut args = Vec::<String>::new();
//         args.push(format!("{}*{}+{}",vars.0,self.linear_coeffs.read.0,self.linear_coeffs.read.1));
//         args.push(format!("{}*{}+{}",vars.1,self.linear_coeffs.phase.0,self.linear_coeffs.phase.1));
//         args.push(format!("{}*{}+{}",vars.2,self.linear_coeffs.slice.0,self.linear_coeffs.slice.1));
//         args.reverse();
//         let arg = args.join(",");
//         format!("CREATE_MATRIX({},{})",self.label,arg)
//     }
//     fn render(&self) -> String {
//         let vars = self.magnitude_vars();
//         let lookup = self.driver.render(&vars.1,&vars.2);
//         format!("{}\n{}",lookup,self.create_matrix())
//     }
// }
//
// impl<D> Derivable for PhaseEncode2Matrix<D> where D:PhaseSliceDriver + Clone {
//     fn derive(&self,label:&str,linear_coeffs:&DerivedLinearMatrixCoeffs) -> DerivedMatrix {
//         let composite_coeffs = self.linear_coeffs.new_composite(linear_coeffs);
//         DerivedMatrix{
//             label:label.to_string(),
//             parent:self.label.clone(),
//             linear_coeffs:composite_coeffs
//         }
//     }
// }
//
// impl<D> PhaseEncode1Matrix<D> where D:PhaseDriver + Clone {
//     pub fn new(label:&str,driver:&D,linear_coeffs:&LinearMatixCoeffs) -> PhaseEncode1Matrix<D> {
//         PhaseEncode1Matrix{
//             label:label.to_string(),
//             driver:driver.clone(),
//             linear_coeffs:linear_coeffs.clone()
//         }
//     }
// }
//
// impl<D> Matrix for PhaseEncode1Matrix<D> where D:PhaseDriver + Clone {
//     fn label(&self) -> String {
//         self.label.clone()
//     }
//     fn magnitude_vars(&self) -> (String,String,String) {
//         (
//             format!("{}_read",&self.label),
//             format!("{}_phase",&self.label),
//             format!("{}_slice",&self.label)
//         )
//     }
//     fn create_matrix(&self) -> String {
//         let vars = self.magnitude_vars();
//         let mut args = Vec::<String>::new();
//         args.push(format!("{}*{}+{}",vars.0,self.linear_coeffs.read.0,self.linear_coeffs.read.1));
//         args.push(format!("{}*{}+{}",vars.1,self.linear_coeffs.phase.0,self.linear_coeffs.phase.1));
//         args.push(format!("{}*{}+{}",vars.2,self.linear_coeffs.slice.0,self.linear_coeffs.slice.1));
//         args.reverse();
//         let arg = args.join(",");
//         format!("CREATE_MATRIX({},{})",self.label,arg)
//     }
//     fn render(&self) -> String {
//         let vars = self.magnitude_vars();
//         let lookup = self.driver.render(&vars.1);
//         format!("{}\n{}",lookup,self.create_matrix())
//     }
// }
//
// impl<D> Derivable for PhaseEncode1Matrix<D> where D:PhaseDriver + Clone {
//     fn derive(&self,label:&str,linear_coeffs:&DerivedLinearMatrixCoeffs) -> DerivedMatrix {
//         let composite_coeffs = self.linear_coeffs.new_composite(linear_coeffs);
//         DerivedMatrix{
//             label:label.to_string(),
//             parent:self.label.clone(),
//             linear_coeffs:composite_coeffs
//         }
//     }
// }
//
// #[test]
// fn test(){
//     let c1 = LinearMatixCoeffs{
//         read:(1,0),
//         phase:(1,0),
//         slice:(1,0)
//     };
//
//     let c2 = DerivedLinearMatrixCoeffs{
//         read:(-1.0,0),
//         phase:(-1.0,0),
//         slice:(-1.0,0)
//     };
//
//     let m1 = DacMatrix::new("mat1",&c1);
//     //let m2 = DerivedMatrix::new("derived",&m1,&c2);
//     //println!("{}",m1.render());
//     //println!("{}",m2.render());
//
//     let lut = LUT::new("view_count",true);
//
//     let fs = FullySampled::new("view_count",(64,Some(64)));
//
//     let md = PhaseEncode2Matrix::new("pe",&lut,&c1);
//     let md2 = PhaseEncode2Matrix::new("pe",&fs,&c1);
//
//     let pe1 = PhaseEncode1Matrix::new("pe2d",&lut,&c1);
//
//
//     let g = pe1.derive("derived",&c2);
//     let g2 = md2.derive("rewind",&c2);
//
//     println!("{}",pe1.render());
//     println!("{}",g.render());
// }
//
