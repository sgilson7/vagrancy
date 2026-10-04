//! The setups the page can start, from the data files.

use crate::road::Condition;
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

/// The player's body carrying `weapon`: the fighter's own body for the
/// sword, or a copy of it with the sword reshaped, added to `bodies`.
fn armed(bodies: &mut Vec<sim::body::BodyDef>, weapon: &str) -> u8 {
    match crate::weapons::weapon(weapon) {
        Some(w) if w.id != crate::weapons::DEFAULT => {
            let mut b = bodies[FIGHTER as usize].clone();
            if let Some(sd) = b.sword.as_ref() {
                b.sword = Some(crate::weapons::reshape(sd, &w));
            }
            bodies.push(b);
            (bodies.len() - 1) as u8
        }
        _ => FIGHTER,
    }
}

/// Two players, each carrying their weapon: at one keyboard or online. An
/// enemy's weapon, or one this build does not know, is the sword.
pub fn versus_with(seed: u64, tuning: u8, weapons: [&str; 2]) -> Setup {
    let mut s = versus(seed, tuning);
    for (seat, w) in weapons.iter().enumerate() {
        let w = match crate::weapons::weapon(w) {
            Some(x) if !x.enemy_only => x.id,
            _ => crate::weapons::DEFAULT.to_string(),
        };
        let body = armed(&mut s.bodies, &w);
        s.seats[seat] = Some(Seat { body, x: balance::START_X });
    }
    s
}

/// The practice yard with the player carrying `weapon`.
pub fn practice_with(seed: u64, tuning: u8, weapon: &str) -> Setup {
    let mut s = practice(seed, tuning);
    let body = armed(&mut s.bodies, weapon);
    s.seats[0] = Some(Seat { body, x: balance::START_X });
    s
}

/// A stop on the road with the player carrying `weapon`.
pub fn road_with(seed: u64, tuning: u8, opponent: &str, weapon: &str) -> Setup {
    let mut s = road(seed, tuning, opponent);
    let body = armed(&mut s.bodies, weapon);
    s.seats[0] = Some(Seat { body, x: balance::START_X });
    s
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
/// An opponent the road gives a weapon (the ferryman's longsword) carries it
/// as another body: the fighter's own, its sword reshaped.
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
    // The weapon the opponent carries, if the road names one.
    if let Some(w) = crate::road::stop(opponent).and_then(|s| s.weapon) {
        seat1 = armed(&mut bodies, &w);
    }
    // The fight's condition, if it has one (data/road.json).
    let rounds_to_win = ROUNDS_TO_WIN;
    let mut physics = Physics::tuned(tuning);
    match crate::road::stop(opponent).and_then(|s| s.condition) {
        Some(Condition::DeepInk) => {
            let mut deep = bodies[seat1 as usize].clone();
            deep.ink *= 2;
            bodies.push(deep);
            seat1 = (bodies.len() - 1) as u8;
        }
        Some(Condition::Light) => physics.gravity = physics.gravity.scale(1, 2),
        None => {}
    }
    let seats = [Some(Seat { body: FIGHTER, x: balance::START_X }), Some(Seat { body: seat1, x: balance::START_X })];
    Setup { seed, mode: Mode::Match, rounds_to_win, physics, bodies, seats }
}
