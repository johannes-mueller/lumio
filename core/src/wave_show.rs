use crate::{
    conf::*, interface::Interface,
    led::Color,
    player::{Show, StepResult},
};

const INTERSTEPS: usize = 10;
const RESOLUTION: usize = INTERSTEPS * 2 * STRIP_LENGTH;

pub fn iscale8(a: i8, b: i8) -> i8 {
    ((a as i16 * b as i16) >> 7) as i8
}

pub struct ColorDiff {
    r: i16,
    g: i16,
    b: i16,
}

pub struct Position {
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

const I8_MIN: i32 = i8::MIN as i32;
const I8_MAX: i32 = i8::MAX as i32;
const I16_MIN: i32 = i16::MIN as i32;
const I16_MAX: i32 = i16::MAX as i32;


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
    decay: i32,
    steps_done: u32,
    source_overlay_past: i8,
    excite_position: usize,
}

impl<const N: usize> Wave8<N> {
    pub fn new(boundary: Boundary, excite_position: usize) -> Self {
        assert!(N >= 3, "Wave8: N >= 3 required");
        Self {
            field_now: [0; N],
            field_before: [0; N],
            boundary,
            decay: 256,
            excite_position,
            source_overlay_past: 0,
            steps_done: 0,
        }
    }

    /// Reset to the resting state; the source history is not restored.
    pub fn reset(&mut self) -> &mut Self {
        self.field_now = [0; N];
        self.field_before = [0; N];
        self.steps_done = 0;
        self.source_overlay_past = 0;
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

    pub fn excite(&mut self, excitement: i8) {
        self.field_now[self.excite_position] = excitement;
    }

    #[inline]
    pub fn step_overlay(&mut self, excitement: i8) {
        // Save old-level edge values before anything is overwritten:
        // right_edge_before = u[N-1]^n, inner_edge_before = u[N-2]^n

        let node = self.excite_position;

        let plain_field = sat8(self.field_now[node] as i32 - self.source_overlay_past as i32);
        self.field_now[node] = sat8(plain_field as i32 + excitement as i32);

        self.advance();

        self.field_before[node] = plain_field;

        self.field_now[node] = sat8(self.field_now[node] as i32 + excitement as i32);
        self.source_overlay_past = excitement;
    }

    pub fn step(&mut self, excitement: i8) {
        self.field_now[self.excite_position] = excitement;
        self.advance();
    }

    pub fn advance(&mut self) {
        let right_edge_before = self.field_now[N - 1];
        let inner_right_edge_before = self.field_now[N - 2];

        let left_edge_before = self.field_now[0];
        let inner_left_edge_before = self.field_now[1];

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

            let new_value_decay = (new_value_raw * self.decay) >> 8;
            //let new_value_decay = new_value_raw;
            self.field_now[j] = sat8(new_value_raw);

            right_neighbour = center_before; // for the next (leftward) j
            j -= 1;
        }

        // Left edge: Dirichlet condition, i.e. the source.

        //self.field_now[self.excite_position] =  excitement;
        // Right edge: closure per boundary type.
        self.field_before[0] = left_edge_before;
        self.field_before[N - 1] = right_edge_before;
        self.field_now[0] = match self.boundary {
            // r = 0 when sigma = 1: exact, one-sample memory only
            Boundary::Mur => inner_left_edge_before,
            // mirror image of the interior node
            Boundary::Neumann => sat8(2 * inner_left_edge_before as i32 - left_edge_before as i32),
            Boundary::Dirichlet => 0,
        };
        self.field_now[N - 1] = match self.boundary {
            // r = 0 when sigma = 1: exact, one-sample memory only
            Boundary::Mur => inner_right_edge_before,
            // mirror image of the interior node
            Boundary::Neumann => sat8(2 * inner_right_edge_before as i32 - right_edge_before as i32),
            Boundary::Dirichlet => 0,
        };

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

        let initial_cos = (I16_MAX as f32 * cos_half_angle + 0.5) as i32;
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

    pub fn finished(&self) -> bool {
        self.finished
    }

    /// 2^(log2_envelope / 2^frac_bits) in Q15, log2_envelope <= 0.
    #[inline(always)]
    fn exp2(&self) -> i32 {
        // Arithmetic shift = floor, so the fraction below is always in [0,1)
        let exponent_int = self.log2_envelope >> self.frac_bits;
        if exponent_int >= 0 {
            return I16_MAX; // exponent >= 0 means envelope >= 1: saturate
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

    pub fn finished(&self) -> bool {
        self.envelope.finished()
    }
}

pub struct ExciterWave {
    steps_per_frame: usize,
    exciter: Exciter,
    wave: Wave8<RESOLUTION>,

    position: Position,

    color: Color,
}

impl ExciterWave {
    pub fn new(position: Position, hue: f32, wave_length: f32) -> ExciterWave {
        let steps_per_frame: usize = 5;
        let lam_led: f32 = wave_length;
        let frequency: f32 = 2.0 as f32;
        let omega: f32 = 10.0;
        let sigma: f32 = 2.0;

        let freq_steps = INTERSTEPS as f32 * lam_led * frequency;
        let exciter = Exciter::new(freq_steps, omega, sigma);

        let alt = 0;//position.alt;

        ExciterWave {
            position,
            color: Color::from_hsv(hue, 1.0, 1.0),
            exciter,
            wave: Wave8::new(Boundary::Dirichlet, alt*INTERSTEPS),
            steps_per_frame
        }
    }

    fn distance_S(&self, led: usize) -> usize {
        let alt = led % STRIP_LENGTH;
        (alt as i32 - self.position.alt as i32).abs() as usize * INTERSTEPS
    }

    fn distance(&self, led: usize) -> usize {
        let alt = (led % STRIP_LENGTH) as i16;
        let strip = (led / STRIP_LENGTH) as i16;

        let p_alt = self.position.alt as i16;
        let p_strip = self.position.strip as i16;

        let d_alt2 = ((alt - p_alt) * (alt - p_alt)) as u16;

        let half_circle = (STRIP_NUM / 2) as u16;
        let d_strip = (strip - p_strip).abs() as u16;

        let d_strip2 = if d_strip <= half_circle  {
            (d_strip * d_strip) as u16
        } else {
            ((STRIP_NUM as u16 - d_strip) * (STRIP_NUM as u16 - d_strip))
        };

        let d = (d_strip2 + d_alt2).isqrt() as usize;
        d * INTERSTEPS
    }

    pub fn process(&mut self, canvas: &mut [ColorDiff; NUM_LED]) {
        for _ in 0..self.steps_per_frame {
            let excitement = self.exciter.update();
            self.wave.step(excitement);

        }

        let wave_field = self.wave.field();

        for i in 0..NUM_LED {
            let dist = self.distance(i);
            let field = wave_field[dist];

            canvas[i].r = canvas[i].r + iscale8((self.color.r >> 1) as i8, field) as i16;
            canvas[i].g = canvas[i].g + iscale8((self.color.g >> 1) as i8, field) as i16;
            canvas[i].b = canvas[i].b + iscale8((self.color.b >> 1) as i8, field) as i16;
        }
    }
}

pub struct WaveShow {
    exciter_waves: [ExciterWave; 3],
    color_diff: [ColorDiff; NUM_LED],
}

impl WaveShow {
    pub fn new() -> Self {
        let color_diff = core::array::from_fn(|_| ColorDiff {r: 0, g: 0, b: 0});

        let exciter_waves = [
            ExciterWave::new(Position {strip: 0, alt: 20}, 0.0, 7.5),
            ExciterWave::new(Position {strip: 15, alt: 50}, 0.33, 12.5),
            ExciterWave::new(Position {strip: 20, alt: 10}, 0.67, 25.5),
        ];

        Self {
            exciter_waves,
            color_diff
        }
    }
}

impl Show for WaveShow {
    fn title(&self) -> &'static str {
        "WaveShow"
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {

    }

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {

        for i in 0..NUM_LED {
            self.color_diff[i] = NEUTRAL;
        }
        for i in 0..3 {
            self.exciter_waves[i].process(&mut self.color_diff);
        }

        iface.led_strip().black();

        for i in 0..NUM_LED {
            let red = sat8(self.color_diff[i].r as i32);
            let green = sat8(self.color_diff[i].g as i32);
            let blue = sat8(self.color_diff[i].b as i32);
            let red_intensity = iscale8(red, red) as u8;
            let green_intensity = iscale8(green, green) as u8;
            let blue_intensity = iscale8(blue, blue) as u8;
            let color = Color {
                r: red_intensity,
                g: green_intensity,
                b: blue_intensity,
            };
            iface.led_strip().set_led(i as isize, color);
        }

        iface.write_spi();

        StepResult::Running
    }
}
