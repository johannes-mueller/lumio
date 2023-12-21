use embedded_hal::digital::v2::InputPin;
use rp_pico::hal::{gpio::{Pin, PinId, SioInput, FunctionSio, PullUp}, Timer};
use rp_pico::hal::timer::Instant;

use crate::conf::LONG_PRESS_TIME;

#[derive(PartialEq, Clone, Copy)]
pub enum ButtonState {
    Up,
    Down,
    ShortPressed,
    LongPressed
}

pub struct Button<P: PinId> {
    pin: Pin<P, FunctionSio<SioInput>, PullUp>,
    press_time: Option<Instant>,
    state: ButtonState
}

impl<P: PinId> Button<P> {
    pub fn new(pin: Pin<P, FunctionSio<SioInput>, PullUp>) -> Button<P> {
        Button {
            pin,
            press_time: None,
            state: ButtonState::Up
        }
    }

    pub fn state(&mut self, timer: &Timer) -> ButtonState {
        self.state = self.determine_state(timer);
        self.state
    }

    fn determine_state(&mut self, timer: &Timer) -> ButtonState {
        let (press_time, state) = if self.pin.is_low().unwrap() {
            self.press_time.map_or_else(
                || (
                    if self.state == ButtonState::Up {
                        Some(timer.get_counter())
                    } else {
                        self.press_time
                    },
                    ButtonState::Down
                ),
                |past| {
                    if timer.get_counter() - past > LONG_PRESS_TIME && self.state != ButtonState::LongPressed {
                        (None, ButtonState::LongPressed)
                    } else {
                        (self.press_time, ButtonState::Down)
                    }
                }
            )
        } else  {
            self.press_time.map_or_else(
                || (None, ButtonState::Up),
                |_| (None, ButtonState::ShortPressed)
            )
        };
        self.press_time = press_time;
        state
    }
}
