use crate::conf::*;
use crate::interface::Interface;
use crate::led::{Color, BLACK};
use crate::ledstrip::LEDStrip;
use crate::player::{Show, StepResult};
use crate::button::ButtonState;

#[derive(Clone, Copy)]
pub struct Snake {
    start_strip: usize,
    head_color: Color,
    tail_color: Color,
    step: usize,
    decay: u8,
    done: bool,
}

impl Default for Snake {
    fn default() -> Snake {
        Snake {
            start_strip: 0,
            head_color: BLACK,
            tail_color: BLACK,
            step: 0,
            decay: 0,
            done: true,
        }
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

    pub fn is_done(&self) -> bool {
        self.done
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        if self.done {
            return;
        }
        let pos = (STRIP_LENGTH * self.start_strip + self.step) as isize;
        if self.step < STRIP_LENGTH {
            led_strip.set_led(pos, self.head_color);
        }
        if self.step > 0 && self.step < STRIP_LENGTH + 1 {
            led_strip.set_led(pos - 1, self.tail_color);
        }
        if self.step > 1 && self.step < STRIP_LENGTH + 2 {
            led_strip.set_led_target(pos - 2, BLACK, self.decay);
        }

        if self.step == STRIP_LENGTH + 1 {
            self.done = true;
        }
        self.step += 1;
    }
}

pub struct SnakeShow {
    constant_snakes: [Snake; STRIP_NUM],
    random_snakes: [Snake; STRIP_NUM],
    running: bool,
    step: usize,
}

impl SnakeShow {
    pub fn new() -> SnakeShow {
        SnakeShow {
            constant_snakes: [Snake::default(); STRIP_NUM],
            random_snakes: [Snake::default(); STRIP_NUM],
            running: false,
            step: 0,
        }
    }
}

impl Show for SnakeShow {
    fn title(&self) -> &'static str {
        "Snake"
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {
        self.running = false;
        self.step = 0;
    }

    fn step(&mut self, interface: &mut Interface<'_>) -> StepResult {
        if !self.running {
            for i in 0..STRIP_NUM {
                self.constant_snakes[i].reset(i, interface.random().value(), 60. / 360.);
            }
        }
        if self.constant_snakes.iter().all(|sn| sn.is_done()) {
            interface.led_off();
            self.running = false;
        }
        if (interface.button_state() == ButtonState::ShortPressed && !self.running) || self.step == 0
        {
            interface.led_on();
            self.running = true;
        }
        for sn in self.constant_snakes.iter_mut() {
            sn.process(&mut interface.led_strip());
        }
        interface.write_spi();
        if interface.random().value8() < SNAKE_PROB {
            let cand = interface.random().value32(STRIP_NUM as u32) as usize;
            if self.random_snakes[cand].is_done() {
                self.random_snakes[cand].reset(cand, interface.random().value(), 60. / 360.);
            }
        }
        for sn in self.random_snakes.iter_mut() {
            sn.process(&mut interface.led_strip());
        }

        self.step = (self.step + 1) % 1024;
        StepResult::Running
    }
}
