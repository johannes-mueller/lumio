use rp_pico::hal::rom_data::float_funcs::float_to_uint;

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

    pub fn n_out_of_m<const M: usize, const N: usize>(&mut self) -> [usize; N] {
        let mut values = [M+1; N];
        for i in 0..N {
            let mut cand = M+1;
            while values[0..i].iter().any(|&x| x == cand) {
                cand = float_to_uint(self.value() * M as f32) as usize;
            }
            values[i] = cand;
        }
        values
    }
}
