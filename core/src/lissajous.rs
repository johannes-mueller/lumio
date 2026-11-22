use crate::{
    conf::*, interface::Interface, led::{BLACK, DARK_WHITE}, math8::{iscale16, sat8}, player::{Show, StepResult}, signals::SineOsc, usb_writeln,
};

use core::f32::consts::PI;



pub struct Lissajous {
    sine_y: SineOsc,
    sine_phi: SineOsc,

    ampl_y: i16,
    ampl_phi: i16,

    center_y: i16,
    center_phi: i16,

    phase_phi: f32,
}


impl Lissajous {
    pub fn new() -> Self {
        let phase_phi = 0.25;
        let mut sine_y = SineOsc::new(1.0 / 20.0);
        let sine_phi = SineOsc::new(1.0 / 10.01);

        let phase_iterations = (phase_phi * sine_y.frames_per_wave()) as u32;
        for _ in 0..phase_iterations {
            sine_y.update();
        }

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
        "StaticWaveShow"
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {}

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {
        iface.led_strip().black();

        let iterations = self.sine_y.frames_per_wave().max(self.sine_phi.frames_per_wave()) as u32;

        for _ in 0..iterations {
            let pos_y = iscale16(self.sine_y.update() as i16, self.ampl_y) + self.center_y;
            let pos_phi = iscale16(self.sine_phi.update() as i16, self.ampl_phi) + self.center_phi;

            usb_writeln!(iface, "{} {} {} {}", pos_y, pos_phi, self.center_y, self.center_phi);

            let pos = (pos_y as isize + STRIP_LENGTH as isize * pos_phi as isize);

            iface.led_strip().set_led(pos, DARK_WHITE);
        }

        iface.write_spi();

        StepResult::Running
    }
}
