use crate::{ledstrip::LEDStrip, conf::STRIP_LENGTH, led::Color};

const HUE_STEP: f32 = 1.0 / STRIP_LENGTH as f32;

pub struct HueWave {
    step: usize
}

impl HueWave {
    pub fn new() -> HueWave {
        HueWave { step: 0 }
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        let mut hue = self.step as f32 / STRIP_LENGTH as f32;
        for y in 0..STRIP_LENGTH {
            for x in 0..12 {
                let pos = (x * STRIP_LENGTH + y) as isize;
                led_strip.set_led(pos, Color::from_hsv(hue, 1.0, 0.03));
            }
            hue = (hue + HUE_STEP) % 1.0;
        }
        self.step = (self.step + 1) % STRIP_LENGTH;
    }

}
