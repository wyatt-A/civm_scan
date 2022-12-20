pub const GRAD_TIME_BLOCK_1:i32 = 300;
pub const GRAD_TIME_BLOCK_2:i32 = 300;

//const SINGLE_CHANNEL_START_DELAY:i32 = 100; // cost of 1 channel start
pub const GRAD_SINGLE_CHANNEL_START_DELAY:i32 = 0; // cost of 1 channel start
pub const GRAD_EXTRA_CHANNEL_START_DELAY:i32 = 30; // added cost per 1 channel more (81 + 31 for 2 channels)

//const TIME_BLOCK:i32 = 150; // clock cycles (100ns)
pub const RF_TIME_BLOCK:i32 = 400; // clock cycles (100ns)
pub const RFSTART_PREDELAY:i32 = 600; // clock cycles
pub const RFSTART_POSTDELAY:i32 = 84; // clock cycles (found experimentally)

// time allocated for setting reciever phase
pub const ACQ_TIME_BLOCK_1:i32 = 500;
// time required after call to acquire before samples are collected
//const TIME_BLOCK_2:i32 = 221;
//const TIME_BLOCK_2:i32 = 600;
pub const ACQ_TIME_BLOCK_2:i32 = 500;