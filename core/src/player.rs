//! The steppable show player.
//!
//! Each show is a state machine whose [`Show::step`] renders exactly one frame.
//! That lets the same playlist run in the firmware (a tight loop) and in the
//! simulator (one frame per `requestAnimationFrame`).

use crate::interface::Interface;
use crate::led::{DARK_BLUE, DARK_GREEN, WHITE, YELLOW};

use crate::fire::Fire;
use crate::particle_crash::ParticleCrash;
use crate::planets::PlanetShow;
use crate::sine_show::{SeaWave, SineShow};
use crate::snake::SnakeShow;
use crate::sparks::{FireWorks, SnowSparks, SparkFall};
use crate::spiral::HueSpiral;
use crate::stars::Stars;
use crate::snow::SnowShow;
use crate::wave_show::{WaveShow, RandomWaveShow, StaticWaveShow, default_waveshow, constant_waveshow};

/// Longest inbound USB CDC line the player keeps while waiting for a newline.
const USB_LINE_MAX: usize = 64;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum StepResult {
    Running,
    Done,
}

/// A single visual effect, advanced one frame at a time.
pub trait Show {
    /// (Re)initialize the show. Called when the show becomes active.
    fn reset(&mut self, iface: &mut Interface<'_>);

    /// Render exactly one frame.
    fn step(&mut self, iface: &mut Interface<'_>) -> StepResult;

    /// Human-readable title, shown in the simulator's show list.
    fn title(&self) -> &'static str;
}

macro_rules! define_player {
    (
        $vis:vis struct $player:ident;
        shows: [
            $( ($field:ident, $ty:ty, $ctor:expr) ),* $(,)?
        ]
    ) => {
        /// The ordered playlist of shows.
        $vis struct $player {
            current: usize,
            $( $field: $ty, )*
            /// Incoming USB CDC bytes not yet terminated by a newline.
            rx_buf: [u8; USB_LINE_MAX],
            rx_len: usize,
        }

        impl $player {
            /// Number of shows in the playlist.
            pub const NUM_SHOWS: usize = [ $( stringify!($field) ),* ].len();

            pub fn new(iface: &mut Interface<'_>) -> Self {
                let mut slot = core::mem::MaybeUninit::<Self>::uninit();
                Self::init_in_place(&mut slot, iface);
                unsafe { slot.assume_init() }
            }

            /// Construct the player directly into `slot`, which must be
            /// uninitialized.
            ///
            /// [`Self::new`] returns the whole playlist by value, so on a
            /// target with a small stack (the RP2040) merely calling it costs
            /// the full size of `Player` in `main`'s frame. Writing each field
            /// straight into `slot` instead keeps the peak stack cost at a
            /// single show rather than the entire playlist.
            pub fn init_in_place(
                slot: &mut core::mem::MaybeUninit<Self>,
                iface: &mut Interface<'_>,
            ) {
                let this = slot.as_mut_ptr();
                unsafe {
                    core::ptr::write(core::ptr::addr_of_mut!((*this).current), 0usize);
                    $(
                        core::ptr::write(core::ptr::addr_of_mut!((*this).$field), $ctor);
                    )*
                    core::ptr::write(core::ptr::addr_of_mut!((*this).rx_buf), [0u8; USB_LINE_MAX]);
                    core::ptr::write(core::ptr::addr_of_mut!((*this).rx_len), 0usize);
                }
                let player = unsafe { slot.assume_init_mut() };
                player.reset_current(iface);
                player.announce(iface);
            }

            #[allow(dead_code)]
            pub fn count() -> usize {
                Self::NUM_SHOWS
            }

            /// Titles of all shows, in playlist order.
            #[allow(dead_code)]
            pub fn names(&self) -> [&'static str; Self::NUM_SHOWS] {
                [ $( self.$field.title(), )* ]
            }

            #[allow(dead_code)]
            pub fn current_index(&self) -> usize {
                self.current
            }

            #[allow(dead_code)]
            pub fn current_name(&self) -> &'static str {
                self.names()[self.current]
            }

            fn show_mut(&mut self) -> &mut dyn Show {
                // The shows are dispatched in list order; `self.current` is
                // just a position in that list.
                let mut position = self.current;
                $(
                    if position == 0 {
                        return &mut self.$field;
                    }
                    position -= 1;
                )*
                unreachable!()
            }

            pub fn reset_current(&mut self, iface: &mut Interface<'_>) {
                self.show_mut().reset(iface);
            }

            #[allow(dead_code)]
            pub fn select(&mut self, index: usize, iface: &mut Interface<'_>) {
                if index < Self::count() {
                    self.current = index;
                    self.reset_current(iface);
                }
            }

            pub fn next(&mut self, iface: &mut Interface<'_>) {
                self.current = (self.current + 1) % Self::count();
                self.reset_current(iface);
                self.announce(iface);
            }

            /// Announce the current show over USB CDC as `"<show>\n"`.
            fn announce(&mut self, iface: &mut Interface<'_>) {
                let title = self.current_name();
                let _ = iface.usb_write(title.as_bytes());
                let _ = iface.usb_write(b"\n");
            }

            /// Service the CDC stack and drain inbound bytes. Newline-terminated
            /// ASCII lines are echoed back so a host can see its message arrived.
            fn poll_usb(&mut self, iface: &mut Interface<'_>) {
                iface.usb_poll();

                let mut chunk = [0u8; 32];
                for _ in 0..4{
                    let count = iface.usb_read(&mut chunk);
                    if count == 0 {
                        break;
                    }
                    for &byte in &chunk[..count] {
                        if byte == b'\n' {
                            let mut line = &self.rx_buf[..self.rx_len];
                            while let Some((b'\r', rest)) = line.split_last() {
                                line = rest;
                            }
                            if !line.is_empty() {
                                let _ = iface.usb_write(line);
                                let _ = iface.usb_write(b"\n");
                            }
                            self.rx_len = 0;
                        } else if self.rx_len < USB_LINE_MAX {
                            self.rx_buf[self.rx_len] = byte;
                            self.rx_len += 1;
                        } else {
                            // Line overflow: drop it like a newline would.
                            self.rx_len = 0;
                        }
                    }
                }
            }

            /// Run exactly one simulation frame. Returns `true` if the active
            /// show changed during this frame.
            pub fn tick(&mut self, iface: &mut Interface<'_>) -> bool {
                iface.begin_tick();
                self.poll_usb(iface);
                let done = self.show_mut().step(iface) == StepResult::Done;
                if done || iface.do_next() {
                    iface.led_strip().black();
                    iface.led_off();
                    self.next(iface);
                    true
                } else {
                    false
                }
            }
        }
    };
}

define_player! {
    pub struct Player;
    shows: [
        (static_waves,     StaticWaveShow<3>, StaticWaveShow::<3>::new()),
        (random_waves,     RandomWaveShow<3>, RandomWaveShow::<3>::new()),
        (planets,          PlanetShow,        PlanetShow::new()),
        (particle_spiral,  ParticleCrash,     ParticleCrash::new_spiral()),
        (sineshow,         SineShow,          SineShow::new()),
        (snowshow_90,      SnowShow<90>,      SnowShow::<90>::new()),

        (fire_green,       Fire,              Fire::new_green()),
        (spiral_lift,      HueSpiral,         HueSpiral::new_lift()),
        (sea_chaos,        SeaWave,           SeaWave::new_chaos()),
        (fireworks,        FireWorks,         FireWorks::new()),
        (waves_1,          WaveShow<3>,       default_waveshow()),
        (spiral_swirl,     HueSpiral,         HueSpiral::new_swirl()),
        (snow,             SnowSparks,        SnowSparks::new()),
        (eo_stars,         Stars,             Stars::new(DARK_GREEN, WHITE)),
        (particle_random,  ParticleCrash,     ParticleCrash::new_random()),
        (sea_order,        SeaWave,           SeaWave::new_order()),
        (snake,            SnakeShow,         SnakeShow::new()),
        (waves_2,          WaveShow<3>,       constant_waveshow()),
        (spark_fall,       SparkFall,         SparkFall::new()),
        (eu_stars,         Stars,             Stars::new(DARK_BLUE, YELLOW)),
        (fire_red,         Fire,              Fire::new_red()),
        (snowshow_180,     SnowShow<180>,     SnowShow::<180>::new()),
    ]
}
