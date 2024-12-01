use crate::{
    conf::*,
    interface::Interface,
    led::{self, Color, BLACK, WHITE},
    ledstrip::LEDStrip,
    sparks::{MonoSpark, SPARK_NUM}
};

const ISTRIP_LENGTH: isize = STRIP_LENGTH as isize;


enum Manor {
    Randomly,
    Spiral
}


#[derive(Clone, Copy)]
pub struct BigParticle {
    strip: isize,
    current_position: isize,
}

impl BigParticle {
    fn new(strip: usize) -> BigParticle {
        BigParticle {
            strip: strip as isize,
            current_position: -1,
        }
    }

    fn is_active(&self) -> bool { self.current_position >= 0 }

    fn deactivate(&mut self) { self.current_position = -1 }

    fn activate(&mut self) { self.current_position = 0 }


    fn process(&mut self, led_strip: &mut LEDStrip) {
        if !self.is_active() {
            return;
        }

        let pos = ISTRIP_LENGTH * self.strip + self.current_position;
        led_strip.set_led(pos, WHITE);

        self.current_position += 1;

        if self.current_position > STRIP_LENGTH as isize {
            self.deactivate();
        }
    }
}


#[derive(Clone, Copy)]
pub struct SmallParticle {
    strip: isize,
    current_position: isize,
}

impl SmallParticle {
    fn new(strip: usize) -> SmallParticle {
        SmallParticle {
            strip: strip as isize,
            current_position: -1,
        }
    }

    fn is_active(&self) -> bool { self.current_position >= 0 }

    fn deactivate(&mut self) { self.current_position = -1 }

    fn activate(&mut self) { self.current_position = 0 }


    fn process(&mut self, led_strip: &mut LEDStrip) {
        if !self.is_active() {
            return;
        }

        let pos = ISTRIP_LENGTH * self.strip + STRIP_LENGTH as isize - self.current_position;
        led_strip.set_led(pos, WHITE);

        self.current_position += 2;

        if self.current_position > STRIP_LENGTH as isize {
            self.deactivate();
        }
    }
}

pub struct ParticleCrash {
    big_particles: [BigParticle; STRIP_NUM],
    small_particles: [SmallParticle; STRIP_NUM],
    sparks: [MonoSpark; SPARK_NUM],
    step: usize
}

impl ParticleCrash {
    pub fn new() -> ParticleCrash {
        ParticleCrash {
            big_particles: core::array::from_fn(|i| i).map(|strip| BigParticle::new(strip)),
            small_particles: core::array::from_fn(|i| i).map(|strip| SmallParticle::new(strip)),
            sparks: core::array::from_fn(|i| i).map(|n| MonoSpark::new_noaccel(n / SPARKS_PER_STRIP)),
            step: 0
        }
    }

    fn no_crash_on_strip(&self, strip: usize) -> bool {
        let start = strip * SPARKS_PER_STRIP;
        let end = start + SPARKS_PER_STRIP;

        !(start..end).any(|i| self.sparks[i].is_active())
    }

    fn handle_colision(&mut self, strip: usize, explosion_hue: f32, interface: &mut Interface) {
        let bp = &mut self.big_particles[strip];
        let sp = &mut self.small_particles[strip];

        if bp.current_position > ISTRIP_LENGTH - sp.current_position {
            let start = strip * SPARKS_PER_STRIP;
            let end = start + SPARKS_PER_STRIP;

            for i in start..end {
                let spark = &mut self.sparks[i];
                let speed = (interface.random().value8()) as isize - 128;
                spark.reset(explosion_hue, speed, 64, 255, bp.current_position);
                //spark.reset(hue, 0.1, speed, current_position);
            }

            bp.deactivate();
            sp.deactivate();
        }
    }

    fn randomly_activate_particles_on_strip(&mut self, strip: usize, interface: &mut Interface) {
        let no_crash_on_strip = self.no_crash_on_strip(strip);

        let bp = &mut self.big_particles[strip];
        let sp = &mut self.small_particles[strip];

        if !bp.is_active() && no_crash_on_strip  && interface.random().value() < SPARK_PROB {
            bp.activate()
        }

        if !sp.is_active() && no_crash_on_strip  && interface.random().value() < SPARK_PROB {
            sp.activate()
        }
    }

    fn activate_for_spiral(&mut self, strip: usize) {
        let no_crash_on_strip = self.no_crash_on_strip(strip);

        let bp = &mut self.big_particles[strip];
        let sp = &mut self.small_particles[strip];

        if !sp.is_active() && bp.is_active()  && strip == self.step {
            sp.activate();
        }

        if self.sparks.iter().any(|s| s.is_active()) {
            return
        }

        if !bp.is_active() {
            bp.activate();
        }
    }

    pub fn show(&mut self, interface: &mut Interface) {
        self.do_show(interface, Manor::Randomly);
    }

    pub fn show_spiral(&mut self, interface: &mut Interface) {
        self.step = 0;
        self.do_show(interface, Manor::Spiral);
    }

    fn do_show(&mut self, interface: &mut Interface, manor: Manor) {
        let mut center_hue = 0.0;

        for s in self.sparks.iter_mut() {
            s.deactivate();
        }
        for bp in self.big_particles.iter_mut() {
            bp.deactivate();
        }

        for sp in self.small_particles.iter_mut() {
            sp.deactivate();
        }
        loop {
            interface.led_strip().black();

            for spark in self.sparks.iter_mut() {
                spark.process(&mut interface.led_strip())
            };


            if !self.big_particles.iter().any(|p| p.is_active()) {
                self.step = 0;
                center_hue = (center_hue + 1.0 / 7.0);
            }

            for strip in 0..STRIP_NUM {
                self.big_particles[strip].process(&mut interface.led_strip());
                self.small_particles[strip].process(&mut interface.led_strip());

                let hue = match manor {
                    Manor::Randomly => interface.random().value(),
                    Manor::Spiral => random_hue_around_given(center_hue, interface)
                };
                self.handle_colision(strip, hue, interface);

                match manor {
                    Manor::Randomly => self.randomly_activate_particles_on_strip(strip, interface),
                    Manor::Spiral => self.activate_for_spiral(strip)
                }
            };

            self.step = (self.step+1) % STRIP_NUM;

            interface.write_spi();
            if interface.do_next() {
                interface.led_strip().black();
                let _ = interface.led_off();
                break;
            }
        }
    }
}


fn random_hue_around_given(center_hue: f32, interface: &mut Interface) -> f32 {
    (interface.random().value() / 6.0 + center_hue) % 1.0
}
