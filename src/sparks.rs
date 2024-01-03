
use crate::{ledstrip::LEDStrip, conf::*, led::{Color, self}, interface::Interface};
use crate::button::ButtonState;

const ISTRIP_LENGTH: isize = STRIP_LENGTH as isize;
const SPARK_NUM: usize = STRIP_NUM * SPARKS_PER_STRIP;

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

    pub fn deactivate(&mut self) {
        self.engine.deactivate();
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

    pub fn deactivate(&mut self) {
        self.engine.deactivate();
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
    color: Color,
    initial_speed: isize
}

impl FallingSparks {
    pub fn new(strip: usize, color: Color) -> FallingSparks {
        FallingSparks {
            engine: SparkEngine::new(strip),
            color,
            initial_speed: 0
        }
    }

    pub fn initial_speed(&self) -> isize {
        self.initial_speed
    }

    pub fn is_active(&self) -> bool {
        self.engine.is_active()
    }

    pub fn deactivate(&mut self) {
        self.engine.deactivate();
    }

    pub fn reset(&mut self, initial_speed: isize, initial_position: isize) {
        self.initial_speed = initial_speed;
        self.engine.reset(initial_speed, initial_position << 6);
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

    fn deactivate(&mut self) {
        self.current_position = -1;
    }

    fn going_down(&self) -> bool {
        self.current_speed < 0
    }

    fn process(&mut self, led_strip: &mut LEDStrip, color: Color) {
        let start_pos = ISTRIP_LENGTH*self.strip;
        if !self.is_active() {
            return;
        }

        let next_position = self.current_position + self.current_speed;

        let pos = self.current_position >> 6;
        if pos < ISTRIP_LENGTH {
            led_strip.set_led(start_pos + pos, color);
        }

        self.current_position = next_position;
        self.current_speed = ((self.current_speed << 2 ) - ACCEL) >> 2 ;
    }
}


pub struct FireWorks {
    mono_sparks: [MonoSpark; SPARK_NUM],
    color_sparks: [ColorSpark; STRIP_NUM]

}

impl FireWorks {
    pub fn new() -> FireWorks {
        FireWorks {
            mono_sparks: core::array::from_fn(|i| i+1).map(|sn| MonoSpark::new(sn / SPARKS_PER_STRIP)),
            color_sparks: core::array::from_fn(|i| i+1).map(|strip| ColorSpark::new(strip))
        }
    }

    pub fn show(&mut self, interface: &mut Interface) {
        loop {
            interface.led_strip().black();
            for sp in self.mono_sparks.iter_mut() {
                sp.process(&mut interface.led_strip());
            }
            for sp in self.color_sparks.iter_mut() {
                sp.process(&mut interface.led_strip());
            }
            interface.write_spi();
            if !self.mono_sparks.iter().any(|sp| sp.is_active()) {
                if interface.random().value() < SPARK_PROB || interface.button_state() == ButtonState::ShortPressed {
                    let _ = interface.led_on();
                    let hue = interface.random().value();
                    for sp in self.mono_sparks.iter_mut() {
                        let speed = interface.random().value8() as isize;
                        let decay = interface.random().value8() >> 2;
                        let brightness = 127 + interface.random().value8() % 128;
                        sp.reset(hue, speed, decay, brightness);
                    };
                    for strip in 0..STRIP_NUM {
                        let start = strip * SPARKS_PER_STRIP;
                        let end = start + SPARKS_PER_STRIP;
                        let speed = (start..end).map(|i| self.mono_sparks[i].speed()).max().unwrap();
                        let decay = 0.1;
                        self.color_sparks[strip].reset(hue + 60./360. % 1.0, decay, speed);
                    };
                } else {
                    let _ = interface.led_off();
                }
            }
            if interface.do_next() {
                interface.led_strip().black();
                let _ = interface.led_off();
                break;
            }
        }
        for ms in self.mono_sparks.iter_mut() {
            ms.deactivate();
        }
        for cs in self.color_sparks.iter_mut() {
            cs.deactivate();
        }
    }
}


const SNOW_SPARKS_PER_STRIP: usize = 2;
const SNOW_SPARK_NUM: usize = SNOW_SPARKS_PER_STRIP * STRIP_NUM;


pub struct SnowSparks {
    sparks: [FallingSparks; SNOW_SPARK_NUM]
}

impl SnowSparks {
    pub fn new() -> SnowSparks {
        SnowSparks {
            sparks: core::array::from_fn(|i| i+1)
                .map(|sn| FallingSparks::new(sn / SNOW_SPARKS_PER_STRIP, Color {r: 32, g: 32, b: 32}))
        }
    }

    pub fn show(&mut self, interface: &mut Interface) {
        loop {
            interface.led_strip().black();
            for fs in self.sparks.iter_mut() {
                let was_active = fs.is_active();
                fs.process(&mut interface.led_strip());
                if !fs.is_active() {
                    if was_active {
                        let speed = match fs.initial_speed() {
                            0 => 192,
                            s => (s * 4) / 5
                        };
                        if speed > 1 {
                            fs.reset(speed, 0);
                        }
                    } else if interface.random().value() < SPARK_PROB * 0.7 {
                        fs.reset(0, STRIP_LENGTH as isize - 1);
                    }
                }
            }
            interface.write_spi();
            if interface.do_next() {
                interface.led_strip().black();
                let _ = interface.led_off();
                break;
            }
        }
        for s in self.sparks.iter_mut() {
            s.deactivate();
        }
    }
}


pub struct SparkFall {
    sparks: [FallingSparks; SPARK_NUM]
}

impl SparkFall {
    pub fn new() -> SparkFall {
        SparkFall {
            sparks: core::array::from_fn(|i| i+1)
                .map(|sn| FallingSparks::new(sn / SPARKS_PER_STRIP, Color {r: 32, g: 0, b: 32}))

        }
    }

    pub fn show(&mut self, interface: &mut Interface) {
        loop {
            interface.led_strip().black();
            for fs in self.sparks.iter_mut() {
                if !fs.is_active() && interface.random().value() < SPARK_PROB {
                    fs.reset(0, STRIP_LENGTH as isize - 1);
                }
                fs.process(&mut interface.led_strip());
            }
            interface.delay_ms(10);
            interface.write_spi();
            if interface.do_next() {
                interface.led_strip().black();
                let _ = interface.led_off();
                break;
            }
        }
        for s in self.sparks.iter_mut() {
            s.deactivate();
        }
    }
}
