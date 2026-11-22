
use crate::math8::{I16_MAX, sat15};

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

    omega_dt: f32,
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
            omega_dt,
        }
    }

    // /// Build from precomputed design constants (float-free at startup).
    // pub const fn from_phase_step(phase_step: i32, initial_cos: i32) -> Self {
    //     Self {
    //         phase_step,
    //         sin_value: 0,
    //         cos_value: initial_cos,
    //         initial_cos,
    //     }
    // }

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

    pub fn frames_per_wave(&self) -> f32 {
        core::f32::consts::PI / (0.5 * self.omega_dt)
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
