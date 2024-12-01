use crate::{
    conf::*,
    interface::Interface,
    led::{self, Color, BLACK, WHITE},
    ledstrip::LEDStrip,
    sparks::{MonoSpark, SPARK_NUM}
};

const ISTRIP_LENGTH: isize = STRIP_LENGTH as isize;


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
    sparks: [MonoSpark; SPARK_NUM]
}

impl ParticleCrash {
    pub fn new() -> ParticleCrash {
        ParticleCrash {
            big_particles: core::array::from_fn(|i| i).map(|strip| BigParticle::new(strip)),
            small_particles: core::array::from_fn(|i| i).map(|strip| SmallParticle::new(strip)),
            sparks: core::array::from_fn(|i| i).map(|n| MonoSpark::new_noaccel(n / SPARKS_PER_STRIP))
        }
    }

    pub fn no_crash_on_strip(&self, strip: usize) -> bool {
        let start = strip * SPARKS_PER_STRIP;
        let end = start + SPARKS_PER_STRIP;

        !(start..end).any(|i| self.sparks[i].is_active())
    }

    pub fn show(&mut self, interface: &mut Interface) {
        loop {
            interface.led_strip().black();

            for spark in self.sparks.iter_mut() {
                // if spark.current_position() > ISTRIP_LENGTH {
                //     spark.deactivate();
                // }
                spark.process(&mut interface.led_strip())
            };
            for strip in 0..STRIP_NUM {
                let no_crash_on_strip = self.no_crash_on_strip(strip);


                let bp = &mut self.big_particles[strip];
                let sp = &mut self.small_particles[strip];

                let current_position = bp.current_position;

                bp.process(&mut interface.led_strip());
                sp.process(&mut interface.led_strip());

                if !bp.is_active() {
                    if no_crash_on_strip  && interface.random().value() < SPARK_PROB {
                        bp.activate()
                    }
                    continue;
                }
                if !sp.is_active() {
                    if no_crash_on_strip && interface.random().value() < SPARK_PROB {
                        sp.activate()
                    }
                    continue;
                }

                if bp.current_position > ISTRIP_LENGTH - sp.current_position {
                    let start = strip * SPARKS_PER_STRIP;
                    let end = start + SPARKS_PER_STRIP;
                    let hue = interface.random().value();

                    for i in start..end {
                        let spark = &mut self.sparks[i];
                        let speed = (interface.random().value8()) as isize - 128;
                        spark.reset(hue, speed, 64, 255, current_position);
                        //spark.reset(hue, 0.1, speed, current_position);
                    }

                    bp.deactivate();
                    sp.deactivate();
                }
            };
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
        for bp in self.big_particles.iter_mut() {
            bp.deactivate();
        }

        for sp in self.small_particles.iter_mut() {
            sp.deactivate();
        }
    }
}
