use crate::{random, conf::NUM_LED, conf::STRIP_NUM, ledstrip::LEDStrip, led::Color, interface::Interface};

const NOVA_PROB: u8 = 3;

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
        let nova_prob = NOVA_PROB * STRIP_NUM as u8;
        if self.random.value8() < nova_prob {
            let pos = self.random.value32(NUM_LED as u32) as usize;
            led_strip.set_led(pos as isize, self.star_color);
            led_strip.led_mut(pos).set_target_flickering(self.sky_color, 2, 96);
        }
    }

    pub fn show(&mut self, interface: &mut Interface) {
        self.reset(&mut interface.led_strip());
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
