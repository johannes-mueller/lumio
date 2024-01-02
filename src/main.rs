//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]


use core::usize;


use bsp::entry;
use defmt::*;
use defmt_rtt as _;
//use embedded_hal::digital::v2::OutputPin;
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use embedded_hal::spi::MODE_0;
use embedded_hal::blocking::spi::Write;
use fugit::RateExtU32;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    spi::Spi,
    gpio::FunctionSpi,
    watchdog::Watchdog,
};

use embedded_hal::digital::v2::OutputPin;

mod conf;
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

use conf::{SNAKE_PROB, SPARK_PROB, SPARKS_PER_STRIP, STRIP_NUM};
use led::{Color, WHITE, YELLOW, DARK_BLUE, DARK_GREEN};
use button::{Button, ButtonState};
use ledstrip::LEDStrip;
use snake::Snake;
use fire::Fire;
use stars::Stars;
use spiral::Spiral;
use huewave::HueWave;
use sparks::{MonoSpark, ColorSpark, FallingSparks};
use random::Random;
use showtimer::ShowTimer;



#[entry]
fn main() -> ! {

    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let timer = bsp::hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // let sclk = pins.gpio18.into_function::<FunctionSpi>();
    // let mosi = pins.gpio19.into_function::<FunctionSpi>();

    // let spi_device = pac.SPI0;
    // let spi_pin_layout = (mosi, sclk);

    // let mut spi0 = Spi::<_, _, _, 8>::new(spi_device, spi_pin_layout)
    //     .init(&mut pac.RESETS, 450_000_000u32.Hz(), 8_000_000u32.Hz(), MODE_0);


    let sclk = pins.gpio14.into_function::<FunctionSpi>();
    let mosi = pins.gpio15.into_function::<FunctionSpi>();

    let spi_device = pac.SPI1;
    let spi_pin_layout = (mosi, sclk);

    let mut spi1 = Spi::<_, _, _, 8>::new(spi_device, spi_pin_layout)
        .init(&mut pac.RESETS, 450_000_000u32.Hz(), 8_000_000u32.Hz(), MODE_0);

    let button_1 = Button::new(pins.gpio21.into_pull_up_input(), &timer);
    let mut button_2 = Button::new(pins.gpio20.into_pull_up_input(), &timer);
    let led_1_pin = pins.gpio10.into_push_pull_output();
    let mut led_2_pin = pins.gpio11.into_push_pull_output();


    let mut led_strip: LEDStrip = LEDStrip::new();

    let mut random = Random::new(2495823494);

    let strips: [usize; STRIP_NUM] = core::array::from_fn(|i| i+1);

    const SPARK_NUM: usize = STRIP_NUM * SPARKS_PER_STRIP;
    let mut mono_sparks: [MonoSpark; SPARK_NUM] = core::array::from_fn(|i| i+1).map(|sn| MonoSpark::new(sn / SPARKS_PER_STRIP));
    let mut color_sparks: [ColorSpark; STRIP_NUM] = core::array::from_fn(|i| i+1).map(|strip| ColorSpark::new(strip));

    let mut constant_snakes: [Snake; STRIP_NUM] = [Snake::default(); STRIP_NUM];
    let mut random_snakes: [Snake; STRIP_NUM] = [Snake::default(); STRIP_NUM];

    let mut fire = Fire::new();
    let mut eu_stars = Stars::new(DARK_BLUE, YELLOW);
    let mut eo_stars = Stars::new(DARK_GREEN, WHITE);
    let mut spiral = Spiral::new(5);
    let mut huewave = HueWave::new();

    let mut showtimer = ShowTimer::new(button_1, led_1_pin, &timer);

    let mut matrix_sparks: [FallingSparks; SPARK_NUM] =
        core::array::from_fn(|i| i+1)
        .map(|sn| FallingSparks::new(sn / SPARKS_PER_STRIP, Color {r: 32, g: 0, b: 32}));

    loop {
        loop {
            led_strip.black();
            for ms in matrix_sparks.iter_mut() {
                if !ms.is_active() && random.value() < SPARK_PROB {
                    ms.reset();
                }
                ms.process(&mut led_strip);
            }
            delay.delay_ms(10);
            let _ = spi1.write(led_strip.dump_0());
            if showtimer.do_next() {
                led_strip.black();
                let _ = led_2_pin.set_low();
                break;
            }
        }

        loop {
            led_strip.black();
            for sp in mono_sparks.iter_mut() {
                sp.process(&mut led_strip);
            }
            for sp in color_sparks.iter_mut() {
                sp.process(&mut led_strip);
            }
            let _ = spi1.write(led_strip.dump_0());
            if !mono_sparks.iter().any(|sp| sp.is_active()) {
                if random.value() < SPARK_PROB || button_2.state() == ButtonState::ShortPressed {
                    let _ = led_2_pin.set_high();
                    let hue = random.value();
                    for sp in mono_sparks.iter_mut() {
                        let speed = random.value8() as isize;
                        let decay = random.value8() >> 2;
                        let brightness = 127 + random.value8() % 128;
                        sp.reset(hue, speed, decay, brightness);
                    };
                    for strip in 0..STRIP_NUM {
                        let start = strip * SPARKS_PER_STRIP;
                        let end = start + SPARKS_PER_STRIP;
                        let speed = (start..end).map(|i| mono_sparks[i].speed()).max().unwrap();
                        let hue = random.value();
                        let decay = 0.1;
                        color_sparks[strip].reset(hue, decay, speed);
                    };
                } else {
                    let _ = led_2_pin.set_low();
                }
            }
            if showtimer.do_next() {
                led_strip.black();
                let _ = led_2_pin.set_low();
                break;
            }
        }

        loop {
            huewave.process(&mut led_strip);
            spiral.process(&mut led_strip);
            let _ = spi1.write(led_strip.dump_0());
            if showtimer.do_next() {
                led_strip.black();
                break;
            }
//            delay.delay_ms(50);
        }

        let mut running = false;
        let mut step = 0;

        loop {
            if !running {
                for i in 0..STRIP_NUM {
                    constant_snakes[i].reset(strips[i], random.value(), 60./360.);
                }
            }
            if constant_snakes.iter().all(|sn| sn.is_done()) {
                let _ = led_2_pin.set_low();
                running = false;
            }
            if (button_2.state() == ButtonState::ShortPressed && !running) || step == 0 {
                let _ = led_2_pin.set_high();
                running = true;
            }
            for sn in constant_snakes.iter_mut() {
                sn.process(&mut led_strip);
            }
            let _ = spi1.write(led_strip.dump_0());
            if random.value8() < SNAKE_PROB {
                let cand = random.value32(STRIP_NUM as u32) as usize;
                if random_snakes[cand].is_done() {
                    random_snakes[cand].reset(cand, random.value(), 60./360.);
                }
            }
            for sn in random_snakes.iter_mut() {
                sn.process(&mut led_strip);
            }

            if showtimer.do_next() {
                led_strip.black();
                let _ = led_2_pin.set_low();
                break;
            }

            step = (step + 1) % 1024;
            //        delay.delay_ms(1);
        }

        eu_stars.reset(&mut led_strip);
        loop {
            eu_stars.process(&mut led_strip);
            let _ = spi1.write(led_strip.dump_0());

            if showtimer.do_next() {
                led_strip.black();
                break;
            }
        }

        loop {
            fire.process(&mut led_strip);
            let _ = spi1.write(led_strip.dump_0());

            if showtimer.do_next() {
                led_strip.black();
                break;
            }
        }

        eo_stars.reset(&mut led_strip);
        loop {
            eo_stars.process(&mut led_strip);
            let _ = spi1.write(led_strip.dump_0());

            if showtimer.do_next() {
                led_strip.black();
                break;
            }
        }
    }
}

// End of file
