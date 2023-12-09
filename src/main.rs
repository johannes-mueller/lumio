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

use embedded_hal::digital::v2::InputPin;
use embedded_hal::digital::v2::OutputPin;

mod conf;
mod led;
mod ledstrip;

use conf::NUM_LED;
use ledstrip::LEDStrip;
use led::{BLUE, YELLOW, BLACK, GREEN, Color};


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

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let sclk = pins.gpio18.into_function::<FunctionSpi>();
    let mosi = pins.gpio19.into_function::<FunctionSpi>();

    let spi_device = pac.SPI0;
    let spi_pin_layout = (mosi, sclk);

    let mut spi0 = Spi::<_, _, _, 8>::new(spi_device, spi_pin_layout)
        .init(&mut pac.RESETS, 125_000_000u32.Hz(), 4_000_000u32.Hz(), MODE_0);


    let sclk = pins.gpio14.into_function::<FunctionSpi>();
    let mosi = pins.gpio15.into_function::<FunctionSpi>();

    let spi_device = pac.SPI1;
    let spi_pin_layout = (mosi, sclk);

    let mut spi1 = Spi::<_, _, _, 8>::new(spi_device, spi_pin_layout)
        .init(&mut pac.RESETS, 125_000_000u32.Hz(), 4_000_000u32.Hz(), MODE_0);



    let button_1_pin = pins.gpio21.into_pull_up_input();
    let button_2_pin = pins.gpio20.into_pull_up_input();
    let mut led_1_pin = pins.gpio10.into_push_pull_output();
    let mut led_2_pin = pins.gpio11.into_push_pull_output();
    // This is the correct pin on the Raspberry Pico board. On other boards, even if they have an
    // on-board LED, it might need to be changed.
    // Notably, on the Pico W, the LED is not connected to any of the RP2040 GPIOs but to the cyw43 module instead. If you have
    // a Pico W and want to toggle a LED with a simple GPIO output pin, you can connect an external
    // LED to one of the GPIO pins, and reference that pin here.
    // let mut led_pin = pins.led.into_push_pull_output();


    let mut led_strip: LEDStrip = LEDStrip::new();

    led_strip.set_led(NUM_LED as isize, YELLOW);
    led_strip.set_led(NUM_LED as isize-2, YELLOW);
    led_strip.set_led(NUM_LED as isize-4, YELLOW);
    led_strip.set_led(NUM_LED as isize-6, YELLOW);

    let mut i: isize = 0;

    loop {
        led_strip.set_led(i+1, YELLOW);
        led_strip.set_led(i, BLUE);
        let nl = NUM_LED as isize;
        //led_strip.set_led(i+nl/2, GREEN);
        //led_strip.set_led(i-1+nl/2, BLACK);
        //led_strip.set_led(i-1, BLACK);
        //led_strip.set_led_target(i-1, BLACK, 0.3);
        led_strip.set_led(i-1, Color { r: 0.1, g: 0.1, b: 0.1 });
        let _ = spi1.write(led_strip.dump_0());
        let _ = spi0.write(led_strip.dump_1());
        //let _ = spi0.write(&buf[NUM_LED/2..NUM_LED]);
        //delay.delay_ms(1);
        if button_1_pin.is_high().unwrap() {
            let _ = led_1_pin.set_high();
        } else {
            let _ = led_1_pin.set_low();
        }
        if button_2_pin.is_high().unwrap() {
            let _ = led_2_pin.set_high();
        } else {
            let _ = led_2_pin.set_low();
        }
        delay.delay_ms(1);
        i = (i+1) % NUM_LED as isize;
        // info!("off!");
        // led_pin.set_low().unwrap();
        // let _ = spi.write(&led_data);
        // delay.delay_ms(500);
    }
}

// End of file
