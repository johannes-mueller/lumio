use crate::conf::*;
use crate::led::{Led, Color};

const HALF: usize = NUM_LED / 2;
const TAIL: usize = HALF / 11;
const DATA_SIZE: usize = HALF*4+4+TAIL;

pub struct LEDStrip {
    bytes_0: [u8; DATA_SIZE],
    bytes_1: [u8; DATA_SIZE],
    leds: [Led; NUM_LED]
}


impl LEDStrip {
    pub fn new() -> LEDStrip {
        let mut bytes_0: [u8; DATA_SIZE] = [0x00u8; DATA_SIZE];
        let mut bytes_1: [u8; DATA_SIZE] = [0x00u8; DATA_SIZE];
        for i in DATA_SIZE-TAIL..DATA_SIZE {
            bytes_0[i] = 0xff;
            bytes_1[i] = 0xff;
        }
        let leds = [Led::new(); NUM_LED];
        LEDStrip { bytes_0, bytes_1, leds }
    }

    pub fn set_led(&mut self, pos: isize, color: Color) {
        let i = index_from_pos(pos);
        self.leds[i].set_color(color);
    }

    pub fn set_led_target(&mut self, pos: isize, color: Color, decay: f32) {
        let i = index_from_pos(pos);
        self.leds[i].set_target(color, decay);
    }

    pub fn dump_0(&mut self) -> &[u8; DATA_SIZE] {
        for i in 0..HALF {
            let led = &mut self.leds[i];
            self.bytes_0[4+i*4] = 0xff;
            self.bytes_0[4+i*4+1] = led.b();
            self.bytes_0[4+i*4+2] = led.g();
            self.bytes_0[4+i*4+3] = led.r();
            led.step();
        }
        &self.bytes_0
    }

    pub fn dump_1(&mut self) -> &[u8; DATA_SIZE] {
        for i in 0..HALF {
            let led = &mut self.leds[i+HALF];
            self.bytes_1[4+i*4] = 0xff;
            self.bytes_1[4+i*4+1] = led.b();
            self.bytes_1[4+i*4+2] = led.g();
            self.bytes_1[4+i*4+3] = led.r();
            led.step();
        }
        &self.bytes_1
    }
}

fn index_from_pos(pos: isize) -> usize {
    (if pos < 0 { NUM_LED - pos.abs() as usize } else { pos as usize }) % NUM_LED
}
