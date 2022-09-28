// use std::cell::{Cell, RefCell};
// use crate::execution::{ExecutionBlock, WaveformData};
// use crate::gradient_frame::GradFrame;
// use std::marker::PhantomData;
// use std::collections::{HashMap, HashSet};
// use itertools::Itertools;
// use crate::utils;
// use serde::{Serialize};
// use serde_json;
// use crate::command_string::CommandString;
// use crate::ppl_function::MIN_DELAY_CLOCKS;
//
//
//
// pub struct Event {
//     pub exec_block:Box<dyn ExecutionBlock>,
//     block_post_delay:i32,
//     placement_type:EventPlacementType,
//     kind:EventType,
//     pub label:String,
//     pub event_start:Option<i32>,
//     solved:bool
// }
//
// #[derive(Copy,Clone,Debug)]
// pub struct BlockInterval {
//     block_start:i32,
//     block_end:i32,
//     index:usize
// }
//
// impl BlockInterval {
//     pub fn new(start:i32,end:i32,index:usize) -> Self {
//         Self {
//             block_start:start,
//             block_end:end,
//             index
//         }
//     }
//     pub fn collides_with(&self,interval:BlockInterval) -> bool {
//         self.block_end > interval.block_start && interval.block_end > self.block_start
//     }
// }
//
// impl Event {
//     fn new(event:Box<dyn ExecutionBlock>,placement:EventPlacementType,kind:EventType,unique_label:&str) -> Self{
//         Self {
//             exec_block:event,
//             block_post_delay: MIN_DELAY_CLOCKS,
//             placement_type:placement,
//             kind,
//             label:unique_label.to_owned(),
//             event_start:None,
//             solved:false
//         }
//     }
//     fn set_post_delay(&mut self,post_delay_extention:u32) -> bool {
//         if self.solved {return false}
//         self.block_post_delay = self.block_post_delay + post_delay_extention as i32;
//         self.solved = true;
//         true
//     }
//     fn min_block_duration(&self) -> i32 {
//         self.exec_block.block_duration() + MIN_DELAY_CLOCKS
//     }
//     pub fn block_end(&self) -> Option<i32> {
//         match self.event_start{
//             None => {return None}
//             Some(start) => {
//                 match self.kind{
//                     EventType::NonBlocking => Some(start + self.exec_block.block_duration() + MIN_DELAY_CLOCKS),
//                     EventType::Blocking => Some(start + self.exec_block.time_to_end() + MIN_DELAY_CLOCKS)
//                 }
//             }
//         }
//     }
//     pub fn block_interval_sec(&self) -> Option<(f32,f32)> {
//         let start = match self.event_start {
//             None => return None,
//             Some(start) => utils::clock_to_sec(start)
//         };
//         let end = match self.block_end() {
//             None => return None,
//             Some(end) => utils::clock_to_sec(end)
//         };
//         Some((start,end))
//     }
//     pub fn event_graph(&self,time_step_us:usize) -> Option<EventGraph> {
//         match self.block_interval_sec() {
//             None => return None,
//             Some(interval) => {
//                 Some(EventGraph{
//                     label:self.label.to_owned(),
//                     block_interval:interval,
//                     event_origin:interval.0 + utils::clock_to_sec(self.exec_block.time_to_start()),
//                     wave_data:self.exec_block.render(time_step_us)
//                 })
//             }
//         }
//     }
// }
//
// pub struct EventQueue {
//     pub items:RefCell<Vec<Event>>,
//     label_hash:HashMap<String,usize>,
//     origin:Option<String>,
// }
//
// impl EventQueue {
//     pub fn new() -> Self {
//         Self {
//             items: RefCell::new(Vec::<Event>::new()),
//             label_hash: HashMap::<String, usize>::new(),
//             origin: None
//         }
//     }
//     pub fn add(&mut self, event: Box<dyn ExecutionBlock>, placement: EventPlacementType,event_type:EventType) -> String {
//         use EventPlacementType::*;
//         match placement {
//             Origin => {
//                 if self.origin.is_some() { panic!("origin has already been set!!"); }
//                 let valid_label = self.push_event(event, placement,event_type);
//                 self.origin = Some(valid_label.clone());
//                 return valid_label;
//             }
//             _ => self.push_event(event, placement,event_type)
//         }
//     }
//     fn push_event(&mut self, event: Box<dyn ExecutionBlock>, placement: EventPlacementType,kind:EventType) -> String {
//         let valid_label = self.find_unique_key(&event.label());
//         self.label_hash.insert(valid_label.clone(), self.items.into_inner().len());
//         self.items.get_mut().push(
//             Event::new(event, placement,kind,&valid_label)
//         );
//         valid_label
//     }
//     fn set_origin(&mut self, origin_name: &str) {
//         if self.label_hash.contains_key(origin_name) {
//             self.origin = Some(origin_name.to_owned());
//         } else {
//             panic!("origin label not found in collection!!");
//         }
//     }
//     fn find_unique_key(&self, key: &str) -> String {
//         let mut ktmp = key.to_owned();
//         let mut i = 1;
//         loop {
//             if !self.label_hash.contains_key(&ktmp) {
//                 return ktmp
//             } else {
//                 ktmp = format!("{}_{}", ktmp,i);
//                 i = i + 1;
//             }
//         }
//     }
//     fn label_to_event_ref_mut(&self, label: &str) -> &mut Event {
//         let idx = self.label_hash.get(label).expect("label not found!!");
//         &mut self.items.get_mut()[*idx]
//     }
//     fn events(&self) -> Vec<&mut Event> {
//         let start_times = self.start_times();
//         let sorted_idx = utils::argsort(start_times);
//         let mut out = Vec::<&mut Event>::new();
//         for idx in sorted_idx.iter(){
//             out.push(self.items[*idx].get_mut());
//         }
//         out
//     }
//     fn index_to_label(&self, index: usize) -> String {
//         if index >= self.items.len() { panic!("index is out of bounds!!") }
//         let event = self.items[index].into_inner();
//         event.label.clone()
//     }
//     fn single_origin(&self) -> bool {
//         let mut origin_found = false;
//         for (index, item) in self.items.iter().enumerate() {
//             match item.into_inner().placement_type {
//                 EventPlacementType::Origin => {
//                     if origin_found {
//                         println!("more than one origin detected at index {}!", index);
//                         return false
//                     };
//                     origin_found = true;
//                 }
//                 _ => {}
//             }
//         }
//         origin_found
//     }
//     fn static_intervals(&self) -> Vec<BlockInterval> {
//         if !self.single_origin() { panic!("problem with event origin!!") }
//         use EventPlacementType::*;
//         self.items.iter().enumerate().flat_map(|(index, item)| {
//             match item.into_inner().placement_type {
//                 Origin => {
//                     let item = item.into_inner();
//                     let start = 0 - item.exec_block.time_to_center();
//                     let end = match item.kind{
//                         EventType::NonBlocking => start + item.exec_block.block_duration() + MIN_DELAY_CLOCKS,
//                         EventType::Blocking => start + item.exec_block.time_to_end() + MIN_DELAY_CLOCKS
//                     };
//                     Some(BlockInterval { block_start: start, block_end: end, index })
//                 }
//                 ExactFromOrigin(location) => {
//                     let item = item.into_inner();
//                     let start = location - item.exec_block.time_to_center();
//                     let end = match item.kind{
//                         EventType::NonBlocking => start + item.exec_block.block_duration() + MIN_DELAY_CLOCKS,
//                         EventType::Blocking => start + item.exec_block.time_to_end() + MIN_DELAY_CLOCKS
//                     };
//                     Some(BlockInterval { block_start: start, block_end: end, index })
//                 }
//                 _ => None
//             }
//         }).collect()
//     }
//     fn set_static_events(&mut self) {
//         use EventPlacementType::*;
//         let ints = self.static_intervals();
//         let collisions = check_block_intervals(&ints);
//         match collisions.0 { // boolean indicating a collision occurred
//             false => {
//                 for i in ints.iter() {
//                     let e = &mut self.items[i.index].get_mut();
//                     match &e.placement_type {
//                         Origin => {
//                             // calculate the start based on the event center (0)
//                             e.event_start = Some(0 - e.exec_block.time_to_center());
//                         }
//                         ExactFromOrigin(location) => {
//                             e.event_start = Some(location - e.exec_block.time_to_center())
//                         }
//                         _ => {// no op}
//                         }
//                     }
//                 }
//                 println!("static events set");
//             }
//             true => {
//                 for collision in collisions.1 {
//                     self.report_collisions(collision)
//                 }
//                 panic!("static collisions must be fixed");
//             }
//         }
//     }
//     fn set_dynamic_events(&mut self) -> bool {
//         use EventPlacementType::*;
//         let mut collision_detected = false;
//         let dynamic_intervals = self.dynamic_intervals();
//         let mut static_intervals = self.static_intervals();
//         for int in dynamic_intervals.iter() {
//             static_intervals.push(int.clone());
//             let collisions = check_block_intervals(&static_intervals);
//             match collisions.0 {
//                 false => {
//                     let mut e = &mut self.items[int.index].get_mut();
//                     e.event_start = Some(int.block_start);
//                 }
//                 true => {
//                     println!("collison detected when solving for event {}", int.index);
//                     for collision in collisions.1.iter(){
//                         self.report_collisions(*collision);
//                     }
//                     collision_detected = true;
//                 }
//             }
//         }
//         collision_detected
//     }
//     fn label_to_index(&self, label: &str) -> usize {
//             *self.label_hash.get(label).expect(&format!("label {}  doesn't exist", label))
//         }
//     fn dynamic_intervals(&self) -> Vec<BlockInterval> {
//             use EventPlacementType::*;
//             let mut out = Vec::<BlockInterval>::new();
//             for (index, event) in self.items.iter().enumerate() {
//                 let event = event.into_inner();
//                 match &event.placement_type {
//                     After(parent, offset) => {
//                         let idx = self.label_to_index(parent);
//                         let p = &self.items[idx].into_inner();
//                         match &p.event_start {
//                             Some(parent_start) => {
//                                 let parent_end = parent_start + p.min_block_duration();
//                                 let start = parent_end + (*offset as i32);
//                                 let end = match event.kind{
//                                     EventType::NonBlocking => start + event.min_block_duration(),
//                                     EventType::Blocking => start + event.exec_block.time_to_end() + MIN_DELAY_CLOCKS
//                                 };
//                                 // child event will not collide with parent in this context
//                                 out.push(BlockInterval::new(start, end, index));
//                             }
//                             None => {
//                                 println!("parent has not been set yet ... continuing");
//                             }
//                         }
//                     }
//                     Before(parent, offset) => {
//                         let idx = self.label_to_index(parent);
//                         let p = &self.items[idx].into_inner();
//                         match &p.event_start {
//                             Some(parent_start) => {
//                                 let end = parent_start - (*offset as i32);
//                                 let start = match event.kind{
//                                     EventType::NonBlocking => end - event.min_block_duration(),
//                                     EventType::Blocking => end - event.exec_block.time_to_end() + MIN_DELAY_CLOCKS
//                                 };
//                                 // child event will not collide with parent in this context
//                                 out.push(BlockInterval::new(start, end, index));
//                             }
//                             None => {
//                                 println!("parent has not been set yet ... continuing");
//                             }
//                         }
//                     }
//                     _ => {}
//                 }
//             }
//             out
//         }
//     fn report_collisions(&self,colliding_block_intervals:(BlockInterval,BlockInterval)){
//         let label1 = self.index_to_label(colliding_block_intervals.0.index);
//         let label2 = self.index_to_label(colliding_block_intervals.1.index);
//         println!("{} & {} collide with ranges {}:{} & {}:{}",label1,label2,
//                  utils::clock_to_us(colliding_block_intervals.0.block_start),
//                  utils::clock_to_us(colliding_block_intervals.0.block_end),
//                  utils::clock_to_us(colliding_block_intervals.1.block_start),
//                  utils::clock_to_us(colliding_block_intervals.1.block_end)
//         );
//     }
//     fn event_set_check(&self) -> bool{
//         let mut unset_event = false;
//         for event in self.items.iter(){
//             let event = event.into_inner();
//             if event.event_start.is_none(){
//                 unset_event = true;
//                 println!("event {} has not been set.",event.label);
//             }
//         }
//         if !unset_event{
//             println!("all events set");
//         }
//         unset_event
//     }
//     pub fn solve(&mut self){
//         self.set_static_events();
//         loop {
//             let collision = self.set_dynamic_events();
//             if collision {panic!("collisions must be fixed")}
//             if !self.event_set_check(){
//                 break
//             }
//             println!("attempting to reset events");
//         }
//     }
//     pub fn export_calc_blocks(&self) -> Vec<CommandString> {
//         self.items.iter().flat_map(|item| item.into_inner().exec_block.block_calculation()).collect()
//     }
//     pub fn export_exec_blocks(&self) -> Vec<CommandString> {
//         self.items.iter().map(|item|
//             item.into_inner().exec_block.block_execution(item.into_inner().block_post_delay).cmd_string()
//         ).collect()
//     }
//     fn sort_events(&self) -> Vec<&mut Event>{
//         let start_times = self.start_times();
//         let sorted_idx = utils::argsort(start_times);
//         let mut out = Vec::<&mut Event>::new();
//         for idx in sorted_idx.iter(){
//             out.push(self.items[*idx].get_mut());
//         }
//         out
//     }
//     fn start_times(&self) -> Vec<i32> {
//         self.items.iter().map(|item| item.into_inner().event_start.expect("event not set")).collect()
//     }
//     fn set_post_delay(&mut self) {
//         // set all event post delays except for the last one, which will include tr makeup time
//         // events need to be ordered based on event start
//         let mut event_refs = self.sort_events();
//         for i in 0..event_refs.len()-1 {
//             let this_end = event_refs[i].block_end().expect("event timing not set");
//             let next_start = event_refs[i+1].event_start.expect("event timing not set!!");
//             println!("end:{} -> start:{}",this_end,next_start);
//             let difference = next_start - this_end;
//             if difference < 0 {panic!("there is something wrong with event placement. Has the queue been solved?")}
//             event_refs[i].set_post_delay(difference as u32);
//         }
//     }
//     fn set_tr_makeup(&mut self,rep_time:f32,loop_time:i32,calc_time:i32) {
//         // set the last event post delay such that tr is accurate
//         let mut event_refs = self.sort_events();
//         let first_event_start = event_refs[0].event_start.expect("event timing not set!");
//         let last_event_idx = event_refs.len()-1;
//         let last_event_end = event_refs[last_event_idx].block_end().expect("event timing not set!");
//         let tr = utils::sec_to_clock(rep_time);
//         let makeup = tr + first_event_start - last_event_end - loop_time - calc_time;
//         if makeup < 0 {
//             let adj = rep_time - utils::clock_to_sec(makeup);
//             panic!("rep time is too short. It needs to be at least {} seconds for loop execution",adj)
//         }
//         self.items[last_event_idx].get_mut().set_post_delay(makeup as u32);
//     }
//     pub fn set_rep_time(&mut self,rep_time:f32,loop_time:i32,calc_time:i32){
//         self.set_post_delay();
//         self.set_tr_makeup(rep_time,loop_time,calc_time);
//     }
// }
// fn check_block_intervals(intervals: &Vec<BlockInterval>) -> (bool,Vec<(BlockInterval,BlockInterval)>) {
//     let mut collision_indices = Vec::<(BlockInterval,BlockInterval)>::new();
//     let mut n_collisions = 0;
//     // check all combinations of intervals for collisions
//     for pair in intervals.iter().combinations(2) {
//         let i1 = *pair.first().unwrap();
//         let i2 = *pair.last().unwrap();
//         let collision = i1.collides_with(*i2);
//         println!("checking {} & {}",i1.index, i2.index);
//         println!("checking {}:{} & {}:{}",i1.block_start,i1.block_end, i2.block_start,i2.block_end);
//         if collision {
//             println!("collision detected between {} and {} ...", i1.index, i2.index);
//             collision_indices.push((*i1,*i2));
//         }
//         n_collisions += collision as i32;
//     }
//     (n_collisions > 0,collision_indices)
// }