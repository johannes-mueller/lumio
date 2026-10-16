use crate::{
    conf::{STRIP_LENGTH, STRIP_NUM},
    huewave::HueWave,
    interface::Interface,
    led::WHITE,
    ledstrip::LEDStrip,
    player::{Show, StepResult},
};

pub struct Spiral {
    start_strip: usize,
    steps: usize,
}

impl Spiral {
    pub fn new(start_strip: usize) -> Spiral {
        Spiral {
            start_strip,
            steps: 0,
        }
    }

    pub fn reset(&mut self) {
        self.steps = 0;
    }

    pub fn swirl(&mut self) {
        self.steps = (self.steps + 1) % (STRIP_LENGTH * 2);
    }

    pub fn step(&mut self) {
        self.steps = STRIP_LENGTH;
        self.start_strip = (self.start_strip + STRIP_NUM - 1) % STRIP_NUM
    }

    pub fn process(&mut self, led_strip: &mut LEDStrip) {
        let start = (self.steps / STRIP_LENGTH) * (self.steps % STRIP_LENGTH);
        let mut strip = (self.start_strip + start) % STRIP_NUM;
        let target = self.steps.min(STRIP_LENGTH);
        for i in start..target {
            let pos = (strip * STRIP_LENGTH + i) as isize;
            led_strip.set_led(pos, WHITE);
            strip = (strip + 1) % STRIP_NUM;
        }
    }
}

#[derive(Clone, Copy)]
pub enum SpiralMode {
    Lift,
    Swirl,
}

pub struct HueSpiral {
    spiral: Spiral,
    huewave: HueWave,
    mode: SpiralMode,
}

impl HueSpiral {
    pub fn new_lift() -> HueSpiral {
        HueSpiral {
            spiral: Spiral::new(0),
            huewave: HueWave::new(),
            mode: SpiralMode::Lift,
        }
    }

    pub fn new_swirl() -> HueSpiral {
        HueSpiral {
            spiral: Spiral::new(0),
            huewave: HueWave::new(),
            mode: SpiralMode::Swirl,
        }
    }
}

impl Show for HueSpiral {
    fn title(&self) -> &'static str {
        match self.mode {
            SpiralMode::Lift => "Spiral (lift)",
            SpiralMode::Swirl => "Spiral (swirl)",
        }
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {
        self.spiral.reset();
    }

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {
        self.huewave.process(&mut iface.led_strip());
        self.spiral.process(&mut iface.led_strip());
        match self.mode {
            SpiralMode::Lift => self.spiral.step(),
            SpiralMode::Swirl => self.spiral.swirl(),
        }
        iface.write_spi();
        StepResult::Running
    }
}
