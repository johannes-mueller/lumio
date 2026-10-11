use crate::conf::LONG_PRESS_TIME;
use crate::platform::Micros;

#[derive(PartialEq, Clone, Copy)]
pub enum ButtonState {
    Up,
    Down,
    ShortPressed,
    LongPressed,
}

/// Debounced button state machine.
///
/// It is platform independent: feed it the raw pressed state and the current
/// time (see [`crate::interface::Interface::begin_tick`]).
#[derive(Clone, Copy)]
pub struct Button {
    press_time: Option<Micros>,
    state: ButtonState,
}

impl Button {
    pub fn new() -> Button {
        Button {
            press_time: None,
            state: ButtonState::Up,
        }
    }

    pub fn state(&self) -> ButtonState {
        self.state
    }

    /// Feed the raw pressed state and the current time; returns the debounced
    /// state.
    pub fn update(&mut self, pressed: bool, now: Micros) -> ButtonState {
        self.state = if pressed {
            match self.press_time {
                None => {
                    if self.state == ButtonState::Up {
                        self.press_time = Some(now);
                    }
                    ButtonState::Down
                }
                Some(past) => {
                    if now - past > LONG_PRESS_TIME && self.state != ButtonState::LongPressed {
                        self.press_time = None;
                        ButtonState::LongPressed
                    } else {
                        ButtonState::Down
                    }
                }
            }
        } else {
            let was_pressed = self.press_time.is_some();
            self.press_time = None;
            if was_pressed {
                ButtonState::ShortPressed
            } else {
                ButtonState::Up
            }
        };
        self.state
    }
}
