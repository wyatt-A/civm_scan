use seq_tools::event_block::EventQueue;

pub trait PulseSequence {
    fn place_events(&self) -> EventQueue;
    fn seq_file_export(&self);
    fn ppl_export(&self);

}


//    pub fn seq_export(&self,sample_period_us:usize,filepath:&str){
//         let q = self.place_events();
//         let (grad_params,rf_params) = q.ppl_seq_params(sample_period_us);
//         //let path = std::env::current_dir().expect("cannot get current dir");
//         let path = Path::new(filepath);
//         let grad_param = Path::new("civm_grad_params").with_extension("txt");
//         let grad_param_path = path.join(grad_param);
//         let rf_param = Path::new("civm_rf_params").with_extension("txt");
//         let rf_param_path = path.join(rf_param);
//         let mut rf_seq_file = File::create(rf_param_path).expect("cannot create file");
//         rf_seq_file.write_all(&SeqFrame::format_as_bytes(&rf_params.unwrap())).expect("trouble writing to file");
//         let mut grad_seq_file = File::create(grad_param_path).expect("cannot create file");
//         grad_seq_file.write_all(&SeqFrame::format_as_bytes(&grad_params.unwrap())).expect("trouble writing to file");
//     }