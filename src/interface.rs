use rp_pico::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    spi::{Spi, Enabled},
    gpio::{
        self,
        Pin,
        Pins,
        FunctionSioOutput,
        FunctionSpi,
        PullDown,
    },
    Timer,
    timer::Instant,
    watchdog::Watchdog,
};

use embedded_hal::{
    spi::MODE_0, digital::v2::OutputPin,
    blocking::spi::Write
};
use fugit::RateExtU32;

use crate::ledstrip::LEDStrip;
use crate::showtimer::ShowTimer;
use crate::button::{Button, ButtonState};
use crate::random::Random;

type ButtonPin1 = gpio::bank0::Gpio21;
type ButtonPin2 = gpio::bank0::Gpio20;
type LedPin1 = gpio::bank0::Gpio10;
type LedPin2 = gpio::bank0::Gpio11;

type SCLK0 = Pin<gpio::bank0::Gpio6, FunctionSpi, PullDown>;
type MOSI0 = Pin<gpio::bank0::Gpio7, FunctionSpi, PullDown>;

type Spi0Pinout = (MOSI0, SCLK0);

type SCLK1 = Pin<gpio::bank0::Gpio14, FunctionSpi, PullDown>;
type MOSI1 = Pin<gpio::bank0::Gpio15, FunctionSpi, PullDown>;

type Spi1Pinout = (MOSI1, SCLK1);


pub struct Peripherals {
    timer: Option<Timer>,
}
impl Peripherals {
    pub fn get_time(&mut self) -> Instant {
        self.timer.map_or(Instant::from_ticks(0), |t| t.get_counter())
    }
}

pub static mut PERIPHERALS: Peripherals = Peripherals {
    timer: None
};

pub struct Interface {
    led_strip: LEDStrip,
    showtimer: ShowTimer<ButtonPin1, LedPin1>,
    button: Button<ButtonPin2>,
    led_pin: Pin<LedPin2, FunctionSioOutput, PullDown>,
    random: Random,
    spi0: Spi<Enabled, pac::SPI0, Spi0Pinout, 8>,
    spi1: Spi<Enabled, pac::SPI1, Spi1Pinout, 8>,
    delay: cortex_m::delay::Delay
}

impl Interface {

    pub fn get_time() -> Instant {
        unsafe { PERIPHERALS.get_time() }
    }
    pub fn new() -> Interface {

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

        unsafe {
            PERIPHERALS.timer = Some(Timer::new(pac.TIMER, &mut pac.RESETS, &clocks));
        };
        let button_1 = Button::new(pins.gpio21.into_pull_up_input());
        let button_2 = Button::new(pins.gpio20.into_pull_up_input());
        let led_1_pin = pins.gpio10.into_push_pull_output();
        let led_2_pin = pins.gpio11.into_push_pull_output();

        let sclk = pins.gpio6.into_function::<FunctionSpi>();
        let mosi = pins.gpio7.into_function::<FunctionSpi>();

        let spi_device = pac.SPI0;
        let spi_pin_layout = (mosi, sclk);

        let spi0 = Spi::<_, _, _, 8>::new(spi_device, spi_pin_layout)
            .init(&mut pac.RESETS, 450_000_000u32.Hz(), 8_000_000u32.Hz(), MODE_0);

        let sclk = pins.gpio14.into_function::<FunctionSpi>();
        let mosi = pins.gpio15.into_function::<FunctionSpi>();

        let spi_device = pac.SPI1;
        let spi_pin_layout = (mosi, sclk);

        let spi1 = Spi::<_, _, _, 8>::new(spi_device, spi_pin_layout)
            .init(&mut pac.RESETS, 450_000_000u32.Hz(), 8_000_000u32.Hz(), MODE_0);

        Interface {
            led_strip: LEDStrip::new(),
            showtimer: ShowTimer::new(button_1, led_1_pin),
            button: button_2,
            led_pin: led_2_pin,
            random: Random::new(423434859),
            spi0, spi1,
            delay: cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz())
        }
    }

    pub fn led_strip(&mut self) -> &mut LEDStrip { &mut self.led_strip }
    pub fn random(&mut self) -> &mut Random { &mut self.random }
    pub fn do_next(&mut self) -> bool { self.showtimer.do_next() }
    pub fn button_state(&mut self) -> ButtonState { self.button.state() }
    pub fn led_on(&mut self) {
        let _ = self.led_pin.set_high();
    }
    pub fn led_off(&mut self) {
        let _ = self.led_pin.set_low();
    }
    pub fn write_spi(&mut self) {
        self.led_strip.process();
        let _ = self.spi0.write(self.led_strip.dump_0());
        let _ = self.spi1.write(self.led_strip.dump_1());
    }

    pub fn delay_ms(&mut self, delay: u32) {
        self.delay.delay_ms(delay);
    }


}
