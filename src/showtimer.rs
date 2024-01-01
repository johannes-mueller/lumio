use embedded_hal::digital::v2::OutputPin;
use rp_pico::hal::gpio::{PinId, Pin, PullDown, FunctionSio, SioOutput};
use rp_pico::hal::Timer;
use rp_pico::hal::timer::Instant;

use crate::button::{Button, ButtonState};
use crate::conf::AUTO_SHOW_DELAY;

pub struct ShowTimer<'a, BP: PinId, LP: PinId> {
    auto_show: bool,
    button: Button<'a, BP>,
    led_pin: Pin<LP, FunctionSio<SioOutput>, PullDown>,
    time_stamp: Instant,
    timer: &'a Timer,
}

impl <'a, BP: PinId, LP: PinId> ShowTimer<'a, BP, LP> {
    pub fn new(button: Button<'a, BP>, led_pin: Pin<LP, FunctionSio<SioOutput>, PullDown>, timer: &'a Timer) -> ShowTimer<'a, BP, LP> {
        let time_stamp = timer.get_counter();
        ShowTimer { auto_show: true, button, led_pin, time_stamp, timer }
    }

    pub fn do_next(&mut self) -> bool {
        let mut do_next = false;
        match self.button.state() {
            ButtonState::ShortPressed => {
                do_next = true;
                self.time_stamp = self.timer.get_counter();
            },
            ButtonState::LongPressed => {
                self.auto_show = !self.auto_show;
                self.time_stamp = self.timer.get_counter();
            },
            _ => {}
        }
        if self.auto_show {
            let _ = self.led_pin.set_high();
            let now = self.timer.get_counter();
            if now - self.time_stamp > AUTO_SHOW_DELAY {
                self.time_stamp = now;
                do_next = true;
            }
        } else {
            let _ = self.led_pin.set_low();
        }
        do_next
    }
}
