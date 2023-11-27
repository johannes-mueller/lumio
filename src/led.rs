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
    pub current: Color,
    pub target: Color,
    pub decay: f32
}

impl Led {

}
