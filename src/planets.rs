use core::f32::consts::PI;

use libm::{fabsf, sqrtf};

use crate::{
    button::ButtonState, conf::*, interface::Interface, led::{Color, BLACK, YELLOW}, sparks::Explosions
};

const DELTA_T: f32 = 10.0;

pub struct Planet {
    p_rad: f32,
    p_phi: f32,

    d_rad: f32,
    d_phi: f32,

    pub color: Color,

    is_active: bool
}


impl Planet {
    pub fn new(p_rad: f32, p_phi: f32, d_rad: f32, d_phi: f32, color: Color) -> Planet {
        Planet{p_rad, p_phi, d_rad, d_phi, color, is_active: true}
    }

    pub fn new_vis_viva(rad: f32, a: f32, phi: f32, direction: f32, color: Color) -> Planet {
        let v = sqrtf(fabsf((2.0/rad) - (1.0/(rad*a))));
        Planet::new(rad, phi, 0.0, direction * v/rad, color)
    }

    pub fn reset_phi(&mut self, phi: f32) {
        self.p_phi = phi;
        self.is_active = true;
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

    fn position(&self) -> (isize, isize) {
        let p_phi = if self.p_phi < 0.0 {
            2.0 * PI + self.p_phi
        } else if self.p_phi > 2.0 * PI {
            self.p_phi % (2.0 * PI)
        } else {
            self.p_phi
        } / (2.0*PI) * STRIP_NUM as f32;

        let p_rad = self.p_rad;
        (p_phi as isize, p_rad as isize)
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


        if self.p_phi < 0.0 {
            self.p_phi += 2.0 * PI;
        }

        self.position()
    }

    fn is_active(&self) -> bool { self.is_active }

    fn deactivate(&mut self) { self.is_active = false; }
}


pub struct PlanetShow {
    explosions: Explosions,
}

const NUM_PLANETS: usize = 10;

impl PlanetShow {
    pub fn new() -> PlanetShow {
        PlanetShow { explosions: Explosions::new() }
    }

    pub fn show(&mut self, interface: &mut Interface) {
        let mut planets = initialize_planets(interface);
        let mut with_collisions = true;

        interface.led_on();

        interface.led_strip().black();

        make_sun_flicker(interface);

        loop {
            for n in 0..STRIP_NUM {
                make_sun_corona_on_strip(n, interface);
                make_rest_of_sky_black(n, interface);
            }

            if with_collisions {
                self.handle_colisions(&mut planets, interface);
            }

            self.process_planets(&mut planets, interface);
            self.explosions.process(interface);

            if interface.button_state() == ButtonState::LongPressed {
                with_collisions = !with_collisions;
                if with_collisions {
                    let _ = interface.led_on();
                } else {
                    let _ = interface.led_off();
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

    fn process_planets(&mut self, planets: &mut [Planet; NUM_PLANETS], interface: &mut Interface) {
        for planet in planets.iter_mut() {
            if !planet.is_active() {
                let (strip, _) = planet.position();
                if self.explosions.no_explosion_on_strip(strip as usize) {
                    planet.reset_phi(interface.random().value() * 2.0 * PI);
                }
            }
            let (strip_num, pos) = planet.process();
            if pos < STRIP_LENGTH as isize && pos > 0 && planet.is_active() {
                interface.led_strip().set_led(flat_pos(strip_num, pos), planet.color);
            }
        }
    }

    fn handle_colisions(&mut self, planets: &mut [Planet; NUM_PLANETS], interface: &mut Interface) {
        let positions: [(isize, isize); NUM_PLANETS] = core::array::from_fn(|i| planets[i].position());

        for i in 0..positions.len() {
            let (sn1, p1) = positions[i];
            for j in i+1..NUM_PLANETS {
                let (sn2, p2) = positions[j];
                if p1 == p2 && sn1 == sn2 {
                    interface.led_strip().set_led(flat_pos(sn1, p1), BLACK);
                    let hue1 = i as f32 / NUM_PLANETS as f32;
                    self.explosions.explode(sn1 as usize, p1 as usize, hue1, interface);
                    planets[i].deactivate();
                    planets[j].deactivate();
                }
            }
        }
    }
}

fn initialize_planets(interface: &mut Interface) -> [Planet; NUM_PLANETS] {
    let mut a = 1.0;
    let mut hue = 0.0;
    let planets: [Planet; NUM_PLANETS] = core::array::from_fn(|_i| {
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
    planets
}

fn make_rest_of_sky_black(n: usize, interface: &mut Interface) {
    (0..STRIP_LENGTH).for_each(|pos| {
        let led = interface.led_strip().led_mut(n * STRIP_LENGTH + pos);
        if !led.is_flickering() {
            led.set_color(BLACK);
        }
    });
}

fn make_sun_corona_on_strip(n: usize, interface: &mut Interface) {
    let pos = n * STRIP_LENGTH + 1;
    if interface.random().value8() < 8 && interface.led_strip().led(pos).is_black() {
        interface.led_strip().set_led(pos as isize, YELLOW);
        interface.led_strip().led_mut(pos).set_target_flickering(BLACK, 96, 255);
    }
}

fn make_sun_flicker(interface: &mut Interface) {
    for n in 0..STRIP_NUM {
        let pos = n * STRIP_LENGTH;
        interface.led_strip().led_mut(pos).set_color_flickering(YELLOW, 192);
    }
}

fn flat_pos(strip_num: isize, pos: isize) -> isize {
    strip_num * STRIP_LENGTH as isize + pos
}
