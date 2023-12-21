use embedded_hal::digital::v2::InputPin;
use rp_pico::hal::{gpio::{Pin, PinId, SioInput, FunctionSio, PullUp}, Timer};

#[derive(PartialEq, Clone, Copy)]
pub enum ButtonState {
    Up,
    Down,
    ShortPressed,
    LongPressed
}

pub struct Button<P: PinId> {
    pin: Pin<P, FunctionSio<SioInput>, PullUp>,
    press_time: Option<u32>,
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
                        Some(timer.get_counter_low())
                    } else {
                        self.press_time
                    },
                    ButtonState::Down
                ),
                |past| {
                    if timer.get_counter_low() - past > 2000000 && self.state != ButtonState::LongPressed {
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
