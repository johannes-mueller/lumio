use crate::{
    conf::NUM_LED, conf::STRIP_NUM, interface::Interface,
    led::{Color, DARK_BLUE, DARK_GREEN, WHITE, YELLOW}, ledstrip::LEDStrip,
    player::{Show, StepResult}, random,
};

const NOVA_PROB: u8 = 3;

pub struct Stars {
    sky_color: Color,
    star_color: Color,
    random: random::Random,
}

impl Stars {
    pub fn new(sky_color: Color, star_color: Color) -> Stars {
        Stars {
            sky_color,
            star_color,
            random: random::Random::new(4023749823),
        }
    }

    pub fn fill_sky(&mut self, led_strip: &mut LEDStrip) {
        for i in 0..NUM_LED {
            led_strip.set_led(i as isize, self.sky_color);
        }
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        let nova_prob = NOVA_PROB * STRIP_NUM as u8;
        if self.random.value8() < nova_prob {
            let pos = self.random.value32(NUM_LED as u32) as usize;
            led_strip.set_led(pos as isize, self.star_color);
            led_strip
                .led_mut(pos)
                .set_target_flickering(self.sky_color, 2, 96);
        }
    }
}

impl Show for Stars {
    fn title(&self) -> &'static str {
        if self.sky_color == DARK_GREEN && self.star_color == WHITE {
            "Stars (Esperanto)"
        } else if self.sky_color == DARK_BLUE && self.star_color == YELLOW {
            "Stars (EU)"
        } else {
            "Stars"
        }
    }

    fn reset(&mut self, iface: &mut Interface<'_>) {
        self.fill_sky(&mut iface.led_strip());
    }

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {
        self.process(&mut iface.led_strip());
        iface.write_spi();
        StepResult::Running
    }
}
