pub const STRIP_LENGTH: usize = 60;
pub const STRIP_NUM: usize = 24;
pub const NUM_LED: usize = STRIP_LENGTH * STRIP_NUM;

/// How long the player waits before automatically advancing to the next show,
/// in microseconds (60 seconds).
pub const AUTO_SHOW_DELAY: u64 = 60_000_000;

pub const SNAKE_PROB: u8 = 32;

/// Minimum time a button must be held to count as a long press, in
/// microseconds (1 second).
pub const LONG_PRESS_TIME: u64 = 1_000_000;

pub const SPARK_PROB: f32 = 1e-2;
pub const SPARKS_PER_STRIP: usize = 8;
