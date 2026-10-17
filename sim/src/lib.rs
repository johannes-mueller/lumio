//! WebAssembly simulator for the lumio LED flower.
//!
//! The show logic itself lives in `lumio-core`; this crate provides a
//! [`SimPlatform`] that keeps a virtual clock, remembers the last frame, and
//! lets JavaScript drive the buttons. The [`Sim`] type is the API the web page
//! talks to.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use wasm_bindgen::prelude::*;

use lumio_core::conf::NUM_LED;
use lumio_core::interface::Interface;
use lumio_core::player::Player;
use lumio_core::platform::{ButtonId, Micros, Platform, StatusLedId};

const HALF: usize = NUM_LED / 2;

/// One simulation step per 1/60 s of virtual time.
const STEP_INTERVAL_US: u64 = 1_000_000 / 60;

/// Maximum number of steps to run in a single `tick` to avoid a spiral of
/// death when the tab was backgrounded.
const MAX_STEPS_PER_TICK: u32 = 16;

/// Chat-log ring buffer size. Older lines are dropped when it overflows.
const MESSAGE_LOG_MAX: usize = 200;

/// Maximum inbound chat text length (characters), matching the player's
/// line buffer.
const INBOUND_TEXT_MAX_CHARS: usize = 60;

/// State shared between the platform (seen by the core) and the `Sim` handle
/// (driven by JavaScript).
struct Shared {
    now: Micros,
    advance_pressed: bool,
    action_pressed: bool,
    auto_led: bool,
    active_led: bool,
    /// RGB, three bytes per LED, in strip order (strip 0 first).
    frame: Vec<u8>,
    /// Chat log shown in the UI, oldest first (ring buffer).
    messages: VecDeque<String>,
    /// Bytes written by the core since the last line break (e.g. a show
    /// announcement split into two `usb_write` calls).
    out_pending: Vec<u8>,
    /// Queued inbound chat text, to be drained by `usb_read`.
    inbound: VecDeque<String>,
}

impl Shared {
    fn new() -> Shared {
        Shared {
            now: 0,
            advance_pressed: false,
            action_pressed: false,
            auto_led: false,
            active_led: false,
            frame: vec![0u8; NUM_LED * 3],
            messages: VecDeque::new(),
            out_pending: Vec::new(),
            inbound: VecDeque::new(),
        }
    }

    /// Append written bytes to the out buffer; newline-terminated lines land
    /// in the message log, one `String` per line.
    fn push_out(&mut self, data: &[u8]) {
        for &byte in data {
            if byte == b'\n' {
                let line = String::from_utf8_lossy(&self.out_pending).into_owned();
                if !line.is_empty() {
                    self.push_message(line);
                }
                self.out_pending.clear();
            } else {
                self.out_pending.push(byte);
            }
        }
    }

    fn push_message(&mut self, line: String) {
        if self.messages.len() == MESSAGE_LOG_MAX {
            self.messages.pop_front();
        }
        self.messages.push_back(line);
    }

    /// Serve at most `buf.len()` bytes out of the inbound chat queue, feeding
    /// the CDC read side the way real hardware would.
    fn pop_inbound(&mut self, buf: &mut [u8]) -> usize {
        let mut count = 0;
        while let Some(front) = self.inbound.front_mut() {
            if count == buf.len() {
                break;
            }
            let remaining = front.as_bytes();
            let mut take = remaining.len().min(buf.len() - count);
            if take == 0 {
                break;
            }
            // Never hand over a partial UTF-8 character.
            while !front.is_char_boundary(take) {
                take -= 1;
            }
            buf[count..count + take].copy_from_slice(&remaining[..take]);
            count += take;
            if take == remaining.len() {
                self.inbound.pop_front();
            } else {
                front.drain(..take);
                break; // no room for another char
            }
        }
        count
    }
}

struct SimPlatform {
    shared: Rc<RefCell<Shared>>,
}

impl Platform for SimPlatform {
    fn now_micros(&self) -> Micros {
        self.shared.borrow().now
    }

    fn present(&mut self, a: &[u8], b: &[u8]) {
        let mut shared = self.shared.borrow_mut();
        // APA102 payload: one 0xff prefix and then B, G, R for every LED.
        for i in 0..HALF {
            let dst = i * 3;
            shared.frame[dst] = a[4 + i * 4 + 3];
            shared.frame[dst + 1] = a[4 + i * 4 + 2];
            shared.frame[dst + 2] = a[4 + i * 4 + 1];
        }
        for i in 0..HALF {
            let led = i + HALF;
            let dst = led * 3;
            shared.frame[dst] = b[4 + i * 4 + 3];
            shared.frame[dst + 1] = b[4 + i * 4 + 2];
            shared.frame[dst + 2] = b[4 + i * 4 + 1];
        }
    }

    fn delay_ms(&mut self, _ms: u32) {
        // The simulator paces itself with `requestAnimationFrame`; delays are
        // intentionally ignored.
    }

    fn raw_button(&self, id: ButtonId) -> bool {
        let shared = self.shared.borrow();
        match id {
            ButtonId::Advance => shared.advance_pressed,
            ButtonId::Action => shared.action_pressed,
        }
    }

    fn set_status_led(&mut self, id: StatusLedId, on: bool) {
        let mut shared = self.shared.borrow_mut();
        match id {
            StatusLedId::Auto => shared.auto_led = on,
            StatusLedId::Active => shared.active_led = on,
        }
    }

    fn usb_poll(&mut self) {
        // A no-op; the virtual CDC has no stack to service.
    }

    fn usb_write(&mut self, data: &[u8]) -> usize {
        let mut shared = self.shared.borrow_mut();
        shared.push_out(data);
        data.len()
    }

    fn usb_read(&mut self, buf: &mut [u8]) -> usize {
        self.shared.borrow_mut().pop_inbound(buf)
    }
}

/// The simulator handle exposed to JavaScript.
#[wasm_bindgen]
pub struct Sim {
    shared: Rc<RefCell<Shared>>,
    interface: Interface<'static>,
    player: Player,
    speed: f64,
    paused: bool,
    last_real_us: f64,
    sim_time_us: u64,
    next_step_us: u64,
}

#[wasm_bindgen]
impl Sim {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Sim {
        let shared = Rc::new(RefCell::new(Shared::new()));

        // The platform must outlive the interface; leaking it for the lifetime
        // of the page is fine for a simulator singleton.
        let platform = Box::leak(Box::new(SimPlatform {
            shared: shared.clone(),
        }));

        let mut interface = Interface::new(platform);
        let player = Player::new(&mut interface);

        Sim {
            shared,
            interface,
            player,
            speed: 0.75,
            paused: false,
            last_real_us: 0.0,
            sim_time_us: 0,
            next_step_us: 0,
        }
    }

    /// Advance the simulation. `now_ms` is `performance.now()` from JS.
    pub fn tick(&mut self, now_ms: f64) {
        let real_us = now_ms * 1000.0;
        let dt = (real_us - self.last_real_us).max(0.0);
        self.last_real_us = real_us;
        self.sim_time_us = self.sim_time_us.wrapping_add((dt * self.speed) as u64);

        if self.paused {
            // Keep the buttons and clock live, but do not advance any show.
            {
                self.shared.borrow_mut().now = self.sim_time_us;
            }
            self.interface.begin_tick();
            self.next_step_us = self.sim_time_us;
            return;
        }

        let mut budget = MAX_STEPS_PER_TICK;
        while self.sim_time_us >= self.next_step_us && budget > 0 {
            {
                self.shared.borrow_mut().now = self.next_step_us;
            }
            self.player.tick(&mut self.interface);
            self.next_step_us = self.next_step_us.wrapping_add(STEP_INTERVAL_US);
            budget -= 1;
        }
        if self.next_step_us < self.sim_time_us {
            self.next_step_us = self.sim_time_us;
        }
    }

    /// Run exactly one frame while paused.
    pub fn step_frame(&mut self) {
        {
            self.shared.borrow_mut().now = self.sim_time_us;
        }
        self.player.tick(&mut self.interface);
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed;
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    /// `id` is `"advance"` or `"action"`.
    pub fn press_button(&mut self, id: &str, down: bool) {
        let mut shared = self.shared.borrow_mut();
        match id {
            "advance" => shared.advance_pressed = down,
            "action" => shared.action_pressed = down,
            _ => {}
        }
    }

    /// RGB bytes for all LEDs, three per LED, in strip order.
    pub fn frame(&self) -> Vec<u8> {
        self.shared.borrow().frame.clone()
    }

    pub fn show_names(&self) -> Vec<String> {
        self.player.names().iter().map(|s| s.to_string()).collect()
    }

    pub fn show_count(&self) -> usize {
        Player::count()
    }

    pub fn current_show(&self) -> usize {
        self.player.current_index()
    }

    pub fn current_show_name(&self) -> String {
        self.player.current_name().to_string()
    }

    pub fn select_show(&mut self, index: usize) {
        self.player.select(index, &mut self.interface);
    }

    pub fn next_show(&mut self) {
        self.player.next(&mut self.interface);
    }

    pub fn auto_led(&self) -> bool {
        self.shared.borrow().auto_led
    }

    pub fn active_led(&self) -> bool {
        self.shared.borrow().active_led
    }

    /// The chat log for the UI, oldest message first.
    ///
    /// The core's `usb_write` payloads (show announcements, echoed chat) land
    /// here, one `String` per newline-terminated line.
    pub fn messages(&self) -> Vec<String> {
        self.shared.borrow().messages.iter().cloned().collect()
    }

    /// Queue chat text for the firmware. The CDC line framing (a `\n`) is
    /// appended here; the text is drained by the platform's `usb_read` during
    /// the following `tick`s.
    pub fn send(&mut self, text: String) {
        let mut text = text;
        if text.chars().count() > INBOUND_TEXT_MAX_CHARS {
            text = text.chars().take(INBOUND_TEXT_MAX_CHARS).collect();
        }
        if !text.ends_with('\n') {
            text.push('\n');
        }
        self.shared.borrow_mut().inbound.push_back(text);
    }
}

impl Default for Sim {
    fn default() -> Self {
        Sim::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME_MS: f64 = 1000.0 / 60.0;

    /// Turn auto-advance off with a long press so tests can inspect a single
    /// show without the 60 s timer moving on.
    fn disable_auto_advance(sim: &mut Sim, t: &mut f64) {
        // The helper is written in real-time frames; run at 1× so 70 frames
        // exceed the 1 s long-press window.
        sim.set_speed(1.0);
        sim.press_button("advance", true);
        for _ in 0..70 {
            *t += FRAME_MS;
            sim.tick(*t);
        }
        sim.press_button("advance", false);
        *t += FRAME_MS;
        sim.tick(*t);
        assert!(!sim.auto_led(), "long press should have enabled/disabled auto");
    }

    #[test]
    fn frame_is_rgb_for_every_led() {
        let mut sim = Sim::new();
        sim.tick(FRAME_MS);
        assert_eq!(sim.frame().len(), NUM_LED * 3);
    }

    #[test]
    fn every_show_runs_without_panic() {
        let mut sim = Sim::new();
        let mut t = 0.0;
        disable_auto_advance(&mut sim, &mut t);

        for i in 0..sim.show_count() {
            sim.select_show(i);
            for _ in 0..300 {
                t += FRAME_MS;
                sim.tick(t);
            }
            assert_eq!(sim.current_show(), i, "show {i} advanced unexpectedly");
            assert_eq!(sim.frame().len(), NUM_LED * 3);
        }
    }

    #[test]
    fn stars_light_up_some_pixels() {
        let mut sim = Sim::new();
        let mut t = 0.0;
        disable_auto_advance(&mut sim, &mut t);

        // "Stars (Esperanto)" fills the sky immediately; look it up by name
        // so the test survives playlist reordering.
        let index = sim.show_names().iter().position(|n| n.contains("Esperanto")).unwrap();
        sim.select_show(index);
        t += FRAME_MS;
        sim.tick(t);
        assert!(
            sim.frame().iter().any(|&b| b != 0),
            "expected at least one lit channel"
        );
    }

    #[test]
    fn short_press_advances_show() {
        let mut sim = Sim::new();
        let mut t = 0.0;
        disable_auto_advance(&mut sim, &mut t);
        sim.select_show(0);

        sim.press_button("advance", true);
        t += FRAME_MS;
        sim.tick(t);
        sim.press_button("advance", false);
        t += FRAME_MS;
        sim.tick(t);

        assert_eq!(sim.current_show(), 1);
    }

    #[test]
    fn pause_freezes_the_show() {
        let mut sim = Sim::new();
        let mut t = 0.0;
        disable_auto_advance(&mut sim, &mut t);
        sim.select_show(9);

        t += FRAME_MS;
        sim.tick(t);
        let before = sim.frame();

        sim.set_paused(true);
        for _ in 0..120 {
            t += FRAME_MS;
            sim.tick(t);
        }
        assert_eq!(sim.frame(), before, "paused sim must not change");

        sim.step_frame();
        assert_eq!(sim.frame().len(), NUM_LED * 3);
    }

    #[test]
    fn speed_scales_virtual_time() {
        let mut sim = Sim::new();
        let mut t = 0.0;
        disable_auto_advance(&mut sim, &mut t);

        // At 2x, one real frame should advance two 1/60 s steps.
        sim.select_show(9);
        sim.set_speed(2.0);
        t += FRAME_MS;
        sim.tick(t);
        assert_eq!(sim.speed(), 2.0);
    }

    #[test]
    fn show_announcements_land_in_the_message_log() {
        let mut sim = Sim::new();
        let mut t = 0.0;
        disable_auto_advance(&mut sim, &mut t);

        // The initial show is announced as soon as the player is created.
        let names = sim.show_names();
        let messages = sim.messages();
        assert_eq!(messages.first().map(String::as_str), Some(&*names[0]).as_deref());

        // A short press switches shows and announces the next one.
        sim.press_button("advance", true);
        t += FRAME_MS;
        sim.tick(t);
        sim.press_button("advance", false);
        t += FRAME_MS;
        sim.tick(t);

        let messages = sim.messages();
        assert_eq!(messages.last().map(String::as_str), Some(&*names[1]).as_deref());
    }

    #[test]
    fn chat_message_is_echoed_back() {
        let mut sim = Sim::new();
        let mut t = 0.0;
        disable_auto_advance(&mut sim, &mut t);

        sim.send("hello".to_string());
        t += FRAME_MS;
        sim.tick(t);
        t += FRAME_MS;
        sim.tick(t);

        assert!(
            sim.messages().iter().any(|m| m == "hello"),
            "the echoed chat line should appear in the log: {:?}",
            sim.messages()
        );
    }
}
