// use crate::matrix_driver::{PhaseSliceDriver,PhaseDriver};
// use crate::rf_event::RfPhaseCycle3D;
//
// #[derive(Clone)]
// pub struct LUT {
//     driver_var:String,
//     temp_lut_var_1:String,
//     temp_lut_var_2:String,
//     //kspace_dims:(usize,Option<usize>),
//     is_16bit:bool,
//     pph_path:String
// }
//
// #[derive(Clone)]
// pub struct FullySampled {
//     driver_var:String,
//     kspace_dims:(usize,Option<usize>)
// }
//
// impl FullySampled {
//     pub fn new(driver_var:&str,kspace_dims:(usize,Option<usize>)) -> FullySampled{
//         FullySampled{
//             driver_var:driver_var.to_string(),
//             kspace_dims
//         }
//     }
//
//     pub fn fully_sampled_3d(&self,matrix_vars:(&str,&str),kspace_dims:(usize,usize)) -> String {
//         let c1 = format!("{}",format!("{}=({}%{}) - {};",matrix_vars.0,self.driver_var,kspace_dims.0,kspace_dims.0/2));
//         let c2 = format!("{}",format!("{}=({}/{}) - {};",matrix_vars.1,self.driver_var,kspace_dims.1,kspace_dims.1/2));
//         format!("{}\n{}",c1,c2)
//     }
//
//     pub fn fully_sampled_2d(&self,matrix_var:&str,kspace_dim:usize) -> String {
//         format!("{}",format!("{}={} - {};",matrix_var,self.driver_var,kspace_dim/2))
//     }
// }
//
// impl LUT {
//     pub fn new(driver_var:&str,is_16bit:bool) -> LUT {
//         LUT {
//             driver_var:driver_var.to_string(),
//             temp_lut_var_1:"c_lut_tempval_1".to_string(),
//             temp_lut_var_2:"c_lut_tempval_2".to_string(),
//             is_16bit,
//             pph_path:r"C:\smis\include\lututils.pph".to_string()
//         }
//     }
//
//     // return some code elements
//     pub fn two_element(&self,out_vars:(&str,&str)) -> String{
//         let mut out_str = Vec::<String>::new();
//         out_str.push(lut_read(&self.driver_var,out_vars.0,2,0,(1,0)));
//         out_str.push(lut_read(&self.driver_var,out_vars.1,2,1,(1,0)));
//         out_str.join("\n")
//     }
//
//     pub fn view_cycle_180_3d(&self,out_var:&str,kspace_dims:(usize,usize)) -> String{
//         let mut out_str = Vec::<String>::new();
//         out_str.push(format!("{} = {}*{} + {};",self.temp_lut_var_1,2,self.driver_var,0));
//         out_str.push(format!("GETLUTENTRY({},{})","lut_idx",self.temp_lut_var_1));
//         out_str.push(format!("{} = {}*{} + {};",self.temp_lut_var_2,2,self.driver_var,1));
//         out_str.push(format!("GETLUTENTRY({},{})","lut_idx",self.temp_lut_var_1));
//         out_str.push(format!("{}=(({}+{}+{}+{})%2)*2+1;",out_var,self.temp_lut_var_1,self.temp_lut_var_2,kspace_dims.0,kspace_dims.1));
//         out_str.join("\n")
//     }
//
//     pub fn single_element(&self,out_var:&str) -> String{
//         lut_read(&self.driver_var,out_var,1,0,(1,0))
//     }
//
//     pub fn include(&self) -> String {
//         //#include "C:\smis\include\lututils.pph"
//         // is16bit = 1;
//         let mut out_str = Vec::<String>::new();
//         out_str.push(format!("#include {}",self.pph_path));
//         out_str.push(format!("is16bit = {};",self.is_16bit as i16));
//         out_str.join("\n")
//     }
//
// }
//
// impl PhaseSliceDriver for LUT {
//     fn render(&self,phase_var:&str,slice_var:&str) -> String {
//         self.two_element((phase_var,slice_var))
//     }
// }
//
// impl PhaseDriver for LUT {
//     fn render(&self,phase_var:&str) -> String {
//         self.single_element(phase_var)
//     }
// }
//
// impl PhaseDriver for FullySampled {
//     fn render(&self,phase_var:&str) -> String {
//         self.fully_sampled_2d(phase_var,self.kspace_dims.0)
//     }
// }
//
// impl PhaseSliceDriver for FullySampled {
//     fn render(&self,phase_var:&str,slice_var:&str) -> String {
//         let dim_slice = match self.kspace_dims.1 {
//             Some(dim) => dim,
//             None => self.kspace_dims.0
//         };
//         self.fully_sampled_3d((phase_var,slice_var),(self.kspace_dims.0,dim_slice))
//     }
// }
//
// // impl RfPhaseCycle3D_180 for LUT {
// //     fn phase_cycle_3d(&self,phase_var:&str) -> String {
// //         self.two_element()
// //     }
// // }
//
// pub fn lut_read(view_counter_var:&str,target_var:&str,n_lookups_per_view:usize,offset:usize,linear_coeffs:(i16,i16)) -> String{
//     let mut out_str = Vec::<String>::new();
//     out_str.push(format!("{} = {}*{} + {};","lut_idx",n_lookups_per_view,view_counter_var,offset));
//     out_str.push(format!("GETLUTENTRY({},{})","lut_idx","lut_tempval"));
//     out_str.push(format!("{} = {}*{} + {};",target_var,"lut_tempval",linear_coeffs.0,linear_coeffs.1));
//     out_str.join("\n")
// }
//
//
//
// #[test]
// fn test(){
//     println!("driver test ...");
//     // lut is driven by view_count, with 2 lookups per view and 16 bit values
//     let lut = LUT::new("view_count",true);
//     let include_code = lut.include();
//     let lut_code = lut.two_element(("phase1","phase2"));
//     println!("{}\n{}",include_code,lut_code);
// }