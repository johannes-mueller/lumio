use crate::{
    conf::*, interface::Interface, led::{Color, BLACK, GREEN}, math8::{iscale8, sat8}, player::{Show, StepResult}, signals::SineOsc, sine::{Sine},
};

use core::f32::consts::PI;



pub struct Lissajous {
    sine_y: Sine,
    sine_phi: Sine,

    ampl_y: i8,
    ampl_phi: i8,

    center_y: i8,
    center_phi: isize,

    phase_phi: f32,
    hue: f32,
}


impl Lissajous {
    pub fn new() -> Self {
        let phase_phi = 0.25;
        let sine_y = Sine::new(28, 302, 26);
        let sine_phi = Sine::new(0, 141, 12);

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
            center_phi: 0,

            phase_phi,
            hue: 0.0,
        }
    }
}


impl Show for Lissajous {
    fn title(&self) -> &'static str {
        "Lissajous"
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {}

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {

        iface.led_strip().black_decay(12);
        //let iterations = self.sine_y.frames_per_wave().max(self.sine_phi.frames_per_wave()) as u32;

        let pos_y = self.sine_y.process();
        let pos_phi = self.sine_phi.process();

        let pos = (pos_y as isize + STRIP_LENGTH as isize * pos_phi as isize);


        let strip_length = STRIP_LENGTH as isize;
        let start = pos_phi as isize * strip_length;
        let end = start + strip_length;

        // for black_pos in start..end {
        //     iface.led_strip().set_led_target(black_pos, BLACK, 16);
        // }

        iface.led_strip().set_led(pos, GREEN);

        iface.write_spi();

        StepResult::Running
    }
}
