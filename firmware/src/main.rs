//! Firmware entry point for the RP2040 LED flower.
//!
//! All the show logic lives in `lumio-core`; this crate only wires up the
//! hardware [`Platform`](platform::FirmwarePlatform) and runs the player.

#![no_std]
#![no_main]

use core::mem::MaybeUninit;
use core::ptr::addr_of_mut;

use defmt_rtt as _;
use panic_probe as _;

use rp_pico::entry;

use lumio_core::interface::Interface;
use lumio_core::platform::Platform;
use lumio_core::player::Player;

mod platform;
mod usb;

use platform::FirmwarePlatform;

/// The three objects the player loop needs, parked in `.bss` instead of on
/// `main`'s stack.
///
/// `Player` eagerly constructs every show in the playlist, so it is ~90 KiB on
/// its own; together with `Interface` (which embeds the ~20 KiB `LEDStrip`)
/// they do not fit in the stack an RP2040 leaves free once the statics are
/// placed, and the release optimiser (LTO + `opt-level=3`) inflates `main`'s
/// frame even further. Storing them here keeps `main`'s frame to a few words
/// and lets `flip-link` place the data in the 256 KiB of RAM.
///
/// They are only touched from the single-threaded, interrupt-free main loop, so
/// raw `MaybeUninit` globals are fine; no lock is required.
static mut PLATFORM: MaybeUninit<FirmwarePlatform> = MaybeUninit::uninit();
static mut INTERFACE: MaybeUninit<Interface<'static>> = MaybeUninit::uninit();
static mut PLAYER: MaybeUninit<Player> = MaybeUninit::uninit();

/// Borrow the constructed `Player`. Unsafe only because the caller must have
/// written the static in `main`'s prologue first.
unsafe fn player() -> &'static mut Player {
    &mut *(*addr_of_mut!(PLAYER)).as_mut_ptr()
}

/// Borrow the constructed `Interface`.
unsafe fn interface() -> &'static mut Interface<'static> {
    &mut *(*addr_of_mut!(INTERFACE)).as_mut_ptr()
}

/// Borrow the constructed [`FirmwarePlatform`].
unsafe fn platform() -> &'static mut FirmwarePlatform {
    &mut *(*addr_of_mut!(PLATFORM)).as_mut_ptr()
}

#[entry]
fn main() -> ! {
    // Construct once. `FirmwarePlatform::new` takes the RP2040 `Peripherals`
    // singleton, so it may run exactly once, before the loop.
    unsafe {
        (*addr_of_mut!(PLATFORM)).write(FirmwarePlatform::new());

        // `Interface` owns the `&mut dyn Platform`; moving it in means nothing
        // else aliases the platform afterwards.
        let platform_ref: &'static mut dyn Platform = platform();
        (*addr_of_mut!(INTERFACE)).write(Interface::new(platform_ref));

        // Construct the player straight into the static; `Player::new` would
        // return the whole playlist by value and round-trip it through the
        // stack, which the RP2040 cannot afford.
        Player::init_in_place(&mut *addr_of_mut!(PLAYER), interface());
    }

    loop {
        unsafe {
            player().tick(interface());
        }
    }
}
