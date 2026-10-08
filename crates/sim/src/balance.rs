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

/// How much of a blocked joint's shortfall each relaxation pass makes up
/// against the body. *(guess; M2.0)*
pub const DRIVE_K: Fx = Fx::ratio(1, 4);

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

/// Three candidate tunings for M2.0 (lab recon-m2, pogo-grid). Only tuning 2
/// gives a clean planted pogo (211 cm, about 1.1 body heights), so it is the
/// default until Sam picks by playing; the page takes `?tuning=0|1|2`.
pub const TUNINGS: [Tuning; 3] = [
    Tuning { motor_accel: Fx::ratio(1, 12), motor_speed: Fx::ratio(1, 9), cap: Fx::int(20), drag: Fx::ratio(1, 200) },
    Tuning { motor_accel: Fx::ratio(1, 6), motor_speed: Fx::ratio(1, 7), cap: Fx::int(26), drag: Fx::ratio(1, 300) },
    Tuning { motor_accel: Fx::ratio(1, 3), motor_speed: Fx::ratio(1, 5), cap: Fx::int(34), drag: Fx::ratio(1, 400) },
];
pub const DEFAULT_TUNING: u8 = 2;

/// A blade cuts only when its touching point moves at least this fast against
/// the part, cm per tick (6 cm/tick is 360 cm/s). Without a floor every run
/// of random input, and a fighter standing still, cut itself within a second
/// (`lab self-cuts`). At 4 it equalled the run speed, so a fighter running
/// into a still sword was cut by it, and the scarecrow, "here so that your
/// first cut costs you nothing", cost the yardstick most of its matches
/// (SECOND-ORDER-M5). A swing's tip moves at about 20. PLANNING-BRIEF Part C
/// made this Sam's question; the agent set it and carries it.
pub const MIN_CUT_SPEED: Fx = Fx::int(6);

/// The fastest a blade may meet a shielded body and still cut it (Sam,
/// 2026-10-08, the shields of Dune: the slow blade gets through). Between
/// MIN_CUT_SPEED and this a cut lands; over it the shield stops the blade.
/// A swing's tip moves at about 20, so a cut through a shield is a
/// deliberate, slow one. Only a body given a shield reads it.
pub const SHIELD_SPEED: Fx = Fx::int(10);

/// The jump, the air jump and the dodge (Sam, 2026-10-03; both jumps raised at
/// his request). A jump raises the fighter's upward speed to 8.5 cm/tick,
/// which at 0.2725 cm/tick² rises 8.5² / (2 · 0.2725) = 133 cm; the air jump
/// pushes at least 8, which rises 117. *(guesses until Sam plays them)*
pub const JUMP_SPEED: Fx = Fx::ratio(17, 2);
pub const AIR_JUMP_SPEED: Fx = Fx::int(8);
/// An air jump with a step key held pushes at least this fast toward it.
pub const AIR_JUMP_SIDE: Fx = Fx::int(4);
/// A dodge lasts this many ticks, uncuttable and with a harmless blade (18:
/// 10 % longer than the first 16, at Sam's request), and the next cannot
/// start for this many after it began.
pub const DODGE_TICKS: u8 = 18;
pub const DODGE_COOLDOWN: u8 = 45;
/// A roll on the ground: the ground carries the fighter this fast.
pub const ROLL_SPEED: Fx = Fx::int(7);
/// An air dodge sets the fighter's sideways speed to this.
pub const AIR_DODGE_SPEED: Fx = Fx::int(9);

/// How long a round may go on with no blade in it, every sword thrown and
/// down, before it is a draw (Sam, 2026-10-05: "it should end in a draw if
/// no one dies after 4 seconds of throwing their weapons").
pub const DISARMED_DRAW_TICKS: u32 = 4 * TICKS_PER_SECOND;

/// A returning weapon (the boomerang, Sam 2026-10-06) is back in the hand
/// this long after it leaves it.
pub const RETURN_TICKS: u32 = 2 * TICKS_PER_SECOND;
/// For the last this-many ticks of that it flies home on its own; before
/// then it flies as it was thrown.
pub const HOMING_TICKS: u32 = 45;
/// The slowest it flies home, cm per tick (faster when it is farther than
/// this covers in the time left): over MIN_CUT_SPEED, so a returning weapon
/// that has not touched the ground still cuts what it meets.
pub const HOMING_SPEED: Fx = Fx::int(12);
