use embedded_hal::digital::v2::OutputPin;
use rp_pico::hal::gpio::{PinId, Pin, PullDown, FunctionSio, SioOutput};
use rp_pico::hal::timer::Instant;

use crate::Interface;
use crate::button::{Button, ButtonState};
use crate::conf::AUTO_SHOW_DELAY;

pub struct ShowTimer<BP: PinId, LP: PinId> {
    auto_show: bool,
    button: Button<BP>,
    led_pin: Pin<LP, FunctionSio<SioOutput>, PullDown>,
    time_stamp: Instant,
}

impl <BP: PinId, LP: PinId> ShowTimer<BP, LP> {
    pub fn new(
        button: Button<BP>, led_pin: Pin<LP, FunctionSio<SioOutput>, PullDown>
    ) -> ShowTimer<BP, LP> {
        let time_stamp = Interface::get_time();
        ShowTimer { auto_show: true, button, led_pin, time_stamp }
    }

    pub fn do_next(&mut self) -> bool {
        let mut do_next = false;
        match self.button.state() {
            ButtonState::ShortPressed => {
                do_next = true;
                self.time_stamp = Interface::get_time();
            },
            ButtonState::LongPressed => {
                self.auto_show = !self.auto_show;
                self.time_stamp = Interface::get_time();
            },
            _ => {}
        }
        if self.auto_show {
            let _ = self.led_pin.set_high();
            let now = Interface::get_time();
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
