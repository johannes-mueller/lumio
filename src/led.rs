use core::u16;

use rp_pico::hal::rom_data::float_funcs::float_to_uint;
use libm::fabsf;

use crate::math8::scale8;

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

pub const BLUE: Color = Color { r: 0, g: 0, b: 255 };
pub const GREEN: Color = Color { r: 0, g: 255, b: 0 };
pub const YELLOW: Color = Color { r: 255, g: 255, b: 0 };
pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };

impl Color {
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Color {
        let c = v * s;

        const N: f32 = 0.0;

        let h6 = h*6.;
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

    pub fn from_tempeature(temperature: u8) -> Color {
        let t192 = scale8(temperature, 191);
        let heatramp = (t192 & 0x3f) << 2;

        if t192 & 0x80 != 0 {
            return Color { r: 255, g: 255, b: heatramp }
        }
        if t192 & 0x40 != 0 {
            return Color { r: 255, g: heatramp, b: 0 }
        }
        Color { r: heatramp, g: 0, b: 0 }
    }
}

#[derive(Clone, Copy)]
pub struct Led {
    current: Color,
    target: Color,
    decay: u8
}

impl Led {
    pub fn new() -> Led {
        Led { current: BLACK, target: BLACK, decay: 0 }
    }

    pub fn set_color(&mut self, color: Color) {
        self.target = color;
        self.current = color;
        self.decay = 0;
    }

    pub fn set_target(&mut self, color: Color, decay: u8) {
        self.decay = decay;
        self.target = color;
    }

    pub fn step(&mut self) {
        self.current.r = decay(self.current.r, self.target.r, self.decay);
        self.current.g = decay(self.current.g, self.target.g, self.decay);
        self.current.b = decay(self.current.b, self.target.b, self.decay);
    }

    pub fn r(&self) -> u8 { self.current.r }
    pub fn g(&self) -> u8 { self.current.g }
    pub fn b(&self) -> u8 { self.current.b }

    pub fn is_off(&self) -> bool {
        self.r() + self.g() + self.b() == 0
    }
}


fn decay(current: u8, target: u8, decay: u8) -> u8 {
    if target == current {
        return target;
    }
    if target > current {
        current + (((target - current) as u16 * decay as u16) >> 8).max(1) as u8
    } else {
        current - (((current - target) as u16 * decay as u16) >> 8).max(1) as u8
    }
}


fn round(v: f32) -> u8 {
    if v >= 1.0 {
        return 255;
    }
    if v <= 0.0 {
        return 0;
    }
    float_to_uint(v * 255.0) as u8
}
