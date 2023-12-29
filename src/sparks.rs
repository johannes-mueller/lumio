
use crate::{random::Random, ledstrip::LEDStrip, conf, led::{Color, BLACK, self}};

const STRIP_LENGTH: isize = conf::STRIP_LENGTH as isize;

const SPARKS_SKATTER: u8 = 32;
const ACCEL: isize = 24;


#[derive(Clone, Copy)]
pub struct Spark {
    strip: isize,
    hue: f32,
    current_speed: isize,
    current_position: isize,
    decay: u8,
    current_brightness: u8,
    exploding: bool
}

impl Spark {
    pub fn new(strip: usize) -> Spark {
        Spark {
            strip: strip as isize,
            hue: 1.0,
            decay: 0,
            current_speed: 0,
            current_position: -1,
            current_brightness: 0,
            exploding: false
        }
    }

    pub fn reset(&mut self, hue: f32, initial_speed: isize, decay: u8, initial_brightness: u8) {
        self.hue = hue;
        self.decay = decay;
        self.current_speed = initial_speed;
        self.current_position = 0;
        self.current_brightness = initial_brightness;
    }

    pub fn is_active(&self) -> bool {
        self.current_position >= 0
    }

    pub fn is_exploding(&self) -> bool {
        self.exploding
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        let start_pos = STRIP_LENGTH*self.strip;
        if !self.is_active() {
            return;
        }

        let next_position = self.current_position + self.current_speed;

        let pos = self.current_position >> 6;
        if pos < STRIP_LENGTH {
            let color = Color::from_hsv(self.hue, 1.0, self.current_brightness as f32 / 255.0);
            led_strip.set_led(start_pos + pos, color);
        }

        self.current_brightness = led::decay(self.current_brightness, 0, self.decay);

        self.exploding = next_position * self.current_position < 0;
        self.current_position = next_position;
        self.current_speed = ((self.current_speed << 2 ) - ACCEL) >> 2 ;
    }
}
