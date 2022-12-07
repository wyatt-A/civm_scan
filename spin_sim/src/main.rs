use spin_sim::spin_operators::transform;
use spin_sim::matmath::{Vector,Matrix};
use spin_sim::spin::{Spin,SpinSystem};
use std::vec;
use spin_sim::spin::Event;

fn main(){

    let mut s = Spin::new();
    s.m = Vector::new(1.0,0.0,0.0);
    s.delta_b = 1.0E-6;

    let mut ss = SpinSystem::new();

    for _ in 0..1{
        ss.spins.push(s.clone());
    }

    for (_, e) in ss.spins.iter().enumerate() {
        println!("{}",e);
    }

    let e = Event{gradient:Vector::new(0.0,0.0,1.0),rf:Vector::new(0.0,0.0,0.0),duration:1.0E-6};

    ss.apply(&e);

    ss.spins.iter().for_each(|spin|{println!("{}",spin)});
    
}
