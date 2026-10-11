
pub fn scale8(a: u8, b: u8) -> u8 {
    ((a as u16 * b as u16) >> 8) as u8
}

pub fn qsub8(a: u8, b: u8) -> u8 {
    if a <= b {
        0u8
    } else {
        a - b
    }
}

pub fn qadd8(a: u8, b: u8) -> u8 {
    if 255 - b < a {
        return 255
    }
    a + b
}
