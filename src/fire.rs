use crate::{ledstrip::LEDStrip, random::Random, math8::{qsub8, scale8, qadd8}, led::Color};

const NUM_LED: usize = 720;
const STRIPE_LENGTH: usize = 60;
const STRIPE_NUM: usize = 12;
const COOLING: u8 = 8;
const SPARK_PROB: u8 = 120;

pub struct Fire {
    heat: [u8; NUM_LED],
    rng: Random
}

impl Fire {
    pub fn new() -> Fire {
        Fire { heat: [0u8; NUM_LED], rng: Random::new(23124923) }
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        for h in self.heat.iter_mut() {
            *h = qsub8(*h, scale8(self.rng.value8(), COOLING))
        }

        for y in (3..STRIPE_LENGTH).rev() {
            for s in 0..STRIPE_NUM {
                let i = s*STRIPE_LENGTH + y;
                self.heat[i] = ((self.heat[i-1] as u16 + self.heat[i-2] as u16 + self.heat[i-3] as u16) / 3) as u8;
            }
        }

        if self.rng.value8() < SPARK_PROB {
            let stripe = self.rng.value32(12) as usize;
            let y = self.rng.value32(7) as usize;
            let i = stripe*STRIPE_LENGTH + y;
            self.heat[i] = qadd8(self.heat[i], self.rng.value8().max(166));
        }

        for i in 0..NUM_LED {
            led_strip.set_led(i as isize, Color::from_tempeature(self.heat[i]))
        }
    }
}
