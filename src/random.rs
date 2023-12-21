use rand::{RngCore, SeedableRng};
use rand::rngs::SmallRng;

pub struct Random {
    rng: SmallRng
}

impl Random {
    pub fn new(seed: u64) -> Random {
        Random { rng: SmallRng::seed_from_u64(seed) }
    }

    pub fn value(&mut self) -> f32 {
        self.rng.next_u32() as f32 / 0xffffffffu32 as f32
    }

    pub fn value8(&mut self) -> u8 {
        self.rng.next_u32() as u8
    }

    pub fn value32(&mut self, upper_limit: u32) -> u32 {
        if upper_limit == 0 {
            return 0u32;
        }
        self.rng.next_u32() % upper_limit
    }
}
