//! The platform seam.
//!
//! Everything the show logic needs from the outside world is expressed by the
//! [`Platform`] trait. The RP2040 firmware implements it with SPI, GPIO and a
//! hardware timer; the WebAssembly simulator implements it with a virtual clock
//! and a framebuffer that gets drawn to a canvas.

/// Monotonic time in microseconds.
pub type Micros = u64;

/// The two physical buttons of the installation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonId {
    /// GPIO 21: short press = next show, long press = toggle auto-advance.
    Advance,
    /// GPIO 20: per-show action button.
    Action,
}

/// The two status LEDs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StatusLedId {
    /// GPIO 10: lit while auto-advance is enabled.
    Auto,
    /// GPIO 11: lit while the current show reports itself as active.
    Active,
}

/// Everything the show logic needs from the outside world.
pub trait Platform {
    /// Current monotonic time in microseconds.
    fn now_micros(&self) -> Micros;

    /// Present one frame. `a` and `b` are the two APA102 byte streams, one per
    /// SPI bus. Called once per simulation frame by
    /// [`crate::interface::Interface::write_spi`].
    fn present(&mut self, a: &[u8], b: &[u8]);

    /// Blocking delay. The simulator treats this as a no-op.
    fn delay_ms(&mut self, ms: u32);

    /// Raw (undebounced) button state: `true` while physically pressed.
    fn raw_button(&self, id: ButtonId) -> bool;

    /// Drive one of the status LEDs.
    fn set_status_led(&mut self, id: StatusLedId, on: bool);

    /// Service the USB CDC stack (host requests, transfers and so on). Call
    /// once per frame; must be polled before a read or write is meaningful.
    ///
    /// The default is a no-op so platforms without USB (the wasm simulator)
    /// are unaffected.
    fn usb_poll(&mut self) {}

    /// Write bytes to the USB CDC serial port. Returns the number of bytes
    /// actually accepted (they may be buffered rather than sent immediately).
    fn usb_write(&mut self, _data: &[u8]) -> usize {
        0
    }

    /// Read any pending bytes from the USB CDC serial port into `buf`.
    /// Returns the number of bytes stored.
    fn usb_read(&mut self, _buf: &mut [u8]) -> usize {
        0
    }
}
