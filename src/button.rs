use embedded_hal::digital::v2::InputPin;
use rp_pico::hal::gpio::{Pin, PinId, SioInput, FunctionSio, PullUp};

pub struct Button<P: PinId> {
    pin: Pin<P, FunctionSio<SioInput>, PullUp>,
    pressed: bool
}

impl<P: PinId> Button<P> {
    pub fn new(pin: Pin<P, FunctionSio<SioInput>, PullUp>) -> Button<P> {
        Button { pin, pressed: false }
    }

    pub fn is_pressed(&mut self) -> bool {
        if self.pin.is_low().unwrap() {
            if !self.pressed {
                self.pressed = true;
                return true
            }
        } else {
            self.pressed = false;
        }
        false
    }
}
