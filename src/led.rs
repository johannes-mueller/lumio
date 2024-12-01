use rp_pico::hal::rom_data::float_funcs::float_to_uint;
use libm::fabsf;

use crate::math8::{scale8,qadd8,qsub8};
use crate::random;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

pub const WHITE: Color = Color { r: 255, g: 255, b: 255 };
pub const RED: Color = Color { r: 255, g: 0, b : 0};
pub const BLUE: Color = Color { r: 0, g: 0, b: 255 };
pub const GREEN: Color = Color { r: 0, g: 255, b: 0 };
pub const YELLOW: Color = Color { r: 255, g: 255, b: 0 };
pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };
pub const DARK_BLUE: Color = Color { r: 0, g: 0, b: 16 };
pub const DARK_GREEN: Color = Color { r: 0, g: 16, b: 0 };


impl Color {
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Color {
        let c = v * s;

        const N: f32 = 0.0;

        let h6 = (h % 1.0) *6.;
        let x = c * (1. - fabsf(h6 % 2. - 1.));
        let (r, g, b) = match h6 {
            h if h >= 0.0 && h < 1.0 => (c, x, N),
            h if h >= 1.0 && h < 2.0 => (x, c, N),
            h if h >= 2.0 && h < 3.0 => (N, c, x),
            h if h >= 3.0 && h < 4.0 => (N, x, c),
            h if h >= 4.0 && h < 5.0 => (x, N, c),
            h if h >= 5.0 && h <= 6.0 => (c, N, x),
            _ => (N, N, N)
        };

        let m = v - c;
        Color {
            r: float_to_uint((r+m) * 255.0) as u8,
            g: float_to_uint((g+m) * 255.0) as u8,
            b: float_to_uint((b+m) * 255.0) as u8,
        }
    }

    pub fn brightness(&self) -> u8 {
        (self.r >> 2) + (self.g >> 2) + (self.g >> 2)
    }
}

#[derive(Clone, Copy)]
pub struct Led {
    current: Color,
    target: Color,
    decay: u8,
    flicker: u8,
    current_flicker: u8
}

impl Led {
    pub fn new() -> Led {
        Led {
            current: BLACK,
            target: BLACK,
            decay: 0,
            flicker: 0,
            current_flicker: 0
        }
    }

    pub fn set_color(&mut self, color: Color) {
        self.target = color;
        self.current = color;
        self.decay = 0;
        self.flicker = 0;
    }

    pub fn set_target(&mut self, color: Color, decay: u8) {
        self.decay = decay;
        self.target = color;
        self.flicker = 0;
    }

    pub fn set_color_flickering(&mut self, color: Color, flicker: u8) {
        self.set_color(color);
        self.flicker = flicker;
    }

    pub fn set_target_flickering(&mut self, color: Color, decay: u8, flicker: u8) {
        self.set_target(color, decay);
        self.flicker = flicker;
    }

    pub fn step(&mut self, random: &mut random::Random) {
        self.current.r = decay(self.current.r, self.target.r, self.decay);
        self.current.g = decay(self.current.g, self.target.g, self.decay);
        self.current.b = decay(self.current.b, self.target.b, self.decay);
        let is_dark = self.current.brightness() < self.target.brightness();
        if self.current == self.target || is_dark  && self.decay != 0 {
            self.flicker = 0;
        }

        if self.flicker != 0 {
            let brightness = self.current.brightness();
            let offset = 0xff - brightness >> 1;
            self.current_flicker = scale8(random.value8(), brightness) + offset;
        }
    }

    pub fn r(&self) -> u8 { self.current().r }
    pub fn g(&self) -> u8 { self.current().g }
    pub fn b(&self) -> u8 { self.current().b }

    pub fn current(&self) -> Color {
        self.maybe_flicker(self.current)
    }

    fn maybe_flicker(&self, color: Color) -> Color {
        if self.flicker == 0 {
            return color
        }

        let mut flickered = color;

        if self.current_flicker > 127 {
            let flicker = scale8(self.current_flicker - 127, self.flicker);
            flickered.r = qadd8(flickered.r,  flicker);
            flickered.g = qadd8(flickered.g,  flicker);
            flickered.b = qadd8(flickered.b,  flicker);
        } else {
            let flicker = scale8(127 - self.current_flicker, self.flicker);
            flickered.r = qsub8(flickered.r,  flicker);
            flickered.g = qsub8(flickered.g,  flicker);
            flickered.b = qsub8(flickered.b,  flicker);
        }
        flickered
    }
}


pub fn decay(current: u8, target: u8, decay: u8) -> u8 {
    if target == current {
        return target;
    }
    if target > current {
        current + (((target - current) as u16 * decay as u16) >> 8).max(1) as u8
    } else {
        current - (((current - target) as u16 * decay as u16) >> 8).max(1) as u8
    }
}
