//! Hardware-independent logic for the lumio LED flower.
//!
//! This crate contains the LED model, the strip framebuffer, the shows, and the
//! player that drives them. It talks to the outside world only through the
//! [`platform::Platform`] trait, so the exact same code runs on the RP2040
//! firmware and in the WebAssembly simulator.

#![no_std]

pub mod button;
pub mod conf;
pub mod interface;
pub mod led;
pub mod ledstrip;
pub mod math8;
pub mod platform;
pub mod player;
pub mod random;
pub mod showtimer;

// Shows
pub mod fire;
pub mod huewave;
pub mod particle_crash;
pub mod planets;
pub mod sine;
pub mod snake;
pub mod sparks;
pub mod spiral;
pub mod stars;
pub mod snow;

pub use interface::Interface;
pub use player::{Player, Show, StepResult};
