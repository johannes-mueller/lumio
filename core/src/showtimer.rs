use crate::button::{Button, ButtonState};
use crate::conf::AUTO_SHOW_DELAY;
use crate::platform::Micros;

/// Drives the "next show" behavior: a short press advances, a long press
/// toggles auto-advance, and while auto-advance is on the player advances by
/// itself after [`AUTO_SHOW_DELAY`].
pub struct ShowTimer {
    auto_show: bool,
    button: Button,
    time_stamp: Micros,
}

impl ShowTimer {
    pub fn new(now: Micros) -> ShowTimer {
        ShowTimer {
            auto_show: true,
            button: Button::new(),
            time_stamp: now,
        }
    }

    pub fn auto_show(&self) -> bool {
        self.auto_show
    }

    /// Returns whether the player should advance to the next show.
    pub fn update(&mut self, pressed: bool, now: Micros) -> bool {
        let mut do_next = false;
        match self.button.update(pressed, now) {
            ButtonState::ShortPressed => {
                do_next = true;
                self.time_stamp = now;
            }
            ButtonState::LongPressed => {
                self.auto_show = !self.auto_show;
                self.time_stamp = now;
            }
            _ => {}
        }
        if self.auto_show && now - self.time_stamp > AUTO_SHOW_DELAY {
            self.time_stamp = now;
            do_next = true;
        }
        do_next
    }
}
