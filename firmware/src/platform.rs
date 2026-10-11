//! The RP2040 hardware [`Platform`] implementation.

use embedded_hal::{
    blocking::spi::Write,
    digital::v2::{InputPin, OutputPin},
    spi::MODE_0,
};
use rp_pico::hal::fugit::RateExtU32;
use rp_pico::hal::{
    clocks::{init_clocks_and_plls, Clock},
    gpio::{self, FunctionSio, FunctionSpi, Pin, Pins, PullDown, PullUp, SioInput, SioOutput},
    pac,
    sio::Sio,
    spi::{Enabled, Spi},
    timer::Timer,
    watchdog::Watchdog,
};

use lumio_core::platform::{ButtonId, Micros, Platform, StatusLedId};

type ButtonPinAdvance = Pin<gpio::bank0::Gpio21, FunctionSio<SioInput>, PullUp>;
type ButtonPinAction = Pin<gpio::bank0::Gpio20, FunctionSio<SioInput>, PullUp>;
type LedPinAuto = Pin<gpio::bank0::Gpio10, FunctionSio<SioOutput>, PullDown>;
type LedPinActive = Pin<gpio::bank0::Gpio11, FunctionSio<SioOutput>, PullDown>;

type Spi0Pinout = (
    Pin<gpio::bank0::Gpio7, FunctionSpi, PullDown>,
    Pin<gpio::bank0::Gpio6, FunctionSpi, PullDown>,
);
type Spi1Pinout = (
    Pin<gpio::bank0::Gpio15, FunctionSpi, PullDown>,
    Pin<gpio::bank0::Gpio14, FunctionSpi, PullDown>,
);

pub struct FirmwarePlatform {
    timer: Timer,
    delay: cortex_m::delay::Delay,
    button_advance: ButtonPinAdvance,
    button_action: ButtonPinAction,
    led_auto: LedPinAuto,
    led_active: LedPinActive,
    spi0: Spi<Enabled, pac::SPI0, Spi0Pinout, 8>,
    spi1: Spi<Enabled, pac::SPI1, Spi1Pinout, 8>,
}

impl FirmwarePlatform {
    pub fn new() -> FirmwarePlatform {
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

        let pins = Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );

        let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

        let button_advance = pins.gpio21.into_pull_up_input();
        let button_action = pins.gpio20.into_pull_up_input();
        let led_auto = pins.gpio10.into_push_pull_output();
        let led_active = pins.gpio11.into_push_pull_output();

        let sclk = pins.gpio6.into_function::<FunctionSpi>();
        let mosi = pins.gpio7.into_function::<FunctionSpi>();
        let spi0 = Spi::<_, _, _, 8>::new(pac.SPI0, (mosi, sclk)).init(
            &mut pac.RESETS,
            450_000_000u32.Hz(),
            8_000_000u32.Hz(),
            MODE_0,
        );

        let sclk = pins.gpio14.into_function::<FunctionSpi>();
        let mosi = pins.gpio15.into_function::<FunctionSpi>();
        let spi1 = Spi::<_, _, _, 8>::new(pac.SPI1, (mosi, sclk)).init(
            &mut pac.RESETS,
            450_000_000u32.Hz(),
            8_000_000u32.Hz(),
            MODE_0,
        );

        FirmwarePlatform {
            timer,
            delay: cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz()),
            button_advance,
            button_action,
            led_auto,
            led_active,
            spi0,
            spi1,
        }
    }
}

impl Platform for FirmwarePlatform {
    fn now_micros(&self) -> Micros {
        self.timer.get_counter().ticks()
    }

    fn present(&mut self, a: &[u8], b: &[u8]) {
        let _ = self.spi0.write(a);
        let _ = self.spi1.write(b);
    }

    fn delay_ms(&mut self, ms: u32) {
        self.delay.delay_ms(ms);
    }

    fn raw_button(&self, id: ButtonId) -> bool {
        match id {
            ButtonId::Advance => self.button_advance.is_low().unwrap_or(false),
            ButtonId::Action => self.button_action.is_low().unwrap_or(false),
        }
    }

    fn set_status_led(&mut self, id: StatusLedId, on: bool) {
        match id {
            StatusLedId::Auto => {
                if on {
                    let _ = self.led_auto.set_high();
                } else {
                    let _ = self.led_auto.set_low();
                }
            }
            StatusLedId::Active => {
                if on {
                    let _ = self.led_active.set_high();
                } else {
                    let _ = self.led_active.set_low();
                }
            }
        }
    }
}
