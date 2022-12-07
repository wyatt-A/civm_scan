
pub const GAMMA:Unit = Unit::HzPerTesla(42000000.0);

pub enum Unit {
    JoulePerTesla(f32),
    GaussPerCm(f32),
    Gauss(f32),
    Tesla(f32),
    HzPerMm(f32),
    GradDac(i16),
    RfDac(i16),
    Cm(f32),
    Mm(f32),
    M(f32),
    Sec(f32),
    Millis(f32),
    Micros(f32),
    HzPerTesla(f32),
}

