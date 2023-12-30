
use crate::{random::Random, ledstrip::LEDStrip, conf, led::{Color, BLACK, self}};

const STRIP_LENGTH: isize = conf::STRIP_LENGTH as isize;

const SPARKS_SKATTER: u8 = 32;
const ACCEL: isize = 24;


#[derive(Clone, Copy)]
pub struct SparkEngine {
    strip: isize,
    current_speed: isize,
    current_position: isize,
}

#[derive(Clone, Copy)]
pub struct MonoSpark {
    engine: SparkEngine,
    hue: f32,
    decay: u8,
    current_brightness: u8
}

impl MonoSpark {
    pub fn new(strip: usize) -> MonoSpark {
        MonoSpark {
            engine: SparkEngine::new(strip),
            hue: 0.0,
            decay: 255,
            current_brightness: 0
        }
    }

    pub fn is_active(&self) -> bool {
        self.engine.is_active()
    }

    pub fn reset(&mut self, hue: f32, initial_speed: isize, decay: u8, initial_brightness: u8) {
        self.hue = hue;
        self.decay = decay;
        self.current_brightness = initial_brightness;
        self.engine.reset(initial_speed);
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        let color = Color::from_hsv(self.hue, 1.0, self.current_brightness as f32 / 255.0);
        self.engine.process(led_strip, color);
        self.current_brightness = led::decay(self.current_brightness, 0, self.decay);
    }
}

impl SparkEngine {
    pub fn new(strip: usize) -> SparkEngine {
        SparkEngine {
            strip: strip as isize,
            current_speed: 0,
            current_position: -1,
        }
    }

    pub fn reset(&mut self, initial_speed: isize) {
        self.current_speed = initial_speed;
        self.current_position = 0;
    }

    pub fn is_active(&self) -> bool {
        self.current_position >= 0
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip, color: Color) {
        let start_pos = STRIP_LENGTH*self.strip;
        if !self.is_active() {
            return;
        }

        let next_position = self.current_position + self.current_speed;

        let pos = self.current_position >> 6;
        if pos < STRIP_LENGTH {
            led_strip.set_led(start_pos + pos, color);
        }

        self.current_position = next_position;
        self.current_speed = ((self.current_speed << 2 ) - ACCEL) >> 2 ;
    }
}
