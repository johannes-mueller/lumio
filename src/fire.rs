use crate::{ledstrip::LEDStrip, conf::NUM_LED, conf::STRIP_LENGTH, conf::STRIP_NUM, random::Random, math8::{qsub8, scale8, qadd8}, led::Color, interface::Interface};

const COOLING: u8 = 8;
const SPARK_PROB: u8 = 10;


pub enum FireColor {
    Red,
    Green
}


pub struct Fire {
    heat: [u8; NUM_LED],
    rng: Random,
    color: FireColor
}



impl Fire {
    pub fn new_red() -> Fire {
        Fire { heat: [0u8; NUM_LED], rng: Random::new(23124923), color: FireColor::Red }
    }
    pub fn new_green() -> Fire {
        Fire { heat: [0u8; NUM_LED], rng: Random::new(23124923), color: FireColor::Green }
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
            let temperature = self.heat[i];
            let color = match self.color {
                FireColor::Red => self.tempeature_to_red_color(temperature),
                FireColor::Green => self.tempeature_to_green_color(temperature)
            };
            led_strip.set_led(i as isize, color);
        }
    }

    pub fn tempeature_to_red_color(&self, temperature: u8) -> Color {
        let t192 = scale8(temperature, 191);
        let heatramp = (t192 & 0x3f) << 2;

        if t192 & 0x80 != 0 {
            return Color { r: 255, g: 255, b: 0 }
        }
        if t192 & 0x40 != 0 {
            return Color { r: 255, g: heatramp, b: 0 }
        }
        Color { r: heatramp, g: 0, b: 0 }
    }

    pub fn tempeature_to_green_color(&self, temperature: u8) -> Color {
        let t192 = scale8(temperature, 191);
        let heatramp = (t192 & 0x3f) << 2;

        if t192 & 0x80 != 0 {
            return Color { r: 255, g: 255, b: 0 }
        }
        if t192 & 0x40 != 0 {
            return Color { r: heatramp, g: 255, b: 0 }
        }
        Color { r: 0, g: heatramp, b: 0 }
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
