use crate::{
    conf::*,
    interface::Interface,
    led::WHITE,
    ledstrip::LEDStrip,
    player::{Show, StepResult},
    sparks::Explosions,
};

const ISTRIP_LENGTH: isize = STRIP_LENGTH as isize;

#[derive(Clone, Copy)]
enum Manor {
    Randomly,
    Spiral,
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

    fn is_active(&self) -> bool {
        self.current_position >= 0
    }

    fn deactivate(&mut self) {
        self.current_position = -1
    }

    fn activate(&mut self) {
        self.current_position = 0
    }

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

    fn is_active(&self) -> bool {
        self.current_position >= 0
    }

    fn deactivate(&mut self) {
        self.current_position = -1
    }

    fn activate(&mut self) {
        self.current_position = 1
    }

    fn process(&mut self, led_strip: &mut LEDStrip) {
        if !self.is_active() {
            return;
        }

        let pos = ISTRIP_LENGTH * self.strip + STRIP_LENGTH as isize - self.current_position;
        led_strip.set_led(pos, WHITE);

        self.current_position += 1;

        if self.current_position > STRIP_LENGTH as isize {
            self.deactivate();
        }
    }
}

pub struct ParticleCrash {
    big_particles: [BigParticle; STRIP_NUM],
    small_particles: [SmallParticle; STRIP_NUM],
    explosions: Explosions,
    manor: Manor,
    center_hue: f32,
    step: usize,
}

impl ParticleCrash {
    fn with_manor(manor: Manor) -> ParticleCrash {
        ParticleCrash {
            big_particles: core::array::from_fn(|i| i).map(|strip| BigParticle::new(strip)),
            small_particles: core::array::from_fn(|i| i).map(|strip| SmallParticle::new(strip)),
            explosions: Explosions::new(),
            manor,
            center_hue: 0.0,
            step: 0,
        }
    }

    pub fn new_random() -> ParticleCrash {
        ParticleCrash::with_manor(Manor::Randomly)
    }

    pub fn new_spiral() -> ParticleCrash {
        ParticleCrash::with_manor(Manor::Spiral)
    }

    fn handle_colision(&mut self, strip: usize, explosion_hue: f32, interface: &mut Interface<'_>) {
        let bp = &mut self.big_particles[strip];
        let sp = &mut self.small_particles[strip];

        if !bp.is_active() {
            return;
        }

        if bp.current_position > ISTRIP_LENGTH - sp.current_position {
            self.explosions
                .explode(strip, bp.current_position as usize, explosion_hue, interface);
            bp.deactivate();
            sp.deactivate();
        }
    }

    fn randomly_activate_particles_on_strip(
        &mut self,
        strip: usize,
        interface: &mut Interface<'_>,
    ) {
        let no_crash_on_strip = self.explosions.no_explosion_on_strip(strip);

        let bp = &mut self.big_particles[strip];
        let sp = &mut self.small_particles[strip];

        if !bp.is_active() && no_crash_on_strip && interface.random().value() < SPARK_PROB {
            bp.activate()
        }

        if !sp.is_active() && no_crash_on_strip && interface.random().value() < SPARK_PROB {
            sp.activate()
        }
    }

    fn activate_for_spiral(&mut self, strip: usize) {
        let bp = &mut self.big_particles[strip];
        let sp = &mut self.small_particles[strip];

        if !sp.is_active() && strip * 2 == self.step {
            sp.activate();
        }

        if !bp.is_active() && strip * 2 == self.step {
            bp.activate();
        }
    }
}

impl Show for ParticleCrash {
    fn reset(&mut self, _iface: &mut Interface<'_>) {
        self.explosions.reset();
        for bp in self.big_particles.iter_mut() {
            bp.deactivate();
        }
        for sp in self.small_particles.iter_mut() {
            sp.deactivate();
        }
        self.center_hue = 0.0;
        self.step = 0;
    }

    fn step(&mut self, interface: &mut Interface<'_>) -> StepResult {
        interface.led_strip().black();

        self.explosions.process(interface);

        if !self.big_particles.iter().any(|p| p.is_active()) {
            self.center_hue += 1.0 / 7.0;
        }

        for strip in 0..STRIP_NUM {
            self.big_particles[strip].process(&mut interface.led_strip());
            self.small_particles[strip].process(&mut interface.led_strip());

            let hue = match self.manor {
                Manor::Randomly => interface.random().value(),
                Manor::Spiral => random_hue_around_given(self.center_hue, interface),
            };
            self.handle_colision(strip, hue, interface);

            match self.manor {
                Manor::Randomly => self.randomly_activate_particles_on_strip(strip, interface),
                Manor::Spiral => self.activate_for_spiral(strip),
            }
        }

        self.step = (self.step + 1) % (STRIP_NUM * 2);

        if self.step == 0 {
            self.center_hue += 1.0 / 7.0;
        }

        interface.write_spi();
        StepResult::Running
    }
}

fn random_hue_around_given(center_hue: f32, interface: &mut Interface<'_>) -> f32 {
    (interface.random().value() / 6.0 + center_hue) % 1.0
}
