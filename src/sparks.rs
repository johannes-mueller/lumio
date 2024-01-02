
use crate::{random::Random, ledstrip::LEDStrip, conf, led::{Color, BLACK, self, DARK_GREEN}};

const STRIP_LENGTH: isize = conf::STRIP_LENGTH as isize;

const SPARKS_SKATTER: u8 = 32;
const ACCEL: isize = 24;


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

    pub fn speed(&self) -> isize {
        self.engine.current_speed
    }

    pub fn is_active(&self) -> bool {
        self.engine.is_active()
    }

    pub fn reset(&mut self, hue: f32, initial_speed: isize, decay: u8, initial_brightness: u8) {
        self.hue = hue;
        self.decay = decay;
        self.current_brightness = initial_brightness;
        self.engine.reset(initial_speed, 0);
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        let color = Color::from_hsv(self.hue, 1.0, self.current_brightness as f32 / 255.0);
        self.engine.process(led_strip, color);
        if self.engine.going_down() {
            self.current_brightness = led::decay(self.current_brightness, 0, self.decay);
        }
    }
}

#[derive(Clone, Copy)]
pub struct ColorSpark {
    engine: SparkEngine,
    hue: f32,
    decay: f32,
    current_sat: f32
}

impl ColorSpark {
    pub fn new(strip: usize) -> ColorSpark {
        ColorSpark {
            engine: SparkEngine::new(strip),
            hue: 0.0,
            decay: 0.0,
            current_sat: 0.0
        }
    }

    pub fn is_active(&self) -> bool {
        self.engine.is_active()
    }

    pub fn reset(&mut self, hue: f32, decay: f32, initial_speed: isize) {
        self.hue = hue;
        self.decay = decay;
        self.current_sat = 0.0;
        self.engine.reset(initial_speed, 0);
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        let color = Color::from_hsv(self.hue, self.current_sat, 1.0);
        self.engine.process(led_strip, color);
        if self.engine.going_down() {
            self.current_sat += (1.0 - self.current_sat) * self.decay;
        }
    }
}

pub struct FallingSparks {
    engine: SparkEngine,
    color: Color
}

impl FallingSparks {
    pub fn new(strip: usize, color: Color) -> FallingSparks {
        FallingSparks {
            engine: SparkEngine::new(strip),
            color
        }
    }

    pub fn is_active(&self) -> bool {
        self.engine.is_active()
    }

    pub fn reset(&mut self) {
        self.engine.reset(0, 59 << 6);
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        self.engine.process(led_strip, self.color);
    }
}

#[derive(Clone, Copy)]
struct SparkEngine {
    strip: isize,
    current_speed: isize,
    current_position: isize,
}

impl SparkEngine {
    fn new(strip: usize) -> SparkEngine {
        SparkEngine {
            strip: strip as isize,
            current_speed: 0,
            current_position: -1,
        }
    }

    fn reset(&mut self, initial_speed: isize, initial_position: isize) {
        self.current_speed = initial_speed;
        self.current_position = initial_position;
    }

    fn is_active(&self) -> bool {
        self.current_position >= 0
    }

    fn going_down(&self) -> bool {
        self.current_speed < 0
    }

    fn process(&mut self, led_strip: &mut LEDStrip, color: Color) {
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
