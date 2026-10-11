//! Firmware entry point for the RP2040 LED flower.
//!
//! All the show logic lives in `lumio-core`; this crate only wires up the
//! hardware [`Platform`](platform::FirmwarePlatform) and runs the player.

#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use rp_pico::entry;

use lumio_core::interface::Interface;
use lumio_core::player::Player;

mod platform;

#[entry]
fn main() -> ! {
    let mut platform = platform::FirmwarePlatform::new();
    let mut interface = Interface::new(&mut platform);
    let mut player = Player::new(&mut interface);

    loop {
        player.tick(&mut interface);
    }
}
