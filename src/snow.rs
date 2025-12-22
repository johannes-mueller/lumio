
use crate::{conf::{STRIP_LENGTH, STRIP_NUM}, interface::Interface, led::DARK_WHITE};

const MAX_ACTIVE_SNOW_FLAKES: usize = 720;
const NUM_SNOW_FLAKES: usize = 180;
const SNOW_START_PROP: u32 = 4;
const CHANGE_CONDITION: u32 = 5;
const FALL_SPEED: isize = 12;

struct SnowFlake {
    strip: isize,
    alt: isize
}


impl SnowFlake {
    fn new() -> SnowFlake {
        SnowFlake {
            strip: 0,
            alt: 0
        }
    }

    fn reset(&mut self, interface: &mut Interface) -> &mut SnowFlake {
        self.strip = interface.random().value32(STRIP_NUM as u32) as isize;
        self.alt = (STRIP_LENGTH << 6) as isize;
        self
    }

    fn is_active(&self) -> bool {
        self.alt > 0
    }

    fn process(&mut self, interface: &mut Interface) {
        if !self.is_active() {
            return
        }

        let random = interface.random().value32(100);
        if random < CHANGE_CONDITION {
            self.strip = if random % 2 == 0 {
                self.strip + 1
            } else {
                self.strip - 1
            } % STRIP_NUM as isize;
        }

        self.alt -= FALL_SPEED;
    }

    fn pos(&self) -> isize {
        let led_alt = self.alt >> 6;
        self.strip * STRIP_LENGTH as isize + led_alt
    }

    fn strip(&self) -> usize {
        (self.strip as usize) % STRIP_NUM
    }
}

pub fn snow(interface: &mut Interface) {
    let mut flakes: [SnowFlake; NUM_SNOW_FLAKES] = core::array::from_fn(|i| i+1).map(|_i| SnowFlake::new());

    let mut active_snow_flakes = 0usize;
    let mut coverage: [usize; STRIP_NUM] = [0; STRIP_NUM];

    loop {
        interface.led_strip().black();

        for flake in flakes.iter_mut() {
            if active_snow_flakes > MAX_ACTIVE_SNOW_FLAKES {
                continue;
            }
            if !flake.is_active() && interface.random().value32(2048) < SNOW_START_PROP {
                flake.reset(interface);
                active_snow_flakes += 1;
                if coverage[flake.strip()] > 0 {
                    coverage[flake.strip()] -= 1;
                }
            }
            if !flake.is_active() {
                continue;
            }
            flake.process(interface);
            if !flake.is_active() {
                coverage[flake.strip()] += 1;
            }
            interface.led_strip().set_led(flake.pos(), DARK_WHITE);

            for strip in 0..STRIP_NUM {
                let mut local_coverage = coverage[strip];
                if local_coverage > 3 {
                    local_coverage = 3 + (local_coverage-3) / 5;
                }
                for y in 0..local_coverage {
                    let pos = (strip * STRIP_LENGTH + y) as isize;
                    interface.led_strip().set_led(pos, DARK_WHITE);
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
}
