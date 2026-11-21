use crate::{
    conf::*,
    interface::{Canvas, Interface},
    led::{BLACK, BLUE, Color, GREEN, RED},
    math8::{iscale8, sat8},
    player::{Show, StepResult},
    signals::{SineOsc, BellEnv},
};

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

        self.active = self.envelope.frames_done() < 2 * CLIMB_FRAMES - 1;
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
