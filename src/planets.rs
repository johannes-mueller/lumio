use core::f32::consts::PI;

use libm::{fabsf, sqrtf};

use crate::{
    conf::*,
    interface::{Interface},
    led::{Color, BLACK, YELLOW},
};

const DELTA_T: f32 = 10.0;

pub struct Planet {
    p_rad: f32,
    p_phi: f32,

    d_rad: f32,
    d_phi: f32,

    pub color: Color
}


impl Planet {
    pub fn new(p_rad: f32, p_phi: f32, d_rad: f32, d_phi: f32, color: Color) -> Planet {
        Planet{p_rad, p_phi, d_rad, d_phi, color}
    }

    pub fn new_vis_viva(rad: f32, a: f32, phi: f32, direction: f32, color: Color) -> Planet {
        let v = sqrtf(fabsf((2.0/rad) - (1.0/(rad*a))));
        Planet::new(rad, phi, 0.0, direction * v/rad, color)

    }

    fn step_rad(&mut self, n: usize) {
        let delta_t = DELTA_T / n as f32;
        for _ in 0..n {
            let dd_rad = self.p_rad * self.d_phi * self.d_phi - 1.0 / (self.p_rad * self.p_rad);
            self.d_rad += dd_rad * delta_t;
            self.p_rad += self.d_rad * delta_t;
        }
    }

    fn step_phi(&mut self, n: usize) {
        let delta_t = DELTA_T / n as f32;
        for _ in 0..n {
        let dd_phi = - 2.0 * self.d_rad * self.d_phi / self.p_rad;
        self.d_phi += dd_phi * delta_t;
        self.p_phi += self.d_phi * delta_t;
        }
    }

    fn process(&mut self) -> (isize, isize) {
        if self.p_rad <= 0.0 {
            return (0, 0)
        }

        let step_num_phi = fabsf(10000.0 * self.d_phi);
        let step_num_rad = fabsf(100.0 * self.d_rad);
        let step_num = (step_num_rad + step_num_phi).max(1.0) as usize;
        self.step_rad(step_num);
        self.step_phi(step_num);

        let p_phi = if self.p_phi < 0.0 {
            2.0 * PI + self.p_phi
        } else if self.p_phi > 2.0 * PI {
            self.p_phi % (2.0 * PI)
        } else {
            self.p_phi
        } / (2.0*PI) * 24.0;

        let p_rad = self.p_rad;

        if self.p_phi < 0.0 {
            self.p_phi += 2.0 * PI;
        }

        (p_phi as isize, p_rad as isize)
    }
}


pub struct PlanetShow {
}

const NUM_PLANETS: usize = 10;

impl PlanetShow {
    pub fn new() -> PlanetShow {
        PlanetShow {  }
    }

    pub fn show(&mut self, interface: &mut Interface) {
        let mut a = 1.0;
        let mut hue = 0.0;
        let mut planets: [Planet; NUM_PLANETS] = core::array::from_fn(|_i| {
            let _a = a;
            a *= 1.1;
            hue += 1.0/(NUM_PLANETS as f32);
            let rad = interface.random().value() * 40.0 + 5.0;
            let phi = interface.random().value() * 2.0 * PI;

            let direction = if interface.random().value8() < 64 {
                -1.0
            } else {
                1.0
            };

            let color = Color::from_hsv(hue, 1.0, 0.5);

            Planet::new_vis_viva(rad, _a, phi, direction, color)
        });
        interface.led_strip().black();
        for n in 0..STRIP_NUM {
            let pos = n * STRIP_LENGTH;
            interface.led_strip().led_mut(pos).set_color_flickering(YELLOW, 192);
        }
        loop {
            for n in 0..STRIP_NUM {
                let pos = n * STRIP_LENGTH + 1;
                if interface.random().value8() < 8 && interface.led_strip().led(pos).is_black() {
                    interface.led_strip().set_led(pos as isize, YELLOW);
                    interface.led_strip().led_mut(pos).set_target_flickering(BLACK, 96, 255);
                }
            }

            for planet in planets.iter_mut() {
                let (strip_num, pos) = planet.process();

                if pos < STRIP_LENGTH as isize && pos > 0 {
                    let _pos = strip_num * STRIP_LENGTH as isize + pos;
                    interface.led_strip().set_led(_pos, planet.color);
                    interface.led_strip().set_led_target(_pos, BLACK, 255);
                }
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
