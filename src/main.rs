//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]


use core::usize;
use rp_pico::entry;
use defmt_rtt as _;
use panic_probe as _;

mod conf;
mod interface;
mod button;
mod showtimer;
mod math8;
mod led;
mod ledstrip;
mod snake;
mod random;
mod fire;
mod stars;
mod spiral;
mod huewave;
mod sparks;
mod particle_crash;

use interface::Interface;

use led::{WHITE, YELLOW, RED, GREEN, DARK_BLUE, DARK_GREEN};
use snake::SnakeShow;
use fire::Fire;
use stars::Stars;
use spiral::HueSpiral;
use sparks::{FireWorks, SparkFall, SnowSparks};
use particle_crash::{ParticleCrash};

#[entry]
fn main() -> ! {
    let mut interface = Interface::new();

    let mut hue_spiral = HueSpiral::new();
    let mut fireworks = FireWorks::new();
    let mut fire_red = Fire::new_red();
    let mut fire_green = Fire::new_green();
    let mut eu_stars = Stars::new(DARK_BLUE, YELLOW);
    let mut eo_stars = Stars::new(DARK_GREEN, WHITE);
    let mut falling_sparks = SparkFall::new();
    let mut snow_sparks = SnowSparks::new();
    let mut snake_show = SnakeShow::new();
    let mut particle_crash = ParticleCrash::new();

    loop {
        particle_crash.show(&mut interface);
        fire_green.show(&mut interface);
        hue_spiral.show_lift(&mut interface);
        fireworks.show(&mut interface);
        hue_spiral.show_swirl(&mut interface);
        snow_sparks.show(&mut interface);
        snake_show.show(&mut interface);
        falling_sparks.show(&mut interface);
        eu_stars.show(&mut interface);
        fire_red.show(&mut interface);
        eo_stars.show(&mut interface);
    }
}

// End of file
