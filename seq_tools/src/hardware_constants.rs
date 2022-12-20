pub const CLOCK_PERIOD_NS:usize = 100;
pub const GRAD_CLOCK_MULTIPLIER:usize = 20; // this means that the gradient clock period is 2 us
pub const RF_CLOCK_MULTIPLIER:usize = 1; // this means that the min rf clock period is 100 ns
pub const GRAD_SEQ_FILE_LABEL:&str = "civm_grad";
pub const RF_SEQ_FILE_LABEL:&str = "civm_rf";
pub const GRAD_MAX_DAC:i16 = 32767;
pub const RF_MAX_DAC:i16 = 2047;