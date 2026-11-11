
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


pub const fn isqrt8(n: u8) -> u8 {
    let mut n = n;
    let mut res: u8 = 0;
    let mut bit: u8 = 0x40;

    loop {
        let t = res + bit;
        if n >= t {
            n -= t;
            res = (res >> 1) + bit;
        } else {
            res >>= 1;
        }
        bit >>= 2;
        if bit == 0 {
            break;
        }
    }
    res
}
