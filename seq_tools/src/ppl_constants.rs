/*
PPL Constants
 */

pub const CIVM_INCLUDE:&str = r"C:\workstation\SequenceTools\CivmSequenceTools_v1.0\civm_var_20_long.PPH";
pub const STD_FN_INCLUDE:&str = r"stdfn_15.pph";
pub const GRAD_FN_INCLUDE:&str = r"m3040_20.pph";
pub const RF_FN_INCLUDE:&str = r"m3031_15.pph";
pub const STD_RF_SEQ:&str = r"c:\smis\seqlib\RFstd.seq";
pub const STD_GRAD_SEQ:&str = r"c:\smis\seqlib\g3040_15.seq";
pub const LUT_INCLUDE:&str = r"C:\smis\include\lututils.pph";

pub const CALC_MATRIX:&str = "c_calc_mat";

pub const SPECTRAL_WIDTH_VAR:&str = "sample_period";
pub const GRAD_STRENGTH_VAR:&str = "grad_var";
pub const RECEIVER_MASK_VAR:&str = "rec_sel";
pub const RECEIVER_MASK_MIN:u32 = 1;
pub const RECEIVER_MASK_MAX:u32 = 65535;

pub const NO_SAMPLES_VAR:&str = "no_samples";
pub const SAMPLE_PERIOD_VAR:&str = "sample_period";
pub const NO_SAMPLES_MIN:u32 = 8;
pub const NO_SAMPLES_MAX:u32 = 65535;

pub const NO_DISCARD_VAR:&str = "no_discard";
pub const NO_DISCARD_MIN:u32 = 0;
pub const NO_DISCARD_MAX:u32 = 64;

pub const NO_ECHOES_VAR:&str = "no_echoes";
pub const NO_ECHOES_MIN:u32 = 1;
pub const NO_ECHOES_MAX:u32 = 64;

pub const NO_VIEWS_VAR:&str = "no_views";
pub const VIEW_LOOP_NAME:&str = "views_loop";
pub const VIEW_LOOP_COUNTER_VAR:&str = "no_completed_views";

pub const NO_VIEWS_MIN:u32 = 1;
pub const NO_VIEWS_MAX:u32 = 500_000;

pub const NO_AVERAGES_VAR:&str = "no_averages";
pub const AVERAGES_LOOP_NAME:&str = "averages_loop";
pub const AVERAGES_LOOP_COUNTER_VAR:&str = "no_completed_averages";
pub const NO_AVERAGES_MIN:u32 = 1;
pub const NO_AVERAGES_MAX:u32 = 65535;

pub const FREQ_OFFSET_MIN:i32 = -40000000;
pub const FREQ_OFFSET_MAX:i32 = 40000000;

pub const LONG_TEMPVAL_VAR_NAME:&str = "tempval_long";
pub const LUT_TEMPVAL_VAR_NAME_1:&str = "lut_tempval_1";
pub const LUT_TEMPVAL_VAR_NAME_2:&str = "lut_tempval_2";
pub const LUT_INDEX_VAR_NAME:&str = "lut_index";


