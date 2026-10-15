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
}

macro_rules! define_player {
    (
        $vis:vis struct $player:ident;
        shows: [
            $( ($field:ident, $ty:ty, $ctor:expr, $name:literal) ),* $(,)?
        ]
    ) => {
        /// The ordered playlist of shows.
        $vis struct $player {
            current: usize,
            $( $field: $ty, )*
        }

        impl $player {
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
                [ $( $name ),* ].len()
            }

            #[allow(dead_code)]
            pub fn names() -> &'static [&'static str] {
                &[ $( $name ),* ]
            }

            #[allow(dead_code)]
            pub fn current_index(&self) -> usize {
                self.current
            }

            #[allow(dead_code)]
            pub fn current_name(&self) -> &'static str {
                Self::names()[self.current]
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
        (planets,          PlanetShow,    PlanetShow::new(),                        "Planets"),
        (particle_spiral,  ParticleCrash, ParticleCrash::new_spiral(),              "Particle Crash (spiral)"),
        (sine,             SineShow,      SineShow::new(),                          "Sine"),
        (snowshow_90,      SnowShow<90>,  SnowShow::<90>::new(),                    "Snow-90"),

        (fire_green,       Fire,          Fire::new_green(),                        "Fire (green)"),
        (spiral_lift,      HueSpiral,     HueSpiral::new_lift(),                    "Spiral (lift)"),
        (sea_chaos,        SeaWave,       SeaWave::new(0.63, 0.5, Some(9800), 8),   "Sea Wave (chaos)"),
        (fireworks,        FireWorks,     FireWorks::new(),                         "Fireworks"),
        (spiral_swirl,     HueSpiral,     HueSpiral::new_swirl(),                   "Spiral (swirl)"),
        (snow,             SnowSparks,    SnowSparks::new(),                        "Snow"),
        (eo_stars,         Stars,         Stars::new(DARK_GREEN, WHITE),            "Stars (Esperanto)"),
        (particle_random,  ParticleCrash, ParticleCrash::new_random(),              "Particle Crash"),
        (sea_order,        SeaWave,       SeaWave::new(0.99, 0.99, Some(1050), 12), "Sea Wave (order)"),
        (snake,            SnakeShow,     SnakeShow::new(),                         "Snake"),
        (spark_fall,       SparkFall,     SparkFall::new(),                         "Spark Fall"),
        (eu_stars,         Stars,         Stars::new(DARK_BLUE, YELLOW),            "Stars (EU)"),
        (fire_red,         Fire,          Fire::new_red(),                          "Fire (red)"),
        (snowshow_180,     SnowShow<180>, SnowShow::<180>::new(),                   "Snow-180"),
    ]
}
