use crate::{
    conf::*,
    interface::Interface,
    led::{self, Color, BLACK, WHITE},
    ledstrip::LEDStrip,
    math8::scale8
};


pub struct Sine {
    center: isize,
    current: isize,
    elastic: isize,
    speed: isize
}

pub fn abs(v: isize) -> isize {
    if v < 0 {
        -v
    } else {
        v
    }
}

pub fn scale(a: isize, b: isize) -> isize {
    (a * b) >> 8
    // let abs = (abs(a) * abs(b)) >> 8;
    // if (a < 0) ^ (b < 0) {
    //     -abs
    // } else {
    //     abs
    // }
}

impl Sine {
    pub fn new(center: isize, elastic: isize, ampl: isize) -> Sine {
        Sine {
            center,
            elastic,
            current: (ampl + center) << 6,
            speed: 0
        }
    }

    fn set_elastic(&mut self, v: isize) -> &mut Sine {
        self.elastic = v;
        self
    }

    fn process(&mut self) -> isize {
        let pos: isize = self.current >> 6;

        let accel = scale(self.elastic, (pos - self.center)) << 2;
        self.speed = ((self.speed << 2 ) - accel) >> 2;

        self.current = self.current + self.speed;

        pos
    }
}


pub struct SineShow {
    sine: Sine
}


impl SineShow {
    pub fn new() -> SineShow {
        SineShow { sine: Sine::new(30, 502, 28) }
    }

    pub fn show(&mut self, interface: &mut Interface) {
        let mut hue = 0.0f32;
        loop {
            interface.led_strip().black();

            for i in 0..3*STRIP_NUM {
                let strip_begin = (i % STRIP_NUM * STRIP_LENGTH) as isize;
                let pos = self.sine.process();
                interface.led_strip().set_led(strip_begin + pos, WHITE);

                let color = Color::from_hsv(hue, 1.0, 0.25);
                for p in 0..pos {
                    interface.led_strip().set_led(strip_begin + p, color);
                }

                hue += 5.0/360.0;
            }

            interface.write_spi();
            if interface.do_next() {
                interface.led_strip().black();
                let _ = interface.led_off();
                break;
            }
        }
    }
}
