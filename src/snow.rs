use crate::{button::ButtonState, conf::{STRIP_LENGTH, STRIP_NUM}, interface::Interface, led::{DARK_WHITE, GREEN, RED}};

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

    fn reset(&mut self, interface: &mut Interface) -> &mut SnowFlake {
        self.strip = interface.random().value32(STRIP_NUM as u32) as isize;
        self.alt = (STRIP_LENGTH << 6) as isize;
        self
    }

    fn is_active(&self) -> bool {
        self.alt > 0
    }

    fn process(&mut self, interface: &mut Interface, wind: i32) {
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

        if interface.random().value8() % 2 == 0 {
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


pub fn snow<const NUM_SNOW_FLAKES: usize>(interface: &mut Interface) {
    let mut flakes: [SnowFlake; NUM_SNOW_FLAKES] = core::array::from_fn(|i| i+1).map(|_i| SnowFlake::new());

    let mut coverage: [usize; STRIP_NUM] = [0; STRIP_NUM];

    let mut wind = 0i32;
    let mut with_wind = true;
    let mut wind_count = 1;
    let _ = interface.led_on();

    loop {
        interface.led_strip().black();

        if with_wind {
            wind_count = (wind_count + 1) % 3;
            if wind_count == 0 {
                wind = make_wind(interface, wind);
            }
        } else {
            wind = 0;
        }

        for flake in flakes.iter_mut() {
            if !flake.is_active() && interface.random().value32(2048) < SNOW_START_PROB {
                flake.reset(interface);
                if coverage[flake.strip()] > 0 {
                    coverage[flake.strip()] -= 1;
                }
            }
            if flake.is_active() {
                process_flake(interface, &mut coverage, flake,  wind);
            }

            handle_coverage(interface, &mut coverage);
        }

        //show_wind(interface, wind);

        interface.write_spi();

        if interface.button_state() == ButtonState::LongPressed {
            with_wind = !with_wind;
            if with_wind {
                let _ = interface.led_on();
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
}

fn make_wind(interface: &mut Interface, wind: i32) -> i32 {
    if interface.random().value8() > 64 {
        return wind
    }

    let wind_change = interface.random().value32(5) as i32 - 2;
    (wind + wind_change).min(32).max(-32)
}

fn show_wind(interface: &mut Interface, wind: i32) {
    if wind < 0 {
        for i in 0..(-wind as usize) {
            interface.led_strip().set_led(i as isize, RED);
        }
    }
    if wind > 0 {
        for i in 0..(wind as usize) {
            interface.led_strip().set_led(i as isize, GREEN);
        }
    }
}

fn handle_coverage(interface: &mut Interface, coverage: &mut [usize; STRIP_NUM]) {
    for strip in 0..STRIP_NUM {
        average_coverage(strip, coverage);
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

fn process_flake(
    interface: &mut Interface,
    coverage: &mut [usize; STRIP_NUM],
    flake: &mut SnowFlake,
    wind: i32,
) {
    flake.process(interface, wind);
    if !flake.is_active() {
        coverage[flake.strip()] += 1;
    }
    interface.led_strip().set_led(flake.pos(), DARK_WHITE);
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
