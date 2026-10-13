
use crate::{
    conf::{STRIP_LENGTH, STRIP_NUM},
    interface::{Interface},
    led::{DARK_WHITE, GREEN, RED},
    player::{Show, StepResult},
};

const SNOW_START_PROB: u32 = 4;
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

    fn reset(&mut self, iface: &mut Interface) -> &mut SnowFlake {
        self.strip = iface.random().value32(STRIP_NUM as u32) as isize;
        self.alt = (STRIP_LENGTH << 6) as isize;
        self
    }

    fn is_active(&self) -> bool {
        self.alt > 0
    }

    fn process(&mut self, iface: &mut Interface, wind: i32) {
        if !self.is_active() {
            return
        }

        let random = iface.random().value32(100);
        if random < CHANGE_CONDITION {
            self.strip = if random % 2 == 0 {
                self.strip + 1
            } else {
                self.strip - 1
            } % STRIP_NUM as isize;
        }

        if iface.random().value8() % 2 == 0 {
            self.strip += (wind / 32) as isize;
        }
        self.strip %= STRIP_NUM as isize;

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


pub struct SnowShow<const NUM_SNOW_FLAKES: usize> {
    flakes: [SnowFlake; NUM_SNOW_FLAKES],
    coverage: [usize; STRIP_NUM],

    wind: i32,
    with_wind: bool,
    wind_count:  i32
}

impl<const NUM_SNOW_FLAKES: usize> SnowShow<NUM_SNOW_FLAKES> {
    pub fn new() -> SnowShow<NUM_SNOW_FLAKES> {
        let flakes: [SnowFlake; NUM_SNOW_FLAKES] = core::array::from_fn(|i| i+1).map(|_i| SnowFlake::new());

        let coverage: [usize; STRIP_NUM] = [0; STRIP_NUM];

        let wind = 0i32;
        let with_wind = true;
        let wind_count = 1;
        SnowShow { flakes, coverage, wind, with_wind, wind_count }
    }
}

impl<const NUM_SNOW_FLAKES: usize> Show for SnowShow<NUM_SNOW_FLAKES> {
    fn reset(&mut self, iface: &mut Interface<'_>) {
        for flake in self.flakes.iter_mut() {
            //flake.reset(iface);
            self.coverage[flake.strip()] = 0;
        }
        let _ = iface.led_on();
    }

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {
        iface.led_strip().black();

        if self.with_wind {
            self.wind_count = (self.wind_count + 1) % 3;
            if self.wind_count == 0 {
                self.wind = make_wind(iface, self.wind);
            }
        } else {
            self.wind = 0;
        }

        for flake in self.flakes.iter_mut() {
            if !flake.is_active() && iface.random().value32(2048) < SNOW_START_PROB {
                flake.reset(iface);
                if self.coverage[flake.strip()] > 0 {
                    self.coverage[flake.strip()] -= 1;
                }
            }
            if flake.is_active() {
                process_flake(iface, &mut self.coverage, flake,  self.wind);
            }

            handle_coverage(iface, &mut self.coverage);
        }

        //show_wind(iface, self.wind);

        iface.write_spi();

        StepResult::Running
    }
}

fn make_wind(iface: &mut Interface, wind: i32) -> i32 {
    if iface.random().value8() > 64 {
        return wind
    }

    let wind_change = iface.random().value32(5) as i32 - 2;
    (wind + wind_change).min(32).max(-32)
}

fn show_wind(iface: &mut Interface, wind: i32) {
    if wind < 0 {
        for i in 0..(-wind as usize) {
            iface.led_strip().set_led(i as isize, RED);
        }
    }
    if wind > 0 {
        for i in 0..(wind as usize) {
            iface.led_strip().set_led(i as isize, GREEN);
        }
    }
}

fn handle_coverage(iface: &mut Interface, coverage: &mut [usize; STRIP_NUM]) {
    for strip in 0..STRIP_NUM {
        average_coverage(strip, coverage);
        let mut local_coverage = coverage[strip];

        if local_coverage > 3 {
            local_coverage = 3 + (local_coverage-3) / 5;
        }
        for y in 0..local_coverage {
            let pos = (strip * STRIP_LENGTH + y) as isize;
            iface.led_strip().set_led(pos, DARK_WHITE);
        }
    }
}

fn process_flake(
    iface: &mut Interface,
    coverage: &mut [usize; STRIP_NUM],
    flake: &mut SnowFlake,
    wind: i32,
) {
    flake.process(iface, wind);
    if !flake.is_active() {
        coverage[flake.strip()] += 1;
    }
    iface.led_strip().set_led(flake.pos(), DARK_WHITE);
}

fn average_coverage(strip: usize, coverage: &mut [usize; STRIP_NUM]) {
    let left_neighbor = if strip == 0 {
        STRIP_NUM - 1
    } else {
        strip - 1
    };
    let right_neighbor = (strip + 1) % STRIP_NUM;
    let cov_left = coverage[left_neighbor];
    let cov_right = coverage[right_neighbor];

    let mut averaged_coverage = coverage[strip];

    let diff_left = averaged_coverage as isize - cov_left as isize;
    let diff_right = averaged_coverage as isize - cov_right as isize;

    if diff_left > 3 {
        coverage[left_neighbor] += 1;
        averaged_coverage -= 1;
    }

    if diff_right > 3 {
        coverage[right_neighbor] += 1;
        averaged_coverage -= 1;
    }

    coverage[strip] = averaged_coverage;
}
