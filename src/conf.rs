use fugit::MicrosDurationU64;

pub const NUM_LED: usize = 1440;
pub const STRIP_LENGTH: usize = 60;
pub const AUTO_SHOW_DELAY: MicrosDurationU64 = MicrosDurationU64::secs(60);
pub const SNAKE_PROB: u8 = 32;
pub const LONG_PRESS_TIME: MicrosDurationU64 = MicrosDurationU64::millis(1_000);
pub const SPARK_PROB: u8 = 4;
