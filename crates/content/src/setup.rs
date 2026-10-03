//! The setups the page can start, from the data files.

use sim::balance::{self, ROUNDS_TO_WIN};
use sim::body::{Mode, Physics, Seat, Setup};
use sim::fx::Fx;

pub const FIGHTER: u8 = 0;
pub const POST: u8 = 1;

fn setup(seed: u64, mode: Mode, tuning: u8, seats: [Option<Seat>; 2]) -> Setup {
    Setup {
        seed,
        mode,
        rounds_to_win: ROUNDS_TO_WIN,
        physics: Physics::tuned(tuning),
        bodies: crate::body::bodies(),
        seats,
    }
}

/// One fighter alone (M1's page).
pub fn alone(seed: u64, tuning: u8) -> Setup {
    setup(seed, Mode::Practice, tuning, [Some(Seat { body: FIGHTER, x: balance::START_X }), None])
}

/// The practice yard: a fighter and a post that does not fight back.
pub fn practice(seed: u64, tuning: u8) -> Setup {
    setup(
        seed,
        Mode::Practice,
        tuning,
        [Some(Seat { body: FIGHTER, x: balance::START_X }), Some(Seat { body: POST, x: Fx::int(60) })],
    )
}

/// Two fighters and a match.
pub fn versus(seed: u64, tuning: u8) -> Setup {
    let seat = Seat { body: FIGHTER, x: balance::START_X };
    setup(seed, Mode::Match, tuning, [Some(seat), Some(seat)])
}
