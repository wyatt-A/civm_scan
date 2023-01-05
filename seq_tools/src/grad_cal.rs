use std::f32::consts::PI;

// these values must match the parfilio file values to output correct gradient strengths
pub const GRAD_MAX_READ:u32 = 101857;
pub const GRAD_MAX_PHASE:u32 = 92456;
pub const GRAD_MAX_SLICE:u32 = 112634;
pub const GAMMA_BAR:f32 = 42.58e6;
pub const GAMMA:f32 = 2.0*PI*GAMMA_BAR;
pub const GRAD_MIN:u32 = GRAD_MAX_PHASE;

pub fn grad_to_dac(grad_hz_per_mm:f32) -> i16 {
    let fraction = grad_hz_per_mm/GRAD_MIN as f32;
    if fraction >= 1.0 {panic!("max gradient strength exceeded. {} hz/mm > {} hz/mm",grad_hz_per_mm,GRAD_MIN)}
    let dac = i16::MAX as f32 * fraction;
    dac as i16
}

// dac -> Hz/mm
pub fn dac_to_grad(grad_dac:i16) -> f32 {
    let fraction = grad_dac as f32 / i16::MAX as f32;
    GRAD_MIN as f32 * fraction
}

pub fn dac_to_hz_per_meter(grad_dac:i16) -> f32 {
    (dac_to_grad(grad_dac))*1000.0
}

pub fn dac_to_tesla_per_meter(grad_dac:i16) -> f32 {
    dac_to_hz_per_meter(grad_dac)/ GAMMA_BAR
}

pub fn tesla_per_mm_to_dac(grad_tpmm:f32) -> i16 {
    grad_to_dac(GAMMA_BAR*grad_tpmm)
}