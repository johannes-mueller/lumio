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
mod sine;
mod planets;
mod snow;

use interface::Interface;

use led::{WHITE, YELLOW, DARK_BLUE, DARK_GREEN};
use snake::SnakeShow;
use fire::Fire;
use stars::Stars;
use spiral::HueSpiral;
use sparks::{FireWorks, SparkFall, SnowSparks};
use particle_crash::ParticleCrash;
use sine::{SeaWave, SineShow};
use planets::PlanetShow;
use snow::snow;

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
    let mut sine_show = SineShow::new();
    let mut planet_show = PlanetShow::new();
    let mut sea_wave_chaos = SeaWave::new(Some(9800), 8);
    let mut sea_wave_order = SeaWave::new(Some(1050), 12);
    //let mut sea_wave_varying = SeaWave::new(None, 12);

    loop {
//        sea_wave_varying.show(&mut interface);
        planet_show.show(&mut interface);
        particle_crash.show_spiral(&mut interface);
        sine_show.show(&mut interface);
        fire_green.show(&mut interface);
        hue_spiral.show_lift(&mut interface);
        snow(&mut interface);
        sea_wave_chaos.show(&mut interface);
        fireworks.show(&mut interface);
        hue_spiral.show_swirl(&mut interface);
        snow_sparks.show(&mut interface);
        eo_stars.show(&mut interface);
        particle_crash.show(&mut interface);
        sea_wave_order.show(&mut interface);
        snake_show.show(&mut interface);
        falling_sparks.show(&mut interface);
        eu_stars.show(&mut interface);
        fire_red.show(&mut interface);
    }
}

// End of file
