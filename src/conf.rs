use fugit::MicrosDurationU64;

pub const NUM_LED: usize = 1440;
pub const STRIP_LENGTH: usize = 60;
pub const STRIP_NUM: usize = 12;
pub const AUTO_SHOW_DELAY: MicrosDurationU64 = MicrosDurationU64::secs(60);
pub const SNAKE_PROB: u8 = 32;
pub const LONG_PRESS_TIME: MicrosDurationU64 = MicrosDurationU64::millis(1_000);
pub const SPARK_PROB: f32 = 1e-2;
pub const SPARKS_PER_STRIP: usize = 8;
