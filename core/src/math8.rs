
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


pub fn iscale8(a: i8, b: i8) -> i8 {
    ((a as i16 * b as i16) >> 7) as i8
}

pub fn iscale16(a: i16, b: i16) -> i16 {
    ((a as i32 * b as i32) >> 15) as i16
}


pub const I8_MIN: i32 = i8::MIN as i32;
pub const I8_MAX: i32 = i8::MAX as i32;
pub const I16_MIN: i32 = i16::MIN as i32;
pub const I16_MAX: i32 = i16::MAX as i32;

#[inline(always)]
pub const fn sat8(value: i32) -> i8 {
    if value > I8_MAX {
        i8::MAX
    } else if value < I8_MIN {
        i8::MIN
    } else {
        value as i8
    }
}

#[inline(always)]
pub const fn sat15(value: i32) -> i32 {
    if value > I16_MAX {
        I16_MAX
    } else if value < I16_MIN {
        I16_MIN
    } else {
        value
    }
}
