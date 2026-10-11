use crate::button::{Button, ButtonState};
use crate::ledstrip::LEDStrip;
use crate::platform::{ButtonId, Platform, StatusLedId};
use crate::random::Random;
use crate::showtimer::ShowTimer;

/// The bridge between the shows and a concrete [`Platform`].
///
/// Everything a show needs is available here. The hardware-specific
/// construction lives in the firmware and simulator crates; this type only
/// needs a `&mut dyn Platform`.
pub struct Interface<'p> {
    led_strip: LEDStrip,
    showtimer: ShowTimer,
    button: Button,
    random: Random,
    platform: &'p mut dyn Platform,
    action_state: ButtonState,
    do_next: bool,
}

impl<'p> Interface<'p> {
    pub fn new(platform: &'p mut dyn Platform) -> Interface<'p> {
        let now = platform.now_micros();
        Interface {
            led_strip: LEDStrip::new(),
            showtimer: ShowTimer::new(now),
            button: Button::new(),
            random: Random::new(423434859),
            platform,
            action_state: ButtonState::Up,
            do_next: false,
        }
    }

    pub fn led_strip(&mut self) -> &mut LEDStrip {
        &mut self.led_strip
    }

    pub fn random(&mut self) -> &mut Random {
        &mut self.random
    }

    /// Whether the player should advance to the next show. Refreshed once per
    /// frame by [`Interface::begin_tick`].
    pub fn do_next(&self) -> bool {
        self.do_next
    }

    /// The action button's debounced state.
    pub fn button_state(&self) -> ButtonState {
        self.action_state
    }

    pub fn led_on(&mut self) {
        self.platform.set_status_led(StatusLedId::Active, true);
    }

    pub fn led_off(&mut self) {
        self.platform.set_status_led(StatusLedId::Active, false);
    }

    /// Refresh the clock and both buttons. Must be called once at the start of
    /// every simulation frame.
    pub fn begin_tick(&mut self) {
        let now = self.platform.now_micros();

        let advance_pressed = self.platform.raw_button(ButtonId::Advance);
        self.do_next = self.showtimer.update(advance_pressed, now);
        let auto_show = self.showtimer.auto_show();
        self.platform.set_status_led(StatusLedId::Auto, auto_show);

        let action_pressed = self.platform.raw_button(ButtonId::Action);
        self.action_state = self.button.update(action_pressed, now);
    }

    /// Build the two APA102 byte streams and hand them to the platform.
    pub fn write_spi(&mut self) {
        // Destructure so the framebuffer and the platform can be borrowed at
        // the same time.
        let Interface {
            led_strip, platform, ..
        } = self;
        led_strip.refresh();
        let (a, b) = led_strip.bytes();
        platform.present(a, b);
    }

    pub fn delay_ms(&mut self, delay: u32) {
        self.platform.delay_ms(delay);
    }
}
