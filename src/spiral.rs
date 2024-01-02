use crate::{ledstrip::LEDStrip, conf::{STRIP_LENGTH, STRIP_NUM}, led::WHITE};

pub struct Spiral {
    start_strip: usize,
    steps: usize
}

impl Spiral {
    pub fn new(start_strip: usize) -> Spiral {
        Spiral { start_strip, steps: 0 }
    }

    pub fn wandering() -> Spiral {
        Spiral { start_strip: 0, steps: STRIP_LENGTH }
    }

    pub fn swirl(&mut self) {
        self.steps = (self.steps + 1) % (STRIP_LENGTH * 2);
    }

    pub fn step(&mut self) {
        self.start_strip = (self.start_strip + STRIP_NUM - 1) % STRIP_NUM
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        let start = (self.steps / STRIP_LENGTH) * (self.steps % STRIP_LENGTH);
        let mut strip = (self.start_strip + start) % STRIP_NUM;
        let target = self.steps.min(STRIP_LENGTH);
        for i in start..target {
            let pos = (strip*STRIP_LENGTH + i) as isize;
            led_strip.set_led(pos, WHITE);
            strip = (strip+1) % STRIP_NUM;
        }
    }
}
