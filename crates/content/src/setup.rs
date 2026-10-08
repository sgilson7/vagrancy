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
        seats: [seats[0], seats[1], None],
        platforms: Vec::new(),
        objective: sim::body::Objective::Rounds,
        arena_half: balance::ARENA_HALF,
    }
}

/// One fighter alone (M1's page).
pub fn alone(seed: u64, tuning: u8) -> Setup {
    setup(seed, Mode::Practice, tuning, [Some(Seat::at(FIGHTER, balance::START_X)), None])
}

/// The player's body carrying `weapon`: the fighter's own body for the
/// sword, or a copy of it with the sword reshaped, added to `bodies`.
pub fn armed(bodies: &mut Vec<sim::body::BodyDef>, weapon: &str) -> u8 {
    match crate::weapons::weapon(weapon) {
        Some(w) if w.id != crate::weapons::DEFAULT => {
            let mut b = bodies[FIGHTER as usize].clone();
            reshape_all(&mut b, &w);
            bodies.push(b);
            (bodies.len() - 1) as u8
        }
        _ => FIGHTER,
    }
}

/// The player's body carrying `weapon`, with four arms and two of it once
/// they are won and chosen (Sam, 2026-10-06: "4 armed mode").
pub fn loadout(bodies: &mut Vec<sim::body::BodyDef>, weapon: &str, four: bool) -> u8 {
    if !four {
        return armed(bodies, weapon);
    }
    let mut b = crate::body::four_armed(true);
    if let Some(w) = crate::weapons::weapon(weapon).filter(|w| w.id != crate::weapons::DEFAULT) {
        reshape_all(&mut b, &w);
    }
    bodies.push(b);
    (bodies.len() - 1) as u8
}

/// Every weapon a body holds (both, with four arms), reshaped as `w`.
pub fn reshape_all(b: &mut sim::body::BodyDef, w: &crate::weapons::Weapon) {
    if let Some(sd) = b.sword.as_ref() {
        b.sword = Some(crate::weapons::reshape(sd, w));
    }
    for sd in &mut b.more {
        *sd = crate::weapons::reshape(sd, w);
    }
}

/// Two players, each carrying their weapon, on a map: at one keyboard or
/// online. An enemy's weapon, or one this build does not know, is the
/// sword; a map it does not know is the flat one.
pub fn versus_with(seed: u64, tuning: u8, weapons: [&str; 2], map: &str) -> Setup {
    let mut s = crate::maps::on(versus(seed, tuning), map);
    for (seat, w) in weapons.iter().enumerate() {
        let w = match crate::weapons::weapon(w) {
            Some(x) if !x.enemy_only => x.id,
            _ => crate::weapons::DEFAULT.to_string(),
        };
        let body = armed(&mut s.bodies, &w);
        s.seats[seat] = Some(Seat::at(body, balance::START_X));
    }
    s
}

/// Two road opponents against each other, each in the body it fights in on
/// the road, the enemies' own weapons and the local deity's arms included:
/// for watch mode (Sam, 2026-10-06) and `lab film`. Neither is a player.
pub fn exhibition(seed: u64, tuning: u8, ids: [&str; 2], map: &str) -> Setup {
    let mut s = crate::maps::on(versus(seed, tuning), map);
    for (seat, id) in ids.iter().enumerate() {
        let body = opponent_body(&mut s.bodies, id);
        s.seats[seat] = Some(Seat::at(body, balance::START_X));
    }
    s
}

/// A duel for a clip (Sam, 2026-10-08: a fight with Dune's shields, won
/// 1 to 0): two fighters' bodies, each carrying its weapon (by name, or as
/// a weapon's JSON), `gap` cm apart at the start (0 for the usual), both
/// with a
/// shield when `shield`, to `rounds` won. Nobody on the road fights this.
pub fn duel(seed: u64, tuning: u8, weapons: [&str; 2], shield: bool, rounds: u32, gap: i32) -> Setup {
    let mut s = versus(seed, tuning);
    s.rounds_to_win = rounds.max(1);
    for (seat, w) in weapons.iter().enumerate() {
        // A weapon by name, or one written out whole, as data/weapons.json
        // writes one: a clip's own blade, which the road never carries.
        let mut b = match serde_json::from_str::<crate::weapons::Weapon>(w) {
            Ok(spec) => {
                let mut b = s.bodies[FIGHTER as usize].clone();
                reshape_all(&mut b, &spec);
                b
            }
            Err(_) => {
                let k = armed(&mut s.bodies, w) as usize;
                s.bodies[k].clone()
            }
        };
        b.shield = shield.then_some(balance::SHIELD_SPEED);
        s.bodies.push(b);
        // Each starts half of `gap` cm from the middle, or where a duel
        // always starts when it is 0.
        let x = if gap > 0 { Fx::int(gap / 2) } else { balance::START_X };
        s.seats[seat] = Some(Seat::at((s.bodies.len() - 1) as u8, x));
    }
    s
}

/// The practice yard with the player carrying `weapon`.
pub fn practice_with(seed: u64, tuning: u8, weapon: &str, four: bool) -> Setup {
    let mut s = practice(seed, tuning);
    let body = loadout(&mut s.bodies, weapon, four);
    s.seats[0] = Some(Seat::at(body, balance::START_X));
    s
}

/// A stop on the road with the player carrying `weapon`.
pub fn road_with(seed: u64, tuning: u8, opponent: &str, weapon: &str, four: bool) -> Setup {
    let mut s = road(seed, tuning, opponent);
    let body = loadout(&mut s.bodies, weapon, four);
    let x = s.seats[0].map(|seat| seat.x).unwrap_or(balance::START_X);
    s.seats[0] = Some(Seat::at(body, x));
    s
}

/// The practice yard: a fighter and a post that does not fight back.
pub fn practice(seed: u64, tuning: u8) -> Setup {
    setup(
        seed,
        Mode::Practice,
        tuning,
        [Some(Seat::at(FIGHTER, balance::START_X)), Some(Seat::at(POST, Fx::int(60)))],
    )
}

/// Two fighters and a match.
pub fn versus(seed: u64, tuning: u8) -> Setup {
    let seat = Seat::at(FIGHTER, balance::START_X);
    setup(seed, Mode::Match, tuning, [Some(seat), Some(seat)])
}

/// A stop on the road: the player in seat 0 against an opponent in seat 1.
/// An opponent the road gives a weapon (the ferryman's longsword) carries it
/// as another body: the fighter's own, its sword reshaped. A flanked stop
/// puts the player in the middle, the opponent on the right and the
/// companion in seat 2 on the left, each as far from the player as two
/// fighters start a duel apart. At half that, a companion who dodged in at
/// the start cut the player on the 25th tick (SECOND-ORDER-M5 row 50).
/// The body an opponent fights in on the road, added to `bodies` if it is
/// not the fighter's own: bare-handed, carrying its weapon, or four-armed.
pub fn opponent_body(bodies: &mut Vec<sim::body::BodyDef>, opponent: &str) -> u8 {
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
        seat1 = armed(bodies, &w);
    }
    // Eight arms, and four of its weapon (the guardian deity).
    if crate::road::stop(opponent).is_some_and(|s| s.eight_arms) {
        let mut b = crate::body::eight_armed();
        if let Some(w) = crate::road::stop(opponent).and_then(|s| s.weapon).and_then(|w| crate::weapons::weapon(&w)) {
            reshape_all(&mut b, &w);
        }
        bodies.push(b);
        seat1 = (bodies.len() - 1) as u8;
    }
    // Four arms: its weapon in one pair of hands, and its second weapon, if
    // it names one, in the other (the village deity).
    if crate::road::stop(opponent).is_some_and(|s| s.four_arms) {
        let mut b = crate::body::four_armed(false);
        if let Some(w) = crate::road::stop(opponent).and_then(|s| s.weapon).and_then(|w| crate::weapons::weapon(&w)) {
            reshape_all(&mut b, &w);
        }
        if let Some(w) = crate::road::stop(opponent).and_then(|s| s.second_weapon).and_then(|w| crate::weapons::weapon(&w)) {
            let base = crate::body::four_armed(false);
            for (sd, base) in b.more.iter_mut().zip(base.more.iter()) {
                *sd = crate::weapons::reshape(base, &w);
            }
        }
        bodies.push(b);
        seat1 = (bodies.len() - 1) as u8;
    }
    // Its costume and armor (data/costumes.json), on a copy: the body may be
    // the player's own.
    if crate::costumes::dressed(opponent) {
        let mut b = bodies[seat1 as usize].clone();
        crate::costumes::dress(opponent, &mut b);
        bodies.push(b);
        seat1 = (bodies.len() - 1) as u8;
    }
    seat1
}

pub fn road(seed: u64, tuning: u8, opponent: &str) -> Setup {
    let mut bodies = crate::body::bodies();
    let mut seat1 = opponent_body(&mut bodies, opponent);
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
    let stop = crate::road::stop(opponent);
    let mut seats = [Some(Seat::at(FIGHTER, balance::START_X)), Some(Seat::at(seat1, balance::START_X)), None];
    if let Some(c) = stop.as_ref().and_then(|s| s.companion.clone()) {
        let body = match c.weapon {
            Some(w) => armed(&mut bodies, &w),
            None => FIGHTER,
        };
        seats[0] = Some(Seat::at(FIGHTER, Fx(0)));
        seats[1] = seats[1].map(|s| Seat { x: balance::START_X * 2, ..s });
        seats[2] = Some(Seat::at(body, balance::START_X * 2));
    }
    let s = Setup { seed, mode: Mode::Match, rounds_to_win, physics, bodies, seats, platforms: Vec::new(), objective: sim::body::Objective::Rounds, arena_half: balance::ARENA_HALF };
    crate::maps::on(s, stop.and_then(|s| s.map).as_deref().unwrap_or(crate::maps::FLAT))
}
