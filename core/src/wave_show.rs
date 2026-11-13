use crate::{
    conf::*, interface::Interface, led::{Color, WHITE}, math8::isqrt8, player::{Show, StepResult}
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


#[inline(always)]
pub const fn sat8(value: i32) -> i8 {
    if value > 127 {
        127
    } else if value < -127 {
        -127
    } else {
        value as i8
    }
}

/// int32 -> Q15, clamped to [-32768, 32767].
#[inline(always)]
pub const fn sat15(value: i32) -> i32 {
    if value > 32767 {
        32767
    } else if value < -32768 {
        -32768
    } else {
        value
    }
}

pub const Q15_MAX: i32 = 32767;

// ===========================================================================
// 2. Wave8 -- leapfrog, sigma = 1
// ===========================================================================

/// Boundary condition at the right-hand end.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Boundary {
    /// Mur, 1st order; exact (reflection-free) when sigma = 1
    Mur,
    /// fixed end, u = 0
    Dirichlet,
    /// free end, du/ds = 0
    Neumann,
}

/// 1-D wave, sigma = c*dt/ds = 1.
///
///     field_now[j]  <=  field_now[j-1] + field_now[j+1] - field_before[j]
///
/// Two int8 arrays = 2*N bytes of total state. Updated in place, walking
/// backwards so the still-unmodified right neighbour is read.
pub struct Wave8<const N: usize> {
    /// current time level u^n
    field_now: [i8; N],
    /// previous time level u^(n-1), supplies the u_tt term
    field_before: [i8; N],
    boundary: Boundary,
    steps_done: u32,
}

impl<const N: usize> Wave8<N> {
    pub fn new(boundary: Boundary) -> Self {
        assert!(N >= 3, "Wave8: N >= 3 required");
        Self {
            field_now: [0; N],
            field_before: [0; N],
            boundary,
            steps_done: 0,
        }
    }

    /// Reset to the resting state; the source history is not restored.
    pub fn reset(&mut self) -> &mut Self {
        self.field_now = [0; N];
        self.field_before = [0; N];
        self.steps_done = 0;
        self
    }

    /// Current time level u^n.
    #[inline(always)]
    pub fn field(&self) -> &[i8; N] {
        &self.field_now
    }

    /// Previous time level u^(n-1).
    #[inline(always)]
    pub fn previous_field(&self) -> &[i8; N] {
        &self.field_before
    }

    #[inline(always)]
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }

    pub fn set_boundary(&mut self, boundary: Boundary) {
        self.boundary = boundary;
    }

    /// Number of steps computed so far.
    #[inline(always)]
    pub fn steps_done(&self) -> u32 {
        self.steps_done
    }

    /// One time step. `left_edge_next` = u(0, t_{n+1}).
    /// Returns the new time level u^(n+1); do not mutate the slice.
    #[inline]
    pub fn step(&mut self, left_edge_next: i8) -> &[i8; N] {
        // Save old-level edge values before anything is overwritten:
        // right_edge_before = u[N-1]^n, inner_edge_before = u[N-2]^n
        let right_edge_before = self.field_now[N - 1];
        let inner_edge_before = self.field_now[N - 2];

        // Walk backwards and cache the old value of u[j] before writing it.
        // Reading an already-overwritten neighbour yields an index error that
        // looks exactly like numerical dispersion.
        let mut right_neighbour = right_edge_before; // old u[j+1]^n
        let mut j = N - 2;
        while j >= 1 {
            let center_before = self.field_now[j]; // old u[j]^n, before writing
            let new_value_raw = self.field_now[j - 1] as i32 + right_neighbour as i32
                - self.field_before[j] as i32; // |new_value_raw| <= 3*127

            self.field_before[j] = center_before; // old u^n becomes u^(n-1)
            self.field_now[j] = sat8(new_value_raw);

            right_neighbour = center_before; // for the next (leftward) j
            j -= 1;
        }

        // Left edge: Dirichlet condition, i.e. the source.
        self.field_before[0] = self.field_now[0];
        self.field_now[0] = left_edge_next;

        // Right edge: closure per boundary type.
        self.field_before[N - 1] = right_edge_before;
        self.field_now[N - 1] = match self.boundary {
            // r = 0 when sigma = 1: exact, one-sample memory only
            Boundary::Mur => inner_edge_before,
            // mirror image of the interior node
            Boundary::Neumann => sat8(2 * inner_edge_before as i32 - right_edge_before as i32),
            Boundary::Dirichlet => 0,
        };

        self.steps_done += 1;
        &self.field_now
    }
}

// ===========================================================================
// 3. SineOsc -- sin(omega*t) without sin
// ===========================================================================

/// Quadrature oscillator ("magic circle").
///
///     sin_new = sin_value + phase_step*cos_value
///     cos_new = cos_value - phase_step*sin_new
///
/// The system matrix has determinant exactly 1, so the generated frequency is
/// exactly omega*dt per step. `cos_value` starts at Q15_MAX*cos(omega*dt/2)
/// so that the amplitude of `sin_value` reaches Q15_MAX.
pub struct SineOsc {
    /// Phase increment in Q15: 2*sin(omega*dt/2) * 2^15
    phase_step: i32,
    /// sin(omega*t) in Q15
    sin_value: i32,
    /// cos(omega*t) in Q15, needed only by the recursion
    cos_value: i32,
    /// Initial value of cos_value, so reset() is exact
    initial_cos: i32,
}

impl SineOsc {
    /// `omega_dt` = omega * dt. sin/cos as Taylor series up to x^5, no libm.
    pub fn new(omega_dt: f32) -> Self {
        let half_angle = 0.5 * omega_dt;
        assert!(
            half_angle > 0.0 && half_angle < 0.5,
            "SineOsc: 0 < omega*dt/2 < 0.5 required"
        );

        let half_angle_sq = half_angle * half_angle;
        // 2*sin(x) = 2x*(1 - x^2/6 + x^4/120)
        let phase_step_f =
            2.0 * half_angle * (1.0 - half_angle_sq / 6.0 + half_angle_sq * half_angle_sq / 120.0);
        // cos(x) = 1 - x^2/2 + x^4/24
        let cos_half_angle =
            1.0 - 0.5 * half_angle_sq + half_angle_sq * half_angle_sq / 24.0;

        let initial_cos = (Q15_MAX as f32 * cos_half_angle + 0.5) as i32;
        Self {
            phase_step: (phase_step_f * 32768.0 + 0.5) as i32,
            sin_value: 0,
            cos_value: initial_cos,
            initial_cos,
        }
    }

    /// Build from precomputed design constants (float-free at startup).
    pub const fn from_phase_step(phase_step: i32, initial_cos: i32) -> Self {
        Self {
            phase_step,
            sin_value: 0,
            cos_value: initial_cos,
            initial_cos,
        }
    }

    /// Restart the oscillator in phase and amplitude.
    pub fn reset(&mut self) -> &mut Self {
        self.sin_value = 0;
        self.cos_value = self.initial_cos;
        self
    }

    /// Returns sin(omega * n * dt) in Q15. First call yields t = dt.
    #[inline(always)]
    pub fn update(&mut self) -> i32 {
        // +16384 rounds instead of truncating, halving the amplitude drift
        let sin_new = self.sin_value + ((self.phase_step * self.cos_value + 16384) >> 15);
        let cos_new = self.cos_value - ((self.phase_step * sin_new + 16384) >> 15);
        self.sin_value = sin_new;
        self.cos_value = cos_new;
        sin_new
    }
}

// ===========================================================================
// 4. BellEnv -- exp(-((t-t0)/sigma)^2) without exp
// ===========================================================================

/// Lower bound of log2(envelope): 2^-10 ~ 0.001, below 1 LSB in int8.
const ENVELOPE_LOG2_FLOOR: i32 = 10;
const INV_LN2: f32 = 1.442_695_040_888_963_4;

/// Gaussian envelope evaluated in the log2 domain.
///
///     log2(envelope) = -C*(n - t0/dt)^2 ,  C = (dt/sigma)^2 / ln2
///
/// That is a parabola, i.e. a constant second difference, so a step costs two
/// additions:
///
///     log2_envelope += log2_slope ;  log2_slope += log2_curvature
///
/// The inverse map 2^x uses a cubic polynomial on [0,1) plus a shift by 2^-k.
/// No lookup table, no exp.
pub struct BellEnv {
    /// log2(envelope) in Q(frac_bits), always <= 0
    log2_envelope: i32,
    /// First difference of log2_envelope
    log2_slope: i32,
    /// Second difference, constant and negative
    log2_curvature: i32,
    /// Lower bound for log2_envelope
    log2_floor: i32,
    /// Fractional bits of log2_envelope
    frac_bits: u32,
    /// Initial value of log2_slope, so reset() is exact
    initial_slope: i32,
    /// True once log2_floor is reached (envelope frozen)
    finished: bool,
}

impl BellEnv {
    /// `dt_over_sigma` = dt / sigma.
    pub fn new(dt_over_sigma: f32) -> Self {
        assert!(
            dt_over_sigma > 0.0 && dt_over_sigma < 3.0,
            "BellEnv: 0 < dt/sigma < 3 required"
        );
        let curvature = dt_over_sigma * dt_over_sigma * INV_LN2;

        // Store C in Q20, then shift left until C*2^frac_bits >= 4096 so the
        // fractional part inside exp2() keeps at least 12 bits of resolution.
        let mut curvature_q20 = (curvature * 1_048_576.0 + 0.5) as i32;
        assert!(curvature_q20 > 0, "BellEnv: C too small");
        let mut frac_bits = 20u32;
        while curvature_q20 < 4096 && frac_bits < 27 {
            curvature_q20 <<= 1;
            frac_bits += 1;
        }
        // frac_bits <= 27 keeps ENVELOPE_LOG2_FLOOR << frac_bits inside i32.

        let initial_slope = -curvature_q20;
        Self {
            log2_envelope: 0,
            log2_slope: initial_slope,
            log2_curvature: -(curvature_q20 << 1),
            log2_floor: -(ENVELOPE_LOG2_FLOOR << frac_bits),
            frac_bits,
            initial_slope,
            finished: false,
        }
    }

    /// Build from precomputed design constants (float-free at startup).
    pub const fn from_raw(initial_slope: i32, log2_curvature: i32, frac_bits: u32) -> Self {
        Self {
            log2_envelope: 0,
            log2_slope: initial_slope,
            log2_curvature,
            log2_floor: -(ENVELOPE_LOG2_FLOOR << frac_bits),
            frac_bits,
            initial_slope,
            finished: false,
        }
    }

    /// Restart at the peak of the parabola.
    pub fn reset(&mut self) -> &mut Self {
        self.log2_envelope = 0;
        self.log2_slope = self.initial_slope;
        self.finished = false;
        self
    }

    /// Returns exp(-((t-t0)/sigma)^2) in Q15. First call yields t = dt.
    #[inline(always)]
    pub fn update(&mut self) -> i32 {
        if self.finished {
            return 0;
        }
        self.log2_envelope += self.log2_slope;
        self.log2_slope += self.log2_curvature;
        if self.log2_envelope <= self.log2_floor {
            // Freeze: without this the parabola climbs again and produces a
            // second (mathematically valid, visually wrong) pulse near 2*t0.
            self.log2_envelope = self.log2_floor;
            self.finished = true;
            return 0;
        }
        self.exp2()
    }

    /// 2^(log2_envelope / 2^frac_bits) in Q15, log2_envelope <= 0.
    #[inline(always)]
    fn exp2(&self) -> i32 {
        // Arithmetic shift = floor, so the fraction below is always in [0,1)
        let exponent_int = self.log2_envelope >> self.frac_bits;
        if exponent_int >= 0 {
            return Q15_MAX; // exponent >= 0 means envelope >= 1: saturate
        }
        if exponent_int < -15 {
            return 0;
        }
        // Fractional part in Q10, range [0, 1024)
        let fraction_q10 =
            (self.log2_envelope - (exponent_int << self.frac_bits)) >> (self.frac_bits - 10);

        // 2^x on [0,1) in Q10, error < 0.05 %:
        // p(x) = 1 + x*(0.69556 + x*(0.22607 + x*0.07837))
        let poly_stage_3 = ((fraction_q10 * 80) >> 10) + 232;
        let poly_stage_2 = ((fraction_q10 * poly_stage_3) >> 10) + 712;
        let poly_stage_1 = ((fraction_q10 * poly_stage_2) >> 10) + 1024; // Q10, 1024..2048

        // Q10 -> Q15, then scale by 2^exponent_int (a right shift)
        sat15((poly_stage_1 << 5) >> (-exponent_int) as u32)
    }
}

// ===========================================================================
// 5. Exciter
// ===========================================================================

/// f(t) = 127 * sin(omega*t) * exp(-((t-t0)/sigma)^2), output as int8.
pub struct Exciter {
    /// sine carrier, produces sin(omega*t)
    carrier: SineOsc,
    /// Gaussian envelope, produces exp(-((t-t0)/sigma)^2)
    envelope: BellEnv,
    steps_done: u32,
}

impl Exciter {
    /// All arguments in SI units: fs in 1/s, omega in rad/s, sigma in s.
    /// t0 does not appear explicitly; it is implied by the step counter and
    /// must match the phase reference of the caller.
    pub fn new(fs: f32, omega: f32, sigma: f32) -> Self {
        Self::from_dt(omega / fs, 1.0 / (fs * sigma))
    }

    /// Variant taking the two dimensionless ratios directly.
    pub fn from_dt(omega_dt: f32, dt_over_sigma: f32) -> Self {
        Self {
            carrier: SineOsc::new(omega_dt),
            envelope: BellEnv::new(dt_over_sigma),
            steps_done: 0,
        }
    }

    /// Restart both generators; the step counter returns to zero.
    pub fn reset(&mut self) -> &mut Self {
        self.carrier.reset();
        self.envelope.reset();
        self.steps_done = 0;
        self
    }

    #[inline(always)]
    pub fn steps_done(&self) -> u32 {
        self.steps_done
    }

    /// Returns f(t_n) as int8. First call yields t = dt, not t = 0.
    #[inline(always)]
    pub fn update(&mut self) -> i8 {
        let carrier_value = self.carrier.update(); // Q15, |value| <= 32767
        let envelope_value = self.envelope.update(); // Q15, 0..32767
        self.steps_done += 1;
        // Q15 * Q15 -> Q15, then Q15 -> int8 with rounding
        sat8((((carrier_value * envelope_value) >> 15) + 128) >> 8)
    }
}

pub struct ExciterWaveShow {
    steps_per_frame: usize,
    exciter: Exciter,
    wave: Wave8<RESOLUTION>
}

impl ExciterWaveShow {
    pub fn new(
    ) -> ExciterWaveShow {
        let steps_per_frame: usize = 1;
        let lam_led: f32 = 4.5;
        let frequency: f32 = 2.0;
        let omega: f32 = 10.0;
        let sigma: f32 = 2.0;


        let freq_steps = INTERSTEPS as f32 * lam_led * frequency;
        let exciter = Exciter::new(freq_steps, omega, sigma);

        ExciterWaveShow { exciter, wave: Wave8::new(Boundary::Mur), steps_per_frame }
    }
}

impl Show for ExciterWaveShow {
    fn title(&self) -> &'static str {
        "WaveShow"
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {

    }

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {
        iface.led_strip().black();
        let left_edge_next = self.exciter.update();
        self.wave.step(left_edge_next);

        let wave_field = self.wave.field();

        for i in 0..NUM_LED {
            let color = Color {
                r: (wave_field[i*INTERSTEPS] as i16 + 127) as u8,
                g: 0, //(self.color_diff[i].g + 127) as u8,
                b: 0, //(self.color_diff[i].b + 127) as u8,
            };
            iface.led_strip().set_led(i as isize, color);
        }

        iface.write_spi();

        StepResult::Running
    }
}






















// struct WavePropagator {
//     curr: [i8; RESOLUTION],
//     prev: [i8; RESOLUTION],

//     stimulus: Option<i8>
// }

// impl WavePropagator {
//     pub fn new() -> WavePropagator {
//         let curr = core::array::from_fn(|_| 0);
//         let prev = core::array::from_fn(|_| 0);

//         WavePropagator { curr, prev, stimulus: None }
//     }

//     pub fn stimulate(&mut self, signal: i8) {
//         self.stimulus = Some(signal);
//     }

//     pub fn process(&mut self, buffer: &mut [i8; RESOLUTION]) {
//         match self.stimulus {
//             Some(signal) => self.curr[0] = signal,
//             None => {}
//         }
//         self.stimulus = None;
//         for i in 1..(RESOLUTION-1) {
//             let prev = self.prev[i] as i16;
//             let curr = self.curr[i] as i16;
//             let curr_right = self.curr[i+1] as i16;
//             let curr_left = self.curr[i-1] as i16;
//             let next = 2 * curr - prev + (curr_right - 2 * curr + curr_left);

//             buffer[i] = next as i8; //.min(127).max(-127) as i8;
//         }
//         buffer[RESOLUTION-1] = self.curr[RESOLUTION-2];

//         for i in 0..RESOLUTION {
//             self.prev[i] = self.curr[i];
//             self.curr[i] = buffer[i];
//         }
//     }
// }


// pub struct Wave {
//     sine: Sine,
//     envelope: Envelope,
//     hue: f32,

//     position: Position,
//     propagator: WavePropagator,
//     val: i8,
// }


// impl Wave {
//     pub fn new(wave_length: u8, speed: u8, hue: f32) -> Wave {
//         let wave_length6 = (wave_length as u32) << 6;
//         let speed6 = (speed as u32) << 3;

//         let frequency = (speed6 / wave_length6) >> 6;
//         let elastic = (frequency * frequency) as isize;

//         let position = Position::new(0, 0);

//         Wave {
//             sine: Sine::new(64, 1, 64),
//             envelope: Envelope::new(64),
//             hue, position, propagator: WavePropagator::new(), val: 64}
//     }

//     fn process(&mut self, led_diff: &mut [ColorDiff; NUM_LED], buffer: &mut [i8; RESOLUTION]) {
//         for _ in 0..10 {
//             let signal = self.envelope.process(self.sine.process() as i8);
//             self.propagator.stimulate(signal);
//             self.propagator.process(buffer);
//             self.val = 0; //(self.val - 1).max(0);

//         }

//         for i in 0..STRIP_LENGTH {
//             let pos = self.position.led_pos();
//             led_diff[pos+i].r = buffer[i * INTERSTEPS] as isize;
//         }
//     }
// }


// pub struct WaveShow {
//     wave: Wave,
//     color_diff: [ColorDiff; NUM_LED],
//     buffer: [i8; RESOLUTION],
// }

// impl WaveShow {
//     pub fn new() -> WaveShow {
//         let color_diff = core::array::from_fn(|_| NEUTRAL);
//         let buffer = core::array::from_fn(|_| 0);

//         WaveShow {wave: Wave::new(6, 2, 1.0), color_diff, buffer}
//     }
// }

// impl Show for WaveShow {
//     fn title(&self) -> &'static str {
//         "WaveShow"
//     }

//     fn reset(&mut self, _iface: &mut Interface<'_>) {

//     }

//     fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {
//         iface.led_strip().black();
//         self.wave.process(&mut self.color_diff, &mut self.buffer);

//         for i in 0..NUM_LED {
//             let color = Color {
//                 r: (self.color_diff[i].r * 4 + 64) as u8,
//                 g: 0, //(self.color_diff[i].g + 127) as u8,
//                 b: 0, //(self.color_diff[i].b + 127) as u8,
//             };
//             iface.led_strip().set_led(i as isize, color);
//         }

//         iface.write_spi();

//         StepResult::Running
//     }
// }
