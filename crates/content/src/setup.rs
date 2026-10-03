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

/// A stop on the road: the player in seat 0 against an opponent in seat 1.
/// An opponent whose pilot names a `sword_len` fights with a longer sword
/// (the ferryman), as a third body: the fighter's own, its sword stretched
/// along its length.
pub fn road(seed: u64, tuning: u8, opponent: &str) -> Setup {
    let mut bodies = crate::body::bodies();
    let mut seat1 = FIGHTER;
    // An unarmed opponent (the scarecrow) stands without a sword: with the
    // longer swords a runner's arm met its still blade fast enough to be cut,
    // which made "your first cut costs you nothing" false (SECOND-ORDER-M5).
    if let pilot::Spec::Still { unarmed: true } = crate::road::pilot(opponent) {
        let mut bare = bodies[0].clone();
        bare.sword = None;
        bodies.push(bare);
        seat1 = (bodies.len() - 1) as u8;
    }
    if let pilot::Spec::Machine { sword_len: Some(len), .. } = crate::road::pilot(opponent) {
        let mut long = bodies[0].clone();
        let s = long.sword.as_mut().expect("the fighter has a sword");
        let axis = s.tip - s.butt;
        let now = axis.len().trunc();
        s.tip = s.butt + axis.scale(len as i64, now as i64);
        // The grips' places along the sword are fractions; keep the hands
        // where they were by rescaling them.
        for g in &mut s.grips {
            g.at = g.at.scale(now as i64, len as i64);
        }
        // A heavier blade: a third more mass at each end.
        s.mass = s.mass * 4 / 3 + 1;
        bodies.push(long);
        seat1 = (bodies.len() - 1) as u8;
    }
    let seats = [Some(Seat { body: FIGHTER, x: balance::START_X }), Some(Seat { body: seat1, x: balance::START_X })];
    Setup { seed, mode: Mode::Match, rounds_to_win: ROUNDS_TO_WIN, physics: Physics::tuned(tuning), bodies, seats }
}
