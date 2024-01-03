use crate::{ledstrip::LEDStrip, conf::{STRIP_LENGTH, STRIP_NUM}, led::WHITE, huewave::HueWave, interface::Interface};

pub struct Spiral {
    start_strip: usize,
    steps: usize
}

impl Spiral {
    pub fn new(start_strip: usize) -> Spiral {
        Spiral { start_strip, steps: 0 }
    }

    pub fn reset(&mut self) {
        self.steps = 0;
    }

    pub fn swirl(&mut self) {
        self.steps = (self.steps + 1) % (STRIP_LENGTH * 2);
    }

    pub fn step(&mut self) {
        self.steps = STRIP_LENGTH;
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


pub struct HueSpiral {
    spiral: Spiral,
    huewave: HueWave
}

impl HueSpiral {
    pub fn new() -> HueSpiral {
        HueSpiral { spiral: Spiral::new(0), huewave: HueWave::new() }
    }

    pub fn show_lift(&mut self, interface: &mut Interface) {
        loop {
            self.huewave.process(&mut interface.led_strip());
            self.spiral.process(&mut interface.led_strip());
            self.spiral.step();

            interface.write_spi();
            if interface.do_next() {
                interface.led_strip().black();
                break;
            }
//            interface.delay_ms(50);
        }
        self.spiral.reset();
    }

    pub fn show_swirl(&mut self, interface: &mut Interface) {
        loop {
            self.huewave.process(&mut interface.led_strip());
            self.spiral.process(&mut interface.led_strip());
            self.spiral.swirl();

            interface.write_spi();
            if interface.do_next() {
                interface.led_strip().black();
                break;
            }
        }
    }
}
