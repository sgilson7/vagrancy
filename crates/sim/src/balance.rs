//! Every number that decides how the fight feels, in one place.
//!
//! The page reads these through the shim; nothing outside `sim` keeps a copy.
//! Values marked *(guess)* have not been measured yet. Each recon replaces
//! a guess with a number and says which command produced it.

use crate::fx::Fx;

/// Ticks per second (D5). The page owns the clock and steps this often.
pub const TICKS_PER_SECOND: u32 = 60;
/// Rounds a fighter must win to win the match (D13; Sam, Q8).
pub const ROUNDS_TO_WIN: u32 = 3;
/// Constraint relaxation passes per tick, in a fixed order (D4). *(guess)*
pub const ITERATIONS: u32 = 8;

/// Half the arena's width, in cm. `x = 0` is the middle.
pub const ARENA_HALF: Fx = Fx::int(600);
/// Where each fighter's feet start, either side of the middle. *(guess)*
pub const START_X: Fx = Fx::int(260);
/// The spawn jitter either way, in whole cm, drawn once per round from the
/// world's Rng and applied mirrored so the seats stay fair (D16).
pub const SPAWN_JITTER_CM: i32 = 6;

/// 981 cm/s² at 60 ticks a second: 981 / 3600 = 0.2725 cm per tick², which
/// is 1,116 raw units (PLAN.md D3, by hand).
pub const GRAVITY: Fx = Fx::ratio(981, 3600);

/// How strongly the balance rule straightens the torso and centers the
/// pelvis over the feet, each pass. *(guess)*
pub const BALANCE_K: Fx = Fx::ratio(1, 6);
/// The ground's grip: the share of this tick's sideways travel a point on
/// the ground gives back, in each relaxation pass. A foot and a planted tip
/// hold; anything else lying on the ground slides a little. *(guess)*
pub const GRIP_FOOT: Fx = Fx::ratio(9, 10);
pub const GRIP_TIP: Fx = Fx::ratio(9, 10);
pub const GRIP_BODY: Fx = Fx::ratio(1, 10);

/// How fast the ground carries a fighter whose step key is held, in cm per
/// tick (D8 as revised: quick movement on rigid legs). 4 cm/tick is
/// 240 cm/s, about five seconds across the arena. *(guess; M1.0)*
pub const RUN_SPEED: Fx = Fx::int(4);

/// The free-energy rule's numbers (D9). Sam chooses among the tunings by
/// playing (M2.0); the practice yard can switch between them.
#[derive(Copy, Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Tuning {
    /// Angular acceleration a held joint key gives the limb beyond it, in
    /// radians per tick², as `Fx`.
    pub motor_accel: Fx,
    /// The joint stops pushing once the limb turns this fast (rad/tick).
    pub motor_speed: Fx,
    /// No particle moves faster than this, in cm per tick.
    pub cap: Fx,
    /// The share of every particle's speed taken away per tick.
    pub drag: Fx,
}

/// Three candidate tunings for M2.0. Index 1 is the default until Sam picks.
pub const TUNINGS: [Tuning; 3] = [
    Tuning { motor_accel: Fx::ratio(1, 300), motor_speed: Fx::ratio(1, 9), cap: Fx::int(20), drag: Fx::ratio(1, 200) },
    Tuning { motor_accel: Fx::ratio(1, 200), motor_speed: Fx::ratio(1, 7), cap: Fx::int(26), drag: Fx::ratio(1, 300) },
    Tuning { motor_accel: Fx::ratio(1, 140), motor_speed: Fx::ratio(1, 5), cap: Fx::int(34), drag: Fx::ratio(1, 400) },
];
pub const DEFAULT_TUNING: u8 = 1;
