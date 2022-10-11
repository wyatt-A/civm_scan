const CYCLES_PER_MICROSECOND:i32 = 10;

pub fn descretize(x:&Vec<f32>,scale:i16) -> Vec<i16> {
    x.iter().map(|val| (scale as f32)*val).map(|val| val as i16).collect()
}

pub fn sec_to_samples(t_seconds:f32,sample_period_us:usize) -> usize{
    let samples = (t_seconds*1.0E6)/(sample_period_us as f32);
    return samples.floor() as usize;
}

pub fn us_to_clock(t_microseconds:i32) -> i32 {
    t_microseconds*CYCLES_PER_MICROSECOND
}

pub fn ms_to_clock(t_milliseconds:i32) -> i32 {
    t_milliseconds*1000*CYCLES_PER_MICROSECOND
}

pub fn sec_to_clock(seconds:f32) -> i32 {
    us_to_clock(sec_to_us(seconds))
}

pub fn sec_to_us(seconds:f32) -> i32 {
    if seconds < 0.0 {panic!("seconds value must be postive")}
    (seconds * 1_000_000.0) as i32
}

pub fn clock_to_sec(clocks:i32) -> f32 {
    us_to_sec(clock_to_us(clocks))
}

pub fn us_to_sec(t_microseconds:i32) -> f32 {
    (t_microseconds as f32)/1_000_000.0
}

pub fn clock_to_us(clocks:i32) -> i32 {
    clocks/CYCLES_PER_MICROSECOND
}

pub fn argsort(data:Vec<i32>) -> Vec<usize> {
    let mut indices = (0..data.len()).collect::<Vec<_>>();
    indices.sort_by_key(|&i| &data[i]);
    indices
}