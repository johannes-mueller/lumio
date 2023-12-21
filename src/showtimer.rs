use embedded_hal::digital::v2::OutputPin;
use rp_pico::hal::gpio::{PinId, Pin, PullDown, FunctionSio, SioOutput};
use rp_pico::hal::Timer;

use crate::button::{Button, ButtonState};

const AUTO_SHOW_DELAY: u32 = 3_000_000;

pub struct ShowTimer<'a, BP: PinId, LP: PinId> {
    auto_show: bool,
    button: Button<BP>,
    led_pin: Pin<LP, FunctionSio<SioOutput>, PullDown>,
    time_stamp: u32,
    timer: &'a Timer,
}

impl <'a, BP: PinId, LP: PinId> ShowTimer<'a, BP, LP> {
    pub fn new(button: Button<BP>, led_pin: Pin<LP, FunctionSio<SioOutput>, PullDown>, timer: &'a Timer) -> ShowTimer<'a, BP, LP> {
        let time_stamp = timer.get_counter_low();
        ShowTimer { auto_show: false, button, led_pin, time_stamp, timer }
    }

    pub fn do_next(&mut self) -> bool {
        let mut do_next = false;
        match self.button.state(self.timer) {
            ButtonState::ShortPressed => { do_next = true; },
            ButtonState::LongPressed => { self.auto_show = !self.auto_show },
            _ => {}
        }
        if self.auto_show {
            let _ = self.led_pin.set_high();
            let now = self.timer.get_counter_low();
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
