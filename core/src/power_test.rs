use crate::{conf::NUM_LED, interface::Interface, led::WHITE, ledstrip::{self, LEDStrip}};


pub struct PowerTest {
    step: isize
}

impl PowerTest {
    pub fn new() -> PowerTest {
        PowerTest { step: 0 }
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        for pos in 0..self.step {
            led_strip.set_led(pos, WHITE);
        }
        self.step += 1;
    }

    pub fn show(&mut self, interface: &mut Interface) {
        loop {
            self.process(&mut interface.led_strip());
            interface.write_spi();

            interface.delay_ms(50);
        }
    }
}
