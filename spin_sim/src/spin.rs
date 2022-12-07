use serde::{Deserialize, Serialize};
use serde_json::{to_string,from_str};
use crate::matmath::Vector;
use crate::spin_operators;
use std::{fmt,vec};
use std::fs::{File, OpenOptions};
use std::io::{prelude, Write};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Spin{
    pub r:Vector,
    pub m:Vector,
    pub t1:f32,
    pub t2:f32,
    pub m0:f32,
    pub delta_b:f32,
    pub gamma:f32
}

impl Spin{
    pub fn new() -> Spin{
        return Spin{
            r:Vector::null(),
            m:Vector::unit('z'),
            t1:1.0,
            t2:0.05,
            m0:1.0,
            delta_b:0.0,
            gamma:42.0e6
        }
    }

    pub fn step(&mut self,b:&Vector,tau:f32){
        self.m = spin_operators::transform(self.m,b,self.t1, self.t2,self.gamma, self.m0,tau);
    }
}

impl fmt::Display for Spin {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "r:{}\nm:{}\nt1:{}\nt2:{}\nm0:{}\ndelta_b:{}\ngamma:{}",
        self.r,self.m,self.t1,self.t2,self.m0,self.delta_b,self.gamma)
    }
}

impl PartialEq for Spin {
    fn eq(&self, other: &Self) -> bool {
        return self.r == other.r && 
        self.m == other.m && 
        self.t1 == other.t1 &&
        self.t2 == other.t2 &&
        self.m0 == other.m0 &&
        self.delta_b == other.delta_b &&
        self.gamma == other.gamma;
    }
}

pub struct SpinSystem{
    pub spins:Vec<Spin>,
}

pub struct Event{
    pub gradient:Vector,
    pub rf:Vector,
    pub duration:f32
}

impl SpinSystem{
    pub fn new() -> SpinSystem{
        let s:Vec<Spin> = Vec::new();
        return SpinSystem{spins:s};
    }

    pub fn apply(&mut self,e:&Event){
        self.spins.iter_mut().for_each(|mut spin|{
            let g = Vector::unit('z')*(e.gradient*spin.r + spin.delta_b);
            let b = g + e.rf;
            spin.step(&b,e.duration);
        });
    }

}