#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

pub const BLUE: Color = Color { r: 0, g: 0, b: 255 };
pub const GREEN: Color = Color { r: 0, g: 1, b: 255 };
pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };

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

    pub fn current_color(&self) -> Color {
        self.current
    }
}
