use crate::{
    conf::*, interface::{Canvas, Interface}, led::{BLACK, BLUE, Color, GREEN, RED}, math8::{I16_MAX, iscale8, sat8, sat15}, player::{Show, StepResult},
};

use crate::usb_writeln;

const INTERSTEPS: usize = 10;
const RESOLUTION: usize = INTERSTEPS * STRIP_LENGTH + STRIP_LENGTH / 2;


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

    pub fn distance(&self, led: usize) -> usize {
        let alt = (led % STRIP_LENGTH) as i16;
        let strip = (led / STRIP_LENGTH) as i16;

        let p_alt = self.alt as i16;
        let p_strip = self.strip as i16;

        let d_alt2 = ((alt - p_alt) * (alt - p_alt)) as u16;

        let half_circle = (STRIP_NUM / 2) as u16;
        let d_strip = (strip - p_strip).abs() as u16;

        let d_strip2 = if d_strip <= half_circle  {
            (d_strip * d_strip) as u16
        } else {
            (STRIP_NUM as u16 - d_strip) * (STRIP_NUM as u16 - d_strip)
        };

        let d = (d_strip2 + d_alt2).isqrt() as usize;
        d * INTERSTEPS
    }

    #[allow(dead_code)] // might be used in the future
    fn distance_alt(&self, led: usize) -> usize {
        let alt = led % STRIP_LENGTH;
        (self.alt as i32 - alt as i32).abs() as usize * INTERSTEPS
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
/// ```text
/// field_now[j]  <=  field_now[j-1] + field_now[j+1] - field_before[j]
/// ```
/// Two int8 arrays = 2*N bytes of total state. Updated in place, walking
/// backwards so the still-unmodified right neighbour is read.
pub struct Wave8<const N: usize> {
    /// current time level u^n
    field_now: [i8; N],
    /// previous time level u^(n-1), supplies the u_tt term
    field_before: [i8; N],
    boundary: Boundary,
    decay_gain: Option<i32>,
    steps_done: u32,
    source_overlay_past: i8,
}

impl<const N: usize> Wave8<N> {
    pub fn new(boundary: Boundary, decay: Option<f32>) -> Self {

        let decay_gain = decay.map(|d| ((1 << 15) as f32 * d) as i32);

        assert!(N >= 3, "Wave8: N >= 3 required");
        Self {
            field_now: [0; N],
            field_before: [0; N],
            boundary,
            decay_gain,
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
    pub fn field(&self, pos: usize) -> i8 {
        ((self.field_before[pos] as i16 + self.field_now[pos] as i16) >> 1) as i8
    }

    /// Previous time level u^(n-1).
    #[inline(always)]
    pub fn previous_field(&self) -> &[i8; N] {
        &self.field_before
    }

    fn inactive(&self) -> bool {
        self.field_now.iter().all(|&f| f == 0)
    }

    #[inline]
    fn damp_both_levels(&mut self) {
        match self.decay_gain {
            Some(gain) => {
                for value in self.field_now.iter_mut().chain(self.field_before.iter_mut()) {
                    *value = damp_scale(*value, gain);
                }
            }
            None => {}
        }
    }

    #[inline(always)]
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }

    pub fn set_boundary(&mut self, boundary: Boundary) {
        self.boundary = boundary;
    }

    pub fn excite(&mut self, excitement: i8) {
        self.field_now[0] = excitement;

    }

    #[inline]
    pub fn step_overlay(&mut self, excitement: i8) {
        // Save old-level edge values before anything is overwritten:
        // right_edge_before = u[N-1]^n, inner_edge_before = u[N-2]^n

        let plain_field = sat8(self.field_now[0] as i32 - self.source_overlay_past as i32);
        self.field_now[0] = sat8(plain_field as i32 + excitement as i32);

        self.advance();

        self.field_before[0] = plain_field;

        self.field_now[0] = sat8(self.field_now[0] as i32 + excitement as i32);
        self.source_overlay_past = excitement;
    }

    pub fn step(&mut self, excitement: i8) {
        self.field_now[0] = excitement;
        self.advance();
    }

    pub fn advance(&mut self) {
        let right_edge_before = self.field_now[N - 1];
        let inner_right_edge_before = self.field_now[N - 2];

        let left_edge_before = self.field_now[0];

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

        // Right edge: closure per boundary type.
        self.field_before[0] = left_edge_before;
        self.field_before[N - 1] = right_edge_before;
        self.field_now[N - 1] = match self.boundary {
            // r = 0 when sigma = 1: exact, one-sample memory only
            Boundary::Mur => inner_right_edge_before,
            // mirror image of the interior node
            Boundary::Neumann => sat8(2 * inner_right_edge_before as i32 - right_edge_before as i32),
            Boundary::Dirichlet => 0,
        };

        self.steps_done += 1;
        if self.steps_done == 8 {
            self.damp_both_levels();
            self.steps_done = 0;
        }
    }
}

#[inline(always)]
fn damp_scale(value: i8, decay_gain: i32) -> i8 {
    let magnitude = value.unsigned_abs() as i32;
    if magnitude == 0 {
        return 0; // no sign to preserve, nothing to do
    }
    let scaled = (magnitude * decay_gain + 16384) >> 15; // round to nearest
    let decayed = if scaled < magnitude { scaled } else { magnitude - 1 };
    if value < 0 { -(decayed as i8) } else { decayed as i8 }
}


// ===========================================================================
// 3. SineOsc -- sin(omega*t) without sin
// ===========================================================================

/// Quadrature oscillator ("magic circle").
///
/// ```text
/// sin_new = sin_value + phase_step*cos_value
/// cos_new = cos_value - phase_step*sin_new
/// ```
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
// 4. BellEnv -- Gaussian envelope, no exp(), no f32 in the hot path
// ===========================================================================

/// Octave span between full scale and the floor: 32768 * 2^-10 == 32.
const BELL_OCTAVES: i64 = 10;

/// Sample value at both feet of the symmetric bell, in Q15.
const BELL_ENDPOINT_Q15: i32 = 32;

/// Legacy mode terminates when the envelope drops below 2^-10 of full scale.
const ENVELOPE_LOG2_FLOOR: i32 = 10;

const INV_LN2: f32 = 1.442_695_040_888_963_4;
const MAX_FRAC_BITS: u32 = 27;      // 10 << 27 still fits in i32
const MIN_FRAC_BITS: u32 = 12;
const MAX_SLOPE_Q: i64 = 1 << 30;   // headroom for slope + curvature

/// Upper bound on the symmetric parameter: beyond this the curvature's
/// quantisation shifts the half-height point by more than ~5 %.
pub const MAX_FRAMES_TO_PEAK: u32 = 3663;

pub struct BellEnv {
    /// log2(envelope) in Q(frac_bits); <= 0 always
    log2_envelope: i32,
    /// First difference of log2_envelope
    log2_slope: i32,
    /// Second difference, constant and negative
    log2_curvature: i32,
    /// Lower bound for log2_envelope, = -(BELL_OCTAVES << frac_bits)
    log2_floor: i32,
    /// Fractional bits of log2_envelope
    frac_bits: u32,
    /// log2_envelope at frame 0: 0 (peak first) or log2_floor (sym: at the foot)
    initial_log2: i32,
    /// log2_slope at frame 0, so reset() is exact
    initial_slope: i32,
    /// 0 = peak-first legacy mode; otherwise frames from frame 0 to the peak
    peak_frame: u32,
    /// frame whose sample the next update() returns (symmetric mode only)
    frame: u32,
    /// True once the envelope has ended
    finished: bool,
}

impl BellEnv {
    // -----------------------------------------------------------------------
    // Legacy constructor: peak first, monotone decay. Unchanged numerics.
    // -----------------------------------------------------------------------
    /// `dt_over_sigma` = dt / sigma. Starts at the peak (frame 1 == dt) and
    /// decays until the envelope falls below 2^-10 of full scale.
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
            initial_log2: 0,
            initial_slope,
            peak_frame: 0,
            frame: 0,
            finished: false,
        }
    }

    // -----------------------------------------------------------------------
    // Symmetric constructor: 32 -> 32767 -> 32, one parameter.
    // -----------------------------------------------------------------------
    /// Symmetric bell. `frames_to_peak` = L gives:
    /// frame 0 = 32, frame L = 32767, frame 2L = 32, then 0.
    /// That is 2L + 1 non-zero samples. sigma is implied: L = 2.628*sigma/dt.
    pub fn symmetric(frames_to_peak: u32) -> Self {
        assert!(frames_to_peak >= 2, "BellEnv: frames_to_peak must be >= 2");
        assert!(
            frames_to_peak <= MAX_FRAMES_TO_PEAK,
            "BellEnv: frames_to_peak must be <= 3663"
        );

        let frames = frames_to_peak as i64;
        let frames_sq = frames * frames;

        // Largest Q-format whose slope and curvature still fit in i32.
        let mut frac_bits = MAX_FRAC_BITS;
        let curvature_q: i32 = loop {
            // rounded: log2 g(k) = -C*(k-L)^2 with C = 10/L^2
            let c = ((BELL_OCTAVES << frac_bits) + frames_sq / 2) / frames_sq;
            if c >= 1 && c * (2 * frames + 1) <= MAX_SLOPE_Q {
                break c as i32;
            }
            assert!(frac_bits > MIN_FRAC_BITS, "BellEnv: frames_to_peak too small");
            frac_bits -= 1;
        };

        let floor = -((BELL_OCTAVES << frac_bits) as i32);
        let initial_slope = (curvature_q as i64 * (2 * frames - 1)) as i32;

        Self {
            log2_envelope: floor,
            log2_slope: initial_slope,
            log2_curvature: -2 * curvature_q,
            log2_floor: floor,
            frac_bits,
            initial_log2: floor,
            initial_slope,
            peak_frame: frames_to_peak,
            frame: 0,
            finished: false,
        }
    }

    /// Restart at frame 0: at the peak (legacy) or at the left foot (symmetric).
    pub fn reset(&mut self) -> &mut Self {
        self.log2_envelope = self.initial_log2;
        self.log2_slope = self.initial_slope;
        self.frame = 0;
        self.finished = false;
        self
    }

    /// True once the envelope has ended; update() returns 0 from then on.
    #[inline(always)]
    pub fn finished(&self) -> bool {
        self.finished
    }

    pub fn is_climbing(&self) -> bool {
        self.frame < self.peak_frame
    }

    pub fn frames_done(&self) -> u32 {
        self.frame
    }

    /// Number of non-zero samples, or None in legacy mode (depends on the
    /// floor crossing, which is not known in advance).
    #[inline]
    pub fn sample_count(&self) -> Option<u32> {
        if self.peak_frame == 0 {
            None
        } else {
            Some(2 * self.peak_frame + 1)
        }
    }

    /// Envelope at the current frame, then advance by one frame.
    /// First call yields t = dt (legacy) or frame 0 = 32 (symmetric).
    #[inline]
    pub fn update(&mut self) -> i32 {
        if self.finished {
            return 0;
        }
        if self.peak_frame == 0 {
            self.update_peak_first()
        } else {
            self.update_symmetric()
        }
    }

    #[inline]
    fn update_peak_first(&mut self) -> i32 {
        self.log2_envelope += self.log2_slope;
        self.log2_slope += self.log2_curvature;
        if self.log2_envelope <= self.log2_floor {
            // Freeze here: without this the parabola climbs again and emits a
            // second (mathematically valid, visually wrong) pulse near 2*t0.
            self.log2_envelope = self.log2_floor;
            self.finished = true;
            return 0;
        }
        self.exp2_at(self.log2_envelope)
    }

    #[inline]
    fn update_symmetric(&mut self) -> i32 {
        if self.frame == 2 * self.peak_frame {
            // Right foot: pinned, so the bell ends on exactly the value it
            // started with regardless of the curvature rounding.
            self.finished = true;
            return BELL_ENDPOINT_Q15;
        }

        // Clamp instead of returning 0: a curvature rounded the wrong way
        // would otherwise snap to silence a few frames before the foot.
        let level = if self.log2_envelope > self.log2_floor {
            self.log2_envelope
        } else {
            self.log2_floor
        };
        let value = self.exp2_at(level);

        self.frame += 1;
        if self.frame == self.peak_frame {
            // Snap to the apex: the curvature rounding error is dropped here,
            // so both arms are mirror images by construction.
            self.log2_envelope = 0;
            self.log2_slope = self.log2_curvature / 2; // = -C
        } else {
            self.log2_envelope += self.log2_slope;
            self.log2_slope += self.log2_curvature;
        }
        value
    }

    /// 2^(log2_value / 2^frac_bits) in Q15, log2_value <= 0.
    #[inline(always)]
    fn exp2_at(&self, log2_value: i32) -> i32 {
        // Arithmetic shift = floor, so the fraction below is always in [0,1)
        let exponent_int = log2_value >> self.frac_bits;
        if exponent_int >= 0 {
            return I16_MAX; // exponent >= 0 means envelope >= 1: saturate
        }
        if exponent_int < -15 {
            return 0;
        }
        // Fractional part in Q10, range [0, 1024)
        let fraction_q10 =
            (log2_value - (exponent_int << self.frac_bits)) >> (self.frac_bits - 10);

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
struct Exciter {
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
    fn new(fs: f32, omega: f32, sigma: f32) -> Self {
        Self::from_dt(omega / fs, 1.0 / (fs * sigma))
    }

    /// Variant taking the two dimensionless ratios directly.
    fn from_dt(omega_dt: f32, dt_over_sigma: f32) -> Self {
        Self {
            carrier: SineOsc::new(omega_dt),
            envelope: BellEnv::new(dt_over_sigma),
            steps_done: 0,
        }
    }

    fn reconfigure(&mut self, fs: f32, omega: f32, sigma: f32) -> &mut Self {
        let omega_dt = omega / fs;
        let dt_over_sigma = 1.0 / (fs * sigma);
        self.carrier = SineOsc::new(omega_dt);
        self.envelope = BellEnv::new(dt_over_sigma);
        self.steps_done = 0;
        self
    }

    /// Restart both generators; the step counter returns to zero.
    fn reset(&mut self) -> &mut Self {
        self.carrier.reset();
        self.envelope.reset();
        self.steps_done = 0;
        self
    }

    /// Returns f(t_n) as int8. First call yields t = dt, not t = 0.
    #[inline(always)]
    fn update(&mut self) -> i8 {
        let carrier_value = self.carrier.update(); // Q15, |value| <= 32767
        let envelope_value = self.envelope.update(); // Q15, 0..32767
        self.steps_done += 1;
        // Q15 * Q15 -> Q15, then Q15 -> int8 with rounding
        sat8((((carrier_value * envelope_value) >> 15) + 128) >> 8)
    }

    fn finished(&self) -> bool {
        self.envelope.finished()
    }
}

struct ExciterWave {
    steps_per_frame: usize,
    exciter: Exciter,
    exciter_gap: usize,
    steps_done: usize,
    wave: Wave8<RESOLUTION>,

    position: Position,

    color: Color,
}

impl ExciterWave {
    fn new(position: Position, color: Color, wave_length: f32, decay: Option<f32>, exciter_gap: usize, boundary: Boundary) -> ExciterWave {
        let steps_per_frame: usize = 5;
        let lam_led: f32 = wave_length;
        let frequency: f32 = 2.0 as f32;
        let omega: f32 = 10.0;
        let sigma: f32 = 2.0;

        let freq_steps = INTERSTEPS as f32 * lam_led * frequency;
        let exciter = Exciter::new(freq_steps, omega, sigma);

        ExciterWave {
            position,
            color,
            exciter,
            exciter_gap,
            steps_done: 0,
            wave: Wave8::new(boundary, decay),
            steps_per_frame
        }
    }

    fn new_uninitialized() -> Self {
        let decay = 0.99;
        let frequency: f32 = 2.0 as f32;
        let omega: f32 = 10.0;
        let sigma: f32 = 2.0;
        let freq_steps = INTERSTEPS as f32 * 10.0 * frequency;

        Self {
            position: Position { strip: 0, alt: 0 },
            color: BLACK,
            exciter: Exciter::new(freq_steps, omega, sigma),
            exciter_gap: 0,
            steps_done: 0,
            wave: Wave8::new(Boundary::Dirichlet, Some(decay)),
            steps_per_frame: 5,
        }
    }

    fn initialize(&mut self, position: Position, color: Color, wave_length: f32) -> &mut Self {
        self.exciter_gap = 120;
        self.steps_done = 0;

        self.position = position;
        self.color = color;

        let lam_led: f32 = wave_length;
        let frequency: f32 = 2.0 as f32;
        let omega: f32 = 10.0;
        let sigma: f32 = 2.0;

        let freq_steps = INTERSTEPS as f32 * lam_led * frequency;
        self.exciter.reconfigure(freq_steps, omega, sigma);
        self
    }

    fn reset(&mut self) {
        self.exciter.reset();
        self.steps_done = 0;
    }

    fn finished(&self) -> bool {
        self.exciter_gap != 0 && self.exciter_gap <= self.steps_done
    }

    fn inactive(&self) -> bool {
        self.wave.inactive()
    }

    fn process(&mut self, canvas: &mut Canvas) {
        if self.color == BLACK {
            return;
        }
        for _ in 0..self.steps_per_frame {

            let excitement = self.exciter.update();
            if !self.exciter.finished() {
                self.wave.excite(excitement);
            } else {
                self.steps_done += 1;
            }
            self.wave.advance();
        }

        for i in 0..NUM_LED {
            let dist = self.position.distance(i);
            let field = self.wave.field(dist);
            canvas.paint(i, self.color, field);
        }
    }
}

pub struct WaveShow<const NUM_WAVES: usize> {
    exciter_waves: [ExciterWave; NUM_WAVES],
}

impl<const NUM_WAVES: usize> RandomWaveShow<NUM_WAVES> {

    pub fn new() -> Self {
        let exciter_waves = core::array::from_fn(|_| ExciterWave::new_uninitialized());
        Self { hue: 0.0, exciter_waves }
    }
}

const WAVE_PROB: f32 = 9e-3;


pub struct RandomWaveShow<const NUM_WAVES: usize> {
    hue: f32,
    exciter_waves: [ExciterWave; NUM_WAVES],
}

impl<const NUM_WAVES: usize> Show for RandomWaveShow<NUM_WAVES> {
    fn title(&self) -> &'static str {
        "RandomWaveShow"
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {
    }

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {

        for exciter_wave in self.exciter_waves.iter_mut() {
            if exciter_wave.inactive() && iface.random().value() < WAVE_PROB {
                let color = Color::from_hsv(self.hue, 1.0, 1.0);
                self.hue += 0.2;
                let position = Position {
                    strip: iface.random().value32(STRIP_NUM as u32) as usize,
                    alt: iface.random().value32(STRIP_LENGTH as u32) as usize
                };
                let wave_length = iface.random().value() * 30.0 + 2.0;
                exciter_wave.initialize(position, color, wave_length);
            }
            exciter_wave.process(&mut iface.canvas());
        }

        iface.write_out_canvas();

        StepResult::Running
    }
}

pub struct RaisingWaveShow<const NUM_WAVES: usize> {
    exciter_waves: [ExciterWave; NUM_WAVES],
}

impl<const NUM_WAVES: usize> RaisingWaveShow<NUM_WAVES> {

    pub fn new() -> Self {
        let exciter_waves = core::array::from_fn(|_| ExciterWave::new_uninitialized());
        Self { exciter_waves }
    }

}

impl<const NUM_WAVES: usize> Show for RaisingWaveShow<NUM_WAVES> {
    fn title(&self) -> &'static str {
        "RaisingWaveShow"
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {
    }

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {

        let all_inactive = self.exciter_waves.iter().all(|e| e.inactive());
        if all_inactive && iface.random().value() < WAVE_PROB {
            let position_0 = Position { strip: 0, alt: 0 };
            let position_1 = Position { strip: 7, alt: 0 };
            let position_2 = Position { strip: 23, alt: 0 };

            let hue_0 = iface.random().value();
            let hue_1 = (hue_0 + 0.1) % 1.0;
            let hue_2 = (hue_0 - 0.1) % 1.0;
            let color_0 = Color::from_hsv(hue_0, 1.0, 1.0);
            let color_1 = Color::from_hsv(hue_1, 1.0, 1.0);
            let color_2 = Color::from_hsv(hue_2, 1.0, 1.0);
            let wave_length_0 = iface.random().value() * 30.0 + 2.0;
            let wave_length_1 = iface.random().value() * 30.0 + 2.0;
            let wave_length_2 = iface.random().value() * 30.0 + 2.0;
            self.exciter_waves[0].initialize(position_0, color_0, wave_length_0);
            self.exciter_waves[1].initialize(position_1, color_1, wave_length_1);
            self.exciter_waves[2].initialize(position_2, color_2, wave_length_2);
        }

        for exciter_wave in self.exciter_waves.iter_mut() {
            exciter_wave.process(iface.canvas());
        }

        iface.write_out_canvas();

        StepResult::Running
    }
}

impl<const NUM_WAVES: usize> WaveShow<NUM_WAVES> {
    fn new(exciter_waves: [ExciterWave; NUM_WAVES]) -> Self {
        Self { exciter_waves }
    }
}

pub fn default_waveshow() -> WaveShow<3> {
    let decay = None;
    let exciter_gap = 1;
    let boundary = Boundary::Mur;

    let exciter_waves = [
        // ExciterWave::new(Position {strip: 0, alt: 10}, RED, 30.5, decay),
        // ExciterWave::new(Position {strip: 15, alt: 50}, GREEN, 40.5, decay),
        // ExciterWave::new(Position {strip: 20, alt: 30}, BLUE, 25.5, decay),
        ExciterWave::new(Position {strip: 0, alt: 20}, RED, 7.5, decay, exciter_gap, boundary),
        ExciterWave::new(Position {strip: 15, alt: 50}, GREEN, 12.5, decay, exciter_gap, boundary),
        ExciterWave::new(Position {strip: 20, alt: 10}, BLUE, 25.5, decay, exciter_gap, boundary),
    ];

    WaveShow::<3>::new(exciter_waves)
}

pub fn constant_waveshow() -> WaveShow<3> {
    let decay = Some(0.99999);
    let exciter_gap = 120;
    let boundary = Boundary::Dirichlet;

    let exciter_waves = [
        // ExciterWave::new(Position {strip: 0, alt: 10}, RED, 30.5, decay),
        // ExciterWave::new(Position {strip: 15, alt: 50}, GREEN, 40.5, decay),
        // ExciterWave::new(Position {strip: 20, alt: 30}, BLUE, 25.5, decay),
        ExciterWave::new(Position {strip: 0, alt: 20}, RED, 7.5, decay, exciter_gap, boundary),
        ExciterWave::new(Position {strip: 15, alt: 50}, GREEN, 12.5, decay, exciter_gap, boundary),
        ExciterWave::new(Position {strip: 20, alt: 10}, BLUE, 25.5, decay, exciter_gap, boundary),
    ];

    WaveShow::<3>::new(exciter_waves)
}


impl<const NUM_WAVES: usize> Show for WaveShow<NUM_WAVES> {
    fn title(&self) -> &'static str {
        "WaveShow"
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {

    }

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {

        for exciter_wave in self.exciter_waves.iter_mut() {
            exciter_wave.process(iface.canvas());
            if exciter_wave.finished() {
                exciter_wave.reset();
            }
        }

        iface.write_out_canvas();

        StepResult::Running
    }
}


struct StaticWave {
    wave: Wave8<RESOLUTION>,
    position: Position,
    color: Color,

    carrier: SineOsc,
    envelope: BellEnv,
    active: bool,
}

impl StaticWave {
    fn new(position: Position, color: Color, omega_dt: f32, dt_over_sigma: f32) -> Self {
        Self {
            wave: Wave8::new(Boundary::Mur, None),
            position,
            color,
            carrier: SineOsc::new(omega_dt),
            envelope: BellEnv::new(dt_over_sigma),
            active: true,
        }
    }

    fn new_uninitialized() -> Self {
        let frequency: f32 = 2.0 as f32;
        let omega: f32 = 10.0;
        let sigma: f32 = 2.0;
        let freq_steps = INTERSTEPS as f32 * 10.0 * frequency;

        let omega_dt = omega / freq_steps;
        let dt_over_sigma = 1.0 / (freq_steps * sigma);

        Self {
            position: Position { strip: 0, alt: 0 },
            color: BLACK,
            carrier: SineOsc::new(omega_dt),
            envelope: BellEnv::new(dt_over_sigma),
            wave: Wave8::new(Boundary::Mur, None),
            active: false,
        }
    }

    fn initialize(&mut self, position: Position, color: Color, wave_length: f32, bell_frames: u32) -> &mut Self {
        self.position = position;
        self.color = color;

        self.envelope = BellEnv::symmetric(bell_frames);

        let omega: f32 = 10.0;
        let frequency: f32 = 2.0 as f32;
        let freq_steps = INTERSTEPS as f32 * wave_length * frequency;
        let omega_dt = omega / freq_steps;

        self.carrier = SineOsc::new(omega_dt);

        for _ in 0..RESOLUTION {
            let signal = self.carrier.update();
            let excitement = sat8(signal >> 8) as i8;
            self.wave.step(excitement);

        }

        self.active = true;

        self
    }

    fn process(&mut self, canvas: &mut Canvas) {
        if self.color == BLACK {
            return;
        }

        let bell_value = sat8(self.envelope.update() >> 9);

        for i in 0..NUM_LED {
            let dist = self.position.distance(i);
            let field = self.wave.field(dist);
            let value = iscale8(field, bell_value);
            canvas.paint(i, self.color, value);
        }

        self.active = self.envelope.frame < 2 * CLIMB_FRAMES - 1;
    }

    fn is_finished(&self) -> bool {
        !self.active
    }
}


pub struct StaticWaveShow<const NUM_WAVES: usize> {
    static_waves: [StaticWave; NUM_WAVES],
    hue: f32,
}

impl<const NUM_WAVES: usize> StaticWaveShow<NUM_WAVES> {
    pub fn new() -> Self {
        let static_waves = core::array::from_fn(|_| StaticWave::new_uninitialized());
        Self { static_waves, hue: 0.4 }
    }
}

const CLIMB_FRAMES: u32 = 150;

impl<const NUM_WAVES: usize> Show for StaticWaveShow<NUM_WAVES> {
    fn title(&self) -> &'static str {
        "StaticWaveShow"
    }

    fn reset(&mut self, _iface: &mut Interface<'_>) {}

    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult {

        let all_finished = self.static_waves.iter().all(|sw| sw.is_finished());

        for i in 0..NUM_WAVES {
            let prev = (i + NUM_WAVES - 1) % NUM_WAVES;
            let previous_climbing_frames = self.static_waves[prev].envelope.frames_done();
            let new_needed = all_finished || previous_climbing_frames > 2 * CLIMB_FRAMES / NUM_WAVES as u32;
            let static_wave = &mut self.static_waves[i];
            if static_wave.is_finished() && new_needed {
                relaunch_wave(iface, static_wave, self.hue);
                self.hue = (self.hue + 0.3) % 1.0;
                static_wave.process(iface.canvas());
                break;
            }
        }
        for static_wave in self.static_waves.iter_mut() {
            if !static_wave.is_finished() {
                static_wave.process(iface.canvas());
            }
        }

        iface.write_out_canvas();

        StepResult::Running
    }
}

fn relaunch_wave(iface: &mut Interface<'_>, static_wave: &mut StaticWave, hue: f32) {
    let position = Position {
        strip: iface.random().value32(STRIP_NUM as u32) as usize,
        alt: iface.random().value32(STRIP_LENGTH as u32) as usize
    };

    let wave_length = iface.random().value() * 20.0 + 8.0;
    let bell_frames = CLIMB_FRAMES;
    static_wave.initialize(position, Color::from_hsv(hue, 1.0, 1.0), wave_length, bell_frames);
}
