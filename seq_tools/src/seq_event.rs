// use crate::acq_event::AcqEvent;
// use crate::execution::ExecutionBlock;
// use crate::gradient_event::GradEvent;
// use crate::gradient_frame::GradFrame;
// use crate::rf_event::RfEvent;
// use crate::rf_frame::RfFrame;
//
// pub struct EventQueue {
//     items:Vec::<Box<dyn ExecutionBlock>>
// }
//
// impl<EB:ExecutionBlock> EventQueue  {
//     pub fn new() -> Self {
//         Self {
//             items:Vec::<Box<dyn ExecutionBlock>>::new()
//         }
//     }
//     pub fn add(&mut self,event:EB) {
//         self.items.push(Box::new(event.clone()));
//     }
// }