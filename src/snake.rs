use crate::conf::STRIP_LENGTH;
use crate::ledstrip::LEDStrip;
use crate::led::{Color, BLACK};


#[derive(Clone, Copy)]
pub struct Snake {
    start_strip: usize,
    head_color: Color,
    tail_color: Color,
    step: usize,
    decay: u8,
    done: bool
}

impl Default for Snake {
    fn default() -> Snake {
        Snake { start_strip: 0, head_color: BLACK, tail_color: BLACK, step: 0, decay: 0, done: true }
    }
}

impl Snake {
    pub fn reset(&mut self, start_strip: usize, head_hue: f32, tail_hue_shift: f32) {
        self.start_strip = start_strip;
        self.head_color = Color::from_hsv(head_hue, 1.0, 1.0);
        let th = if head_hue > 1.0 - tail_hue_shift {
            head_hue - (1.0 - tail_hue_shift)
        } else {
            head_hue + tail_hue_shift
        };
        self.tail_color = Color::from_hsv(th, 1.0, 0.4);
        self.step = 0;
        self.decay = 64;
        self.done = false;
    }

    pub fn is_active(&self) -> bool {
        !self.done
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        if self.done {
            return
        }
        let pos = (STRIP_LENGTH*self.start_strip + self.step) as isize;
        if self.step < STRIP_LENGTH {
            led_strip.set_led(pos, self.head_color);
        }
        if self.step > 0 && self.step < STRIP_LENGTH + 1 {
            led_strip.set_led(pos-1, self.tail_color);
        }
        if self.step > 1 && self.step < STRIP_LENGTH + 2{
            led_strip.set_led_target(pos-2, BLACK, self.decay);
        }

        if self.step == STRIP_LENGTH + 1 {
            self.done = true;
        }
        self.step += 1;
    }
}
