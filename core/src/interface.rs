use core::fmt::{self, Write as _};

use crate::button::{Button, ButtonState};
use crate::ledstrip::LEDStrip;
use crate::platform::{ButtonId, Platform, StatusLedId};
use crate::random::Random;
use crate::showtimer::ShowTimer;

/// Size of the stack-allocated USB line buffer owned by [`Interface`]. Enough
/// for a verbose per-LED debug line plus a newline.
const USB_TX_LINE: usize = 120;

/// [`core::fmt::Write`] target that appends into the line buffer borrowed from
/// [`Interface`] and flushes the accumulated bytes to [`Platform::usb_write`]
/// when dropped.
///
/// Obtained via [`Interface::usb_writer`], which lets the standard
/// `write!`/`writeln!` macros format straight into the USB CDC port:
///
/// ```text
/// writeln!(iface.usb_writer(), "x = {}", x);
/// ```
pub struct UsbWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
    platform: &'a mut dyn Platform,
}

impl fmt::Write for UsbWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let room = self.buf.len() - self.pos;
        let n = bytes.len().min(room);
        self.buf[self.pos..self.pos + n].copy_from_slice(&bytes[..n]);
        self.pos += n;
        if n == bytes.len() {
            Ok(())
        } else {
            // Stop formatting on overflow; the partial line is still flushed.
            Err(fmt::Error)
        }
    }
}

impl Drop for UsbWriter<'_> {
    fn drop(&mut self) {
        let _ = self.platform.usb_write(&self.buf[..self.pos]);
    }
}

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
    usb_buf: [u8; USB_TX_LINE],
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
            usb_buf: [0; USB_TX_LINE],
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

    // -- USB CDC passthrough -------------------------------------------------

    /// Service the USB CDC stack. Call once per frame, before any read/write.
    pub fn usb_poll(&mut self) {
        self.platform.usb_poll();
    }

    /// Write bytes to the USB CDC serial port.
    pub fn usb_write(&mut self, data: &[u8]) -> usize {
        self.platform.usb_write(data)
    }

    /// Borrow the internal USB line buffer as a [`core::fmt::Write`] target.
    /// The bytes formatted into the returned [`UsbWriter`] are flushed to
    /// [`Platform::usb_write`] when it is dropped, so
    ///
    /// ```text
    /// writeln!(iface.usb_writer(), "x = {}", x);
    /// ```
    ///
    /// sends one newline-terminated line.
    pub fn usb_writer(&mut self) -> UsbWriter<'_> {
        UsbWriter {
            buf: &mut self.usb_buf[..],
            pos: 0,
            platform: &mut *self.platform,
        }
    }

    /// Format one line and send it over USB CDC, appending a newline. The
    /// argument is a pre-built [`core::fmt::Arguments`]; the `usb_writeln!`
    /// macro is the convenient call site:
    ///
    /// ```text
    /// usb_writeln!(iface, "a = {}, b = {}", a, b);
    /// ```
    pub fn usb_writeln(&mut self, args: fmt::Arguments<'_>) {
        let mut writer = self.usb_writer();
        let _ = writer.write_fmt(args);
        let _ = writer.write_str("\n");
    }

    /// Read pending bytes from the USB CDC serial port.
    pub fn usb_read(&mut self, buf: &mut [u8]) -> usize {
        self.platform.usb_read(buf)
    }
}

/// Format-and-send helper: builds [`core::fmt::Arguments`] at the call site and
/// forwards to [`Interface::usb_writeln`], so logging over USB CDC is one line.
///
/// ```text
/// usb_writeln!(iface, "a = {}, b = {}", a, b);
/// usb_writeln!(iface); // blank line
/// ```
///
/// Exported at the crate root, so show modules need `use crate::usb_writeln;`
/// (or reference it as `crate::usb_writeln!(...)`) to bring it into scope.
#[macro_export]
macro_rules! usb_writeln {
    ($iface:expr) => {
        $iface.usb_writeln(format_args!(""))
    };
    ($iface:expr, $($arg:tt)+) => {
        $iface.usb_writeln(format_args!($($arg)+))
    };
}

