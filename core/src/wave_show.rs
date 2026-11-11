use crate::{
    conf::*, interface::Interface, led::{Color, WHITE}, math8::isqrt8, player::{Show, StepResult}, sine::{Envelope, Sine}
};

const INTERSTEPS: usize = 10;
const RESOLUTION: usize = INTERSTEPS * STRIP_LENGTH;


struct ColorDiff {
    r: isize,
    g: isize,
    b: isize,
}

struct Position {
    strip: usize,
    alt: usize,
}

impl Position {
    pub fn new(strip: usize, alt: usize) -> Position {
        Position { strip, alt }
    }

    pub fn led_pos(&self) -> usize {
        self.alt + self.strip * STRIP_LENGTH
    }
}

const NEUTRAL: ColorDiff = ColorDiff { r: 0, g: 0, b: 0 };

struct WavePropagator {
    curr: [i8; RESOLUTION],
    prev: [i8; RESOLUTION],

    stimulus: Option<i8>
}

impl WavePropagator {
    pub fn new() -> WavePropagator {
        let curr = core::array::from_fn(|_| 0);
        let prev = core::array::from_fn(|_| 0);

        WavePropagator { curr, prev, stimulus: None }
    }

    pub fn stimulate(&mut self, signal: i8) {
        self.stimulus = Some(signal);
    }

    pub fn process(&mut self, buffer: &mut [i8; RESOLUTION]) {
        match self.stimulus {
            Some(signal) => self.curr[0] = signal,
            None => {}
        }
        self.stimulus = None;
        for i in 1..(RESOLUTION-1) {
            let prev = self.prev[i] as i16;
            let curr = self.curr[i] as i16;
            let curr_right = self.curr[i+1] as i16;
            let curr_left = self.curr[i-1] as i16;
            let next = 2 * curr - prev + (curr_right - 2 * curr + curr_left);

            buffer[i] = next as i8; //.min(127).max(-127) as i8;
        }
        buffer[RESOLUTION-1] = self.curr[RESOLUTION-2];

        for i in 0..RESOLUTION {
            self.prev[i] = self.curr[i];
            self.curr[i] = buffer[i];
        }
    }
}


pub struct Wave {
    sine: Sine,
    envelope: Envelope,
    hue: f32,

    position: Position,
    propagator: WavePropagator,
    val: i8,
}


impl Wave {
    pub fn new(wave_length: u8, speed: u8, hue: f32) -> Wave {
        let wave_length6 = (wave_length as u32) << 6;
        let speed6 = (speed as u32) << 3;

        let frequency = (speed6 / wave_length6) >> 6;
        let elastic = (frequency * frequency) as isize;

        let position = Position::new(0, 0);

        Wave {
            sine: Sine::new(64, 1, 64),
            envelope: Envelope::new(1),
            hue, position, propagator: WavePropagator::new(), val: 64}
    }

    fn process(&mut self, led_diff: &mut [ColorDiff; NUM_LED], buffer: &mut [i8; RESOLUTION]) {
        for _ in 0..10 {
            let signal = self.envelope.process(self.sine.process() as i8);
            self.propagator.stimulate(signal);
            self.propagator.process(buffer);
            self.val = 0; //(self.val - 1).max(0);

        }

        for i in 0..STRIP_LENGTH {
            let pos = self.position.led_pos();
            led_diff[pos+i].r = buffer[i * INTERSTEPS] as isize;
        }
    }
}


pub struct WaveShow {
    wave: Wave,
    color_diff: [ColorDiff; NUM_LED],
    buffer: [i8; RESOLUTION],
}

impl WaveShow {
    pub fn new() -> WaveShow {
        let color_diff = core::array::from_fn(|_| NEUTRAL);
        let buffer = core::array::from_fn(|_| 0);

        WaveShow {wave: Wave::new(6, 2, 1.0), color_diff, buffer}
    }
}

impl Show for WaveShow {
    fn title(&self) -> &'static str {
        "WaveShow"
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {

    }

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {
        iface.led_strip().black();
        self.wave.process(&mut self.color_diff, &mut self.buffer);

        for i in 0..NUM_LED {
            let color = Color {
                r: (self.color_diff[i].r * 4 + 64) as u8,
                g: 0, //(self.color_diff[i].g + 127) as u8,
                b: 0, //(self.color_diff[i].b + 127) as u8,
            };
            iface.led_strip().set_led(i as isize, color);
        }

        iface.write_spi();

        StepResult::Running
    }
}
