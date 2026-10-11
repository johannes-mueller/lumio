use crate::conf::*;
use crate::led::{Color, Led, BLACK};
use crate::random::Random;

const HALF: usize = NUM_LED / 2;
const TAIL: usize = HALF / 11;
pub const DATA_SIZE: usize = HALF * 4 + 4 + TAIL;

/// The full 24×60 LED framebuffer, plus the APA102 byte streams for the two
/// SPI buses (the first [`HALF`] LEDs go to bus 0, the rest to bus 1).
pub struct LEDStrip {
    bytes_0: [u8; DATA_SIZE],
    bytes_1: [u8; DATA_SIZE],
    leds: [Led; NUM_LED],
    random: Random,
}

impl LEDStrip {
    pub fn new() -> LEDStrip {
        let mut bytes_0: [u8; DATA_SIZE] = [0x00u8; DATA_SIZE];
        let mut bytes_1: [u8; DATA_SIZE] = [0x00u8; DATA_SIZE];
        for i in DATA_SIZE - TAIL..DATA_SIZE {
            bytes_0[i] = 0xff;
            bytes_1[i] = 0xff;
        }
        let leds = [Led::new(); NUM_LED];
        LEDStrip {
            bytes_0,
            bytes_1,
            leds,
            random: Random::new(423234098),
        }
    }

    pub fn set_led(&mut self, pos: isize, color: Color) {
        let i = index_from_pos(pos);
        self.leds[i].set_color(color);
    }

    pub fn set_led_target(&mut self, pos: isize, color: Color, decay: u8) {
        let i = index_from_pos(pos);
        self.leds[i].set_target(color, decay);
    }

    pub fn led(&self, pos: usize) -> &Led {
        &self.leds[pos]
    }

    pub fn led_mut(&mut self, pos: usize) -> &mut Led {
        &mut self.leds[pos]
    }

    /// Update both APA102 byte streams from the LED colors and advance every
    /// LED by one animation step.
    pub fn refresh(&mut self) {
        for i in 0..HALF {
            let led = &mut self.leds[i];
            self.bytes_0[4 + i * 4] = 0xff;
            self.bytes_0[4 + i * 4 + 1] = led.b();
            self.bytes_0[4 + i * 4 + 2] = led.g();
            self.bytes_0[4 + i * 4 + 3] = led.r();
            led.step(&mut self.random);
        }
        for i in 0..HALF {
            let led = &mut self.leds[i + HALF];
            self.bytes_1[4 + i * 4] = 0xff;
            self.bytes_1[4 + i * 4 + 1] = led.b();
            self.bytes_1[4 + i * 4 + 2] = led.g();
            self.bytes_1[4 + i * 4 + 3] = led.r();
            led.step(&mut self.random);
        }
    }

    /// The two APA102 byte streams, for bus 0 and bus 1 respectively.
    pub fn bytes(&self) -> (&[u8; DATA_SIZE], &[u8; DATA_SIZE]) {
        (&self.bytes_0, &self.bytes_1)
    }

    pub fn black(&mut self) {
        for led in self.leds.iter_mut() {
            led.set_color(BLACK);
        }
    }
}

fn index_from_pos(pos: isize) -> usize {
    (if pos < 0 {
        NUM_LED - pos.abs() as usize
    } else {
        pos as usize
    }) % NUM_LED
}
