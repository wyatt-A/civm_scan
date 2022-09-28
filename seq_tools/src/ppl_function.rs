pub const MIN_DELAY_CLOCKS:i32 = 66;

pub fn wait_timer(clock_cycles:i32) -> String {
    if clock_cycles > u16::MAX as i32 {panic!("waittimer call exceeds max value {} > {}",clock_cycles,u16::max as i32);}
    if clock_cycles < 30 {panic!("waittimer call doesn't meet minimum value {} < {}",clock_cycles,30);}
    format!("waittimer({});",clock_cycles)
}

pub fn rf_start(frame_uid:u8,pulse_duration_us:u16,frame_label:&str,predelay_us:u16) -> String {
    format!("MR3031_RFSTART({},{},{},{},{})",frame_uid,pulse_duration_us,frame_label,predelay_us,4)
}

pub fn grad_start(channel_mask:&str) -> String {
    format!("MR3040_Start({});",channel_mask)
}

pub fn set_phase_with_var(varname:&str) -> String {
    format!("phase({});",varname)
}

pub fn set_phase_with_val(phase:u16) -> String {
    format!("phase({});",phase)
}

pub fn set_rec_phase_with_val(phase:u16) -> String {
    format!("rphase({});",phase)
}

pub fn set_rec_phase_with_var(varname:&str) -> String {
    format!("rphase({});",varname)
}

pub fn start_timer() -> String {
    "starttimer();".to_owned()
}

pub fn resync() -> String {
    "resync();".to_owned()
}

pub fn delay_with_var(delay_var:&str) -> String {
    format!("delay32({})",delay_var)
}

pub fn delay(clocks:i32) -> String {
    if clocks < MIN_DELAY_CLOCKS {panic!("delay must be at least {}. Received {}",MIN_DELAY_CLOCKS,clocks)}
    // account for 2 clock cycles that aren't needed if the argument is a literal value
    format!("delay32({}L);",clocks-2)
}

pub fn init_list_var(list_label:&str) -> String {
    format!("{} = MR3040_InitList();",list_label)
}

pub fn init_list(seqfile_label:&str,list_label:&str) -> String {
    let arg1 = format!("{}.address.\"{}\"",seqfile_label,list_label);
    let arg2 = format!("{}.size.\"{}\"",seqfile_label,list_label);
    let arg3 = format!("{}.waits.\"{}\"",seqfile_label,list_label);
    format!("MR3040_Output(NOLOOP,{},{},{});",arg1,arg2,arg3)
}

pub fn acquire(no_samples_var:&str,sample_period_var:&str) -> String {
    format!("acquire({},{});",sample_period_var,no_samples_var)
}

pub fn host_request() -> String {
    String::from("hostrequest();")
}

pub fn system_out() -> String {
    String::from("systemout(pts_mask);")
}

pub fn base_matrix(orientation:(i16,i16,i16)) -> String {
    format!("BASEMATRIX_LONG1({},{},{})",orientation.0,orientation.1,orientation.2)
}

pub fn delay_us(time_us:i16) -> String {
    format!("delay( {}, us);",time_us)
}

pub fn grad_deglitch() -> String {
    String::from("MR3040_DEGLITCH")
}

pub fn grad_clock(clocks_per_sample:i16) -> String {
    if clocks_per_sample < 20 {panic!("grad clock setting must be atleast 20 clocks per sample")}
    format!("MR3040_Clock({});",clocks_per_sample)
}

pub fn set_base_freq() -> String {
    vec![
        String::from("frequency_buffer(0);"),
        String::from("frequency(MHz, kHz, Hz, rx1MHz);"),
        String::from("reset_frequency();")
    ].join("\n")
}

pub fn set_discard_samples(discard_var:&str) -> String {
    format!("discard({});",discard_var)
}