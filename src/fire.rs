use crate::{ledstrip::LEDStrip, conf::NUM_LED, conf::STRIP_LENGTH, conf::STRIP_NUM, random::Random, math8::{qsub8, scale8, qadd8}, led::Color, interface::Interface};

const COOLING: u8 = 8;
const SPARK_PROB: u8 = 10;

pub struct Fire {
    heat: [u8; NUM_LED],
    rng: Random
}

impl Fire {
    pub fn new() -> Fire {
        Fire { heat: [0u8; NUM_LED], rng: Random::new(23124923) }
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {

        let spark_prob = SPARK_PROB * STRIP_NUM as u8;

        for h in self.heat.iter_mut() {
            *h = qsub8(*h, scale8(self.rng.value8(), COOLING))
        }

        for y in (3..STRIP_LENGTH).rev() {
            for s in 0..STRIP_NUM {
                let i = s*STRIP_LENGTH + y;
                self.heat[i] = ((self.heat[i-1] as u16 + self.heat[i-2] as u16 + self.heat[i-3] as u16) / 3) as u8;
            }
        }

        if self.rng.value8() < spark_prob {
            let stripe = self.rng.value32(STRIP_NUM as u32) as usize;
            let y = self.rng.value32(7) as usize;
            let i = stripe*STRIP_LENGTH + y;
            self.heat[i] = qadd8(self.heat[i], self.rng.value8().max(166));
        }

        for i in 0..NUM_LED {
            led_strip.set_led(i as isize, Color::from_tempeature(self.heat[i]))
        }
    }

    pub fn show(&mut self, interface: &mut Interface) {
        loop {
            self.process(&mut interface.led_strip());
            interface.write_spi();

            if interface.do_next() {
                interface.led_strip().black();
                break;
            }
        }
    }
}
