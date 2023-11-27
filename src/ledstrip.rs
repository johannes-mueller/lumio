use crate::conf::*;
use crate::led::{Led, Color, BLACK};

pub struct LEDStrip {
    bytes: [u8; DATA_SIZE],
    leds: [Led; NUM_LED]
}


impl LEDStrip {
    pub fn new() -> LEDStrip {
        let mut bytes: [u8; DATA_SIZE] = [0x00u8; DATA_SIZE];
        for i in DATA_SIZE-TAIL..DATA_SIZE {
            bytes[i] = 0xff;
        }
        let leds = [Led::new(); NUM_LED];
        LEDStrip { bytes, leds }
    }

    pub fn set_led(&mut self, pos: isize, color: Color) {
        let i: usize = if pos < 0 {
            (NUM_LED - pos.abs() as usize) % NUM_LED
        } else {
            pos as usize % NUM_LED
        };
        self.leds[i].set_color(color);
    }

    pub fn dump(&mut self) -> &[u8; DATA_SIZE] {
        for i in 0..NUM_LED {
            self.bytes[4+i*4] = 0xff;
            let c = self.leds[i].current_color();
            self.bytes[4+i*4+1] = c.b;
            self.bytes[4+i*4+2] = c.g;
            self.bytes[4+i*4+3] = c.r;
        }
        &self.bytes
    }
}
