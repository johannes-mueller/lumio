use rp_pico::hal::rom_data::float_funcs::float_to_uint;
use libm::fabsf;

#[derive(Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32
}

pub const BLUE: Color = Color { r: 0.0, g: 0.0, b: 1.0 };
pub const GREEN: Color = Color { r: 0.0, g: 1.0, b: 0.0 };
pub const YELLOW: Color = Color { r: 1.0, g: 1.0, b: 0.0 };
pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

impl Color {
    pub fn from_hsv(h: f32, v: f32, s: f32) -> Color {
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
        Color { r: r+m, g: g+m, b: b+m }
    }
}

#[derive(Clone, Copy)]
pub struct Led {
    current: Color,
    target: Color,
    decay: f32
}

impl Led {
    pub fn new() -> Led {
        Led { current: BLACK, target: BLACK, decay: 0.0 }
    }

    pub fn set_color(&mut self, color: Color) {
        self.target = color;
        self.current = color;
        self.decay = 0.0;
    }

    pub fn set_target(&mut self, color: Color, decay: f32) {
        self.decay = decay;//.max(0.0).min(1.0);
        self.target = color;
    }

    pub fn step(&mut self) {
        self.current.r += (self.target.r - self.current.r) * self.decay;
        self.current.g += (self.target.g - self.current.g) * self.decay;
        self.current.b += (self.target.b - self.current.b) * self.decay;
    }

    pub fn r(&self) -> u8 { round(self.current.r) }
    pub fn g(&self) -> u8 { round(self.current.g) }
    pub fn b(&self) -> u8 { round(self.current.b) }
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
