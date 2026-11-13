

pub struct Sine {
    center: isize,
    current: isize,
    elastic: isize,
    speed: isize,
}

pub fn scale(a: isize, b: isize) -> isize {
    (a * b) >> 8
}

pub fn scale8(a: i8, b: i8) -> i8 {
    ((a as i16 * b as i16) >> 8) as i8
}

impl Sine {
    pub fn new(center: isize, elastic: isize, ampl: isize) -> Sine {
        Sine {
            center,
            elastic,
            current: (ampl + center) << 6,
            speed: 0,
        }
    }

    pub fn set_elastic(&mut self, v: isize) -> &mut Sine {
        self.elastic = v;
        self
    }

    pub fn process(&mut self) -> isize {
        let pos: isize = self.current >> 6;

        let accel = scale(self.elastic, pos - self.center) << 5;
        self.speed = ((self.speed << 5) - accel) >> 5;

        self.current = self.current + self.speed;

        pos
    }
}
