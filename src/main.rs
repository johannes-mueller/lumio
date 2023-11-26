//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

//#![feature(alloc)]

extern crate alloc;

use core::usize;

use embedded_alloc::Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

use alloc::vec::Vec;

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

const NUM_LED: usize = 240;
const TAIL: usize = NUM_LED / 11;

#[entry]
fn main() -> ! {

    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

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

    let mut led_data = Vec::<u8>::from([0x00u8; NUM_LED*4+4+TAIL]);
    for i in NUM_LED*4+4..NUM_LED*4+4+TAIL {
        led_data[i] = 0xff;
    }
    // led_data[NUM_LED*4+4] = 0xff;
    // led_data[NUM_LED*4+5] = 0xff;
    // led_data[NUM_LED*4+6] = 0xff;
    // led_data[NUM_LED*4+7] = 0xff;


    let mut i: usize = 0;
    loop {
        led_data[4+i*4] = 0xff;
        led_data[4+i*4+1] = 0xff;
        let im1: usize = if i == 0 {
            (NUM_LED-1) as usize
        } else {
            ((i-1) % NUM_LED) as usize
        };

        led_data[4+im1%NUM_LED*4+0] = 0x70;
        led_data[4+im1*4+1] = 0x00;
        let _ = spi.write(&led_data);
        //delay.delay_ms(1);
        delay.delay_us(500);
        i = (i+1) % NUM_LED;
        // info!("off!");
        // led_pin.set_low().unwrap();
        // let _ = spi.write(&led_data);
        // delay.delay_ms(500);
    }
}

// End of file
