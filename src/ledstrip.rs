use crate::conf::*;
use crate::led::{Led, Color, BLACK};
use crate::random::Random;

const HALF: usize = NUM_LED / 2;
const DATA_SIZE: usize = NUM_LED*4+8;
const HALF_BYTES: usize = DATA_SIZE / 2;
const LED_DATA_SIZE: usize = 4;
const SPI_OFFSET: usize = 4;
const RED_OFFSET: usize = 3;
const GREEN_OFFSET: usize = 2;
const BLUE_OFFSET: usize = 1;

pub struct LEDStrip {
    bytes: [u8; DATA_SIZE],
    leds: [Led; NUM_LED],
    random: Random
}


impl LEDStrip {
    pub fn new() -> LEDStrip {
        let bytes: [u8; DATA_SIZE] = [0x00u8; DATA_SIZE];
        let leds = [Led::new(); NUM_LED];
        LEDStrip { bytes, leds, random: Random::new(423234098) }
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

    pub fn process(&mut self) {
        self.process_half_from(0);
        self.process_half_from(HALF);
    }

    fn process_half_from(&mut self, start_led: usize) {
        let start_byte = start_led * LED_DATA_SIZE + SPI_OFFSET;
        for i in 0..HALF {
            let led = &mut self.leds[i+start_led];
            let offset = start_byte + i * LED_DATA_SIZE;
            self.bytes[offset] = 0xff;
            self.bytes[offset+BLUE_OFFSET] = led.b();
            self.bytes[offset+GREEN_OFFSET] = led.g();
            self.bytes[offset+RED_OFFSET] = led.r();
            led.step(&mut self.random);
        }
    }

    pub fn dump_0(&mut self) -> &[u8] {
        &self.bytes[..HALF_BYTES]
    }

    pub fn dump_1(&mut self) -> &[u8] {
        &self.bytes[HALF_BYTES..]
    }

    pub fn black(&mut self) {
        for led in self.leds.iter_mut() {
            led.set_color(BLACK);
        }
    }
}

fn index_from_pos(pos: isize) -> usize {
    (if pos < 0 { NUM_LED - pos.abs() as usize } else { pos as usize }) % NUM_LED
}
