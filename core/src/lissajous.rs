use crate::{
    conf::*, interface::Interface, led::{BLACK, DARK_WHITE}, math8::{iscale8, sat8}, player::{Show, StepResult}, signals::SineOsc, sine::{Sine},
};

use core::f32::consts::PI;



pub struct Lissajous {
    sine_y: Sine,
    sine_phi: Sine,

    ampl_y: i8,
    ampl_phi: i8,

    center_y: i8,
    center_phi: i8,

    phase_phi: f32,
}


impl Lissajous {
    pub fn new() -> Self {
        let phase_phi = 0.25;
        let mut sine_y = Sine::new(28, 1007, 30);
        let sine_phi = Sine::new(12, 998, 12);

        // let phase_iterations = (phase_phi * sine_y.frames_per_wave()) as u32;
        // for _ in 0..phase_iterations {
        //     sine_y.update();
        // }

        Lissajous {
            sine_y,
            sine_phi,

            ampl_y: 20,
            ampl_phi: 10,

            center_y: 30,
            center_phi: 12,

            phase_phi
        }
    }
}


impl Show for Lissajous {
    fn title(&self) -> &'static str {
        "Lissajous"
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {}

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {
        iface.led_strip().black();

        //let iterations = self.sine_y.frames_per_wave().max(self.sine_phi.frames_per_wave()) as u32;

        for _ in 0..(2*STRIP_NUM) {
            let pos_y = self.sine_y.process();
            let pos_phi = self.sine_phi.process();

            let pos = (pos_y as isize + STRIP_LENGTH as isize * pos_phi as isize);

            iface.led_strip().set_led(pos, DARK_WHITE);
        }

        iface.write_spi();

        StepResult::Running
    }
}
