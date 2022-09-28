use crate::gradient_frame::GradFrame;
use crate::pulse::Trapezoid;
use crate::gradient_matrix::{Matrix,DacValues};
//use crate::matrix_driver::{, DacMatrix};
use crate::seqframe::GRAD_SEQ_FILE_LABEL;

pub struct GradList<GF> where GF:GradFrame{
    frame:GF,
    label:String,
}

impl<GF> GradList<GF> where GF:GradFrame + Copy {
    pub fn new(grad_frame:GF,label:&str) -> GradList<GF> {
        GradList{
            frame:grad_frame,
            label:label.to_string()
        }
    }

    pub fn render(&self) -> String {
        let init = format!("{} = MR3040_InitList();",&self.label);
        let mut args = Vec::<String>::new();
        args.push(format!("{}.address.\"{}\"",GRAD_SEQ_FILE_LABEL,&self.label));
        args.push(format!("{}.size.\"{}\"",GRAD_SEQ_FILE_LABEL,&self.label));
        args.push(format!("{}.waits.\"{}\"",GRAD_SEQ_FILE_LABEL,&self.label));
        format!("{}\nMR3040_Output(NOLOOP,{});",init,args.join(","))
    }
}


#[test]
fn test(){
    // let d = DacValues::new(Some(10),None,None);
    // let m = Matrix::new_static("mat1",d);
    let t = Trapezoid::new(100E-6,1E-3);
    let t_list = GradList::new(t,"readout_trapezoid");
    let s = t_list.render();
    println!("{}",s);
}