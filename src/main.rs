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

const NUM_LED: usize = 720;
const TAIL: usize = NUM_LED / 11;
const DATA_SIZE: usize = NUM_LED*4+4+TAIL;

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8
}

const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const GREEN: Color = Color { r: 0, g: 1, b: 255 };
const BLACK: Color = Color { r: 0, g: 0, b: 0 };

#[derive(Clone, Copy)]
struct Led {
    current: Color,
    target: Color,
    decay: f32
}

impl Led {

}

struct LEDStrip {
    bytes: [u8; DATA_SIZE],
    leds: [Led; NUM_LED]
}

impl LEDStrip {
    pub fn new() -> LEDStrip {
        let mut bytes: [u8; DATA_SIZE] = [0x00u8; DATA_SIZE];
        for i in DATA_SIZE-TAIL..DATA_SIZE {
            bytes[i] = 0xff;
        }
        let default_led = Led{ current: BLACK, target: BLACK, decay: 0.0 };
        let leds = [default_led; NUM_LED];
        LEDStrip { bytes, leds }
    }

    pub fn set_led(&mut self, pos: isize, color: Color) {
        let i: usize = if pos < 0 {
            (NUM_LED - pos.abs() as usize) % NUM_LED
        } else {
            pos as usize % NUM_LED
        };
        self.leds[i].target = color;
        self.leds[i].current = color;

    }

    pub fn dump(&mut self) -> &[u8; DATA_SIZE] {
        for i in 0..NUM_LED {
            self.bytes[4+i*4] = 0xff;
            self.bytes[4+i*4+1] = self.leds[i].current.b;
            self.bytes[4+i*4+2] = self.leds[i].current.g;
            self.bytes[4+i*4+3] = self.leds[i].current.r;
        }
        &self.bytes
    }
}



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

    let mut spi = Spi::<_, _, _, 8>::new(spi_device, spi_pin_layout)
        .init(&mut pac.RESETS, 125_000_000u32.Hz(), 4_000_000u32.Hz(), MODE_0);


    // This is the correct pin on the Raspberry Pico board. On other boards, even if they have an
    // on-board LED, it might need to be changed.
    // Notably, on the Pico W, the LED is not connected to any of the RP2040 GPIOs but to the cyw43 module instead. If you have
    // a Pico W and want to toggle a LED with a simple GPIO output pin, you can connect an external
    // LED to one of the GPIO pins, and reference that pin here.
    // let mut led_pin = pins.led.into_push_pull_output();


    let mut led_strip: LEDStrip = LEDStrip::new();

    let mut i: isize = 0;
    loop {
        led_strip.set_led(i, BLUE);
        led_strip.set_led(i-1, BLACK);
        let _ = spi.write(led_strip.dump());
        //delay.delay_ms(1);
        delay.delay_us(500);
        i = (i+1) % NUM_LED as isize;
        // info!("off!");
        // led_pin.set_low().unwrap();
        // let _ = spi.write(&led_data);
        // delay.delay_ms(500);
    }
}

// End of file
