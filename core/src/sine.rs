use crate::{
    conf::*,
    interface::Interface,
    led::{Color, WHITE},
    player::{Show, StepResult},
};

pub struct Sine {
    center: isize,
    current: isize,
    elastic: isize,
    speed: isize,
}

pub fn scale(a: isize, b: isize) -> isize {
    (a * b) >> 8
}

impl Sine {
    pub fn new(center: isize, elastic: isize, ampl: isize) -> Sine {
        Sine {
            center,
            elastic,
            current: (ampl + center) << 6,
            speed: 0,
        }
    }

    fn set_elastic(&mut self, v: isize) -> &mut Sine {
        self.elastic = v;
        self
    }

    fn process(&mut self) -> isize {
        let pos: isize = self.current >> 6;

        let accel = scale(self.elastic, pos - self.center) << 2;
        self.speed = ((self.speed << 2) - accel) >> 2;

        self.current = self.current + self.speed;

        pos
    }
}

pub struct SineShow {
    sine: Sine,
    hue: f32,
}

impl SineShow {
    pub fn new() -> SineShow {
        SineShow {
            sine: Sine::new(30, 502, 28),
            hue: 0.0,
        }
    }

    fn sort(&self, p1: isize, p2: isize, p3: isize) -> (isize, isize, isize) {
        if p1 < p2 {
            if p2 < p3 {
                return (p1, p2, p3);
            } else if p1 < p3 {
                return (p1, p3, p2);
            } else {
                return (p3, p1, p2);
            }
        } else {
            if p1 < p3 {
                return (p2, p1, p3);
            } else if p2 < p3 {
                return (p2, p3, p1);
            } else {
                return (p3, p2, p1);
            }
        }
    }
}

impl Show for SineShow {
    fn reset(&mut self, _iface: &mut Interface<'_>) {
        self.sine = Sine::new(30, 502, 28);
        self.hue = 0.0;
    }

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {
        let hue_step = STRIP_NUM as f32 / 360.0;

        iface.led_strip().black();

        let wave_pos: [isize; 3 * STRIP_NUM] = core::array::from_fn(|_i| self.sine.process());

        for i in 0..STRIP_NUM {
            let strip_begin = (i % STRIP_NUM * STRIP_LENGTH) as isize;

            let pos_1 = wave_pos[i];
            let pos_2 = wave_pos[i + STRIP_NUM];
            let pos_3 = wave_pos[i + 2 * STRIP_NUM];

            iface.led_strip().set_led(strip_begin + pos_1, WHITE);
            iface.led_strip().set_led(strip_begin + pos_2, WHITE);
            iface.led_strip().set_led(strip_begin + pos_3, WHITE);

            let (pos_1, pos_2, pos_3) = self.sort(pos_1, pos_2, pos_3);

            self.hue += 15.0 / 360.0;

            let color = Color::from_hsv(self.hue, 1.0, 0.25);

            for p in pos_1 + 1..pos_2 {
                iface.led_strip().set_led(strip_begin + p, color);
            }

            let color = Color::from_hsv(self.hue + 0.5, 1.0, 0.25);

            for p in pos_2 + 1..pos_3 {
                iface.led_strip().set_led(strip_begin + p, color);
            }
        }

        self.hue += hue_step / 20.0;

        iface.write_spi();
        StepResult::Running
    }
}

enum Elastic {
    Constant(isize),
    Varying(Sine),
}

pub struct SeaWave {
    hue: f32,
    hue_spark: f32,
    elastic: Elastic,
    ampl: isize,
    sine: Sine,
}

impl SeaWave {
    pub fn new(
        hue: f32,
        hue_spark: f32,
        elastic: Option<isize>,
        ampl: isize
    ) -> SeaWave {
        let elastic = match elastic {
            Some(v) => Elastic::Constant(v),
            None => Elastic::Varying(Sine::new(1100, 12, 200)),
        };
        SeaWave {
            hue,
            hue_spark,
            elastic,
            ampl,
            sine: Sine::new(40, 0, ampl),
        }
    }

    fn elastic(&mut self) -> isize {
        match &mut self.elastic {
            Elastic::Constant(v) => *v,
            Elastic::Varying(sine) => sine.process(),
        }
    }
}

impl Show for SeaWave {
    fn reset(&mut self, _iface: &mut Interface<'_>) {
        self.sine = Sine::new(40, 0, self.ampl);
    }

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {
        iface.led_strip().black();

        let elastic = self.elastic();
        self.sine.set_elastic(elastic);

        for i in 0..STRIP_NUM {
            let strip_begin = (i % STRIP_NUM * STRIP_LENGTH) as isize;

            let pos = self.sine.process();
            for p in 0..pos {
                let hue = if iface.random().value8() < 32 {
                    random_hue_around_given(self.hue_spark, iface)
                } else {
                    self.hue
                };
                iface
                    .led_strip()
                    .set_led(strip_begin + p, Color::from_hsv(hue, 1.0, 0.25));
            }
        }

        iface.write_spi();
        StepResult::Running
    }
}

fn random_hue_around_given(center_hue: f32, interface: &mut Interface<'_>) -> f32 {
    (center_hue - interface.random().value() / 6.0) % 1.0
}
