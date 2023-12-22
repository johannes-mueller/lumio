use crate::{random, ledstrip::LEDStrip, led::Color};

const NUM_LED: usize = 720;
const NOVA_PROB: u8 = 32;

pub struct Stars {
    sky_color: Color,
    star_color: Color,
    random: random::Random
}

impl Stars {
    pub fn new(sky_color: Color, star_color: Color) -> Stars {
        Stars {
            sky_color,
            star_color,
            random: random::Random::new(4023749823)
        }
    }

    pub fn reset(&mut self, led_strip: &mut LEDStrip) {
        for i in 0..NUM_LED {
            led_strip.set_led(i as isize, self.sky_color);
        }
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        if self.random.value8() < NOVA_PROB {
            let pos = self.random.value32(NUM_LED as u32) as usize;
            led_strip.set_led(pos as isize, self.star_color);
            led_strip.led_mut(pos).set_target_flickering(self.sky_color, 2, 96);
        }
    }
}
