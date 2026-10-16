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
use crate::sine::{SeaWave, SineShow};
use crate::snake::SnakeShow;
use crate::sparks::{FireWorks, SnowSparks, SparkFall};
use crate::spiral::HueSpiral;
use crate::stars::Stars;
use crate::snow::SnowShow;

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
        }

        impl $player {
            /// Number of shows in the playlist.
            pub const NUM_SHOWS: usize = [ $( stringify!($field) ),* ].len();

            pub fn new(iface: &mut Interface<'_>) -> Self {
                let mut player = Self {
                    current: 0,
                    $( $field: $ctor, )*
                };
                player.reset_current(iface);
                player
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
            }

            /// Run exactly one simulation frame. Returns `true` if the active
            /// show changed during this frame.
            pub fn tick(&mut self, iface: &mut Interface<'_>) -> bool {
                iface.begin_tick();
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
        (planets,          PlanetShow,    PlanetShow::new()),
        (particle_spiral,  ParticleCrash, ParticleCrash::new_spiral()),
        (sine,             SineShow,      SineShow::new()),
        (snowshow_90,      SnowShow<90>,  SnowShow::<90>::new()),

        (fire_green,       Fire,          Fire::new_green()),
        (spiral_lift,      HueSpiral,     HueSpiral::new_lift()),
        (sea_chaos,        SeaWave,       SeaWave::new_chaos()),
        (fireworks,        FireWorks,     FireWorks::new()),
        (spiral_swirl,     HueSpiral,     HueSpiral::new_swirl()),
        (snow,             SnowSparks,    SnowSparks::new()),
        (eo_stars,         Stars,         Stars::new(DARK_GREEN, WHITE)),
        (particle_random,  ParticleCrash, ParticleCrash::new_random()),
        (sea_order,        SeaWave,       SeaWave::new_order()),
        (snake,            SnakeShow,     SnakeShow::new()),
        (spark_fall,       SparkFall,     SparkFall::new()),
        (eu_stars,         Stars,         Stars::new(DARK_BLUE, YELLOW)),
        (fire_red,         Fire,          Fire::new_red()),
        (snowshow_180,     SnowShow<180>, SnowShow::<180>::new()),
    ]
}
