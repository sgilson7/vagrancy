//! The hand-computed cases (PLANNING-BRIEF A.4; PLAN.md §4). Each test
//! carries the arithmetic it checks, so a disagreement is between two
//! written-down numbers and not between a number and a feeling.
//!
//! These live inside the crate because a test world (no ground, a single
//! particle, a set starting speed) is built by reaching into `World`, which
//! nothing outside `sim` may do.

use crate::body::{BodyDef, Mode, Physics, PointDef, Roles, Seat, Setup};
use crate::fx::{Fx, V2};
use crate::input::Input;
use crate::world::World;

pub(crate) fn lone_point() -> BodyDef {
    BodyDef {
        points: vec![PointDef { at: V2::cm(0, 10_000), rad: Fx(0) }],
        parts: vec![],
        sticks: vec![],
        hinges: vec![],
        roles: Roles::default(),
        anchored: vec![],
        balance: false,
        ink: 0,
        sword: None,
    }
}

/// A world with nothing in it but the given bodies, no ground and no walls.
pub(crate) fn test_world(bodies: Vec<BodyDef>, seats: [Option<Seat>; 2], gravity: Fx) -> World {
    let mut ph = Physics::tuned(1);
    ph.gravity = gravity;
    ph.ground = false;
    ph.walls = false;
    ph.tuning.drag = Fx(0);
    ph.tuning.cap = Fx::int(100_000);
    World::new(Setup { seed: 1, mode: Mode::Practice, rounds_to_win: 3, physics: ph, bodies, seats: [seats[0], seats[1], None], platforms: Vec::new() })
}

/// H1 — the order of the integrator.
///
/// Starting speed 1 downward, gravity 4 a tick, speed updated first: the
/// steps are 5, 9, 13, …, 4k + 1, and Σ_{k=1..n} (4k + 1) = 2n(n + 1) + n =
/// n(2n + 3). At n = 2 that is 2·7 = 14; at n = 10, 10·23 = 230. Position
/// first would give n(2n − 1) = 6 at n = 2; the Gauss sum n(n + 1)/2 gives
/// 3. In Verlet the starting speed is `q = p + 1` (one centimeter above).
#[test]
fn h1_the_integrator_updates_speed_before_position() {
    for (n, want) in [(2u32, 14), (10, 230)] {
        let mut w = test_world(vec![lone_point()], [Some(Seat { body: 0, x: Fx(0) }), None], Fx::int(4));
        let start = w.particles[0].p.y;
        w.particles[0].q.y = start + Fx::int(1);
        for _ in 0..n {
            w.step([Input::NONE; 2]);
        }
        let fallen = start - w.particles[0].p.y;
        assert_eq!(fallen, Fx::int(want), "after {n} ticks the point has fallen {fallen:?}, and n(2n+3) = {want}");
    }
}

use crate::body::{Cause, FatalBand, Mode as BodyMode, PartDef};
use crate::fight::{Event, Phase};
use crate::world::Owner;

fn part(near: u8, far: u8, mass: i32, drain: i32, fatal: Vec<FatalBand>, copy: &str) -> PartDef {
    PartDef { copy: copy.into(), near, far, radius: Fx::int(3), mass, drain, fatal, motor: None, hand: false }
}

/// A floating figure: heart (0), shoulder... in cm. Parts by index:
/// 0 chest (heart→waist, heart band to 0.6), 1 neck (heart→head, fatal),
/// 2 upper arm (heart→elbow, drains 20), 3 forearm (elbow→wrist, drains 10),
/// 4 limb of mass 8 hanging off the waist (waist→foot).
fn figure(ink: i32) -> BodyDef {
    let pts = [(0, 150), (0, 100), (0, 175), (30, 150), (60, 150), (0, 60)];
    BodyDef {
        points: pts.iter().map(|&(x, y)| PointDef { at: V2::cm(x, y), rad: Fx(0) }).collect(),
        parts: vec![
            part(0, 1, 30, 40, vec![FatalBand { from: Fx(0), to: Fx::ratio(6, 10), cause: Cause::Heart }], "chest"),
            part(0, 2, 2, 0, vec![FatalBand { from: Fx(0), to: Fx::int(1), cause: Cause::Neck }], "neck"),
            part(0, 3, 3, 20, vec![], "arm"),
            part(3, 4, 2, 10, vec![], "arm"),
            part(1, 5, 8, 25, vec![], "leg"),
        ],
        sticks: vec![],
        hinges: vec![],
        roles: Roles::default(),
        anchored: vec![],
        balance: false,
        ink,
        sword: None,
    }
}

fn floating(seats: [Option<Seat>; 2], mode: BodyMode, ink: i32) -> World {
    let mut w = test_world(vec![figure(ink)], seats, Fx(0));
    w.setup.mode = mode;
    w
}

const ONE_SEAT: [Option<Seat>; 2] = [Some(Seat { body: 0, x: Fx(0) }), None];

/// H3 — which side drops. A uniform limb of mass 8, attached at its near
/// end, cut a quarter of the way along: the fighter keeps 8·1/4 = 2 and the
/// piece that drops is 8·3/4 = 6, and it is the far side.
#[test]
fn h3_a_cut_drops_the_far_side_with_its_share_of_the_mass() {
    let mut w = floating(ONE_SEAT, BodyMode::Practice, 1000);
    let foot = w.parts[4].b;
    w.cut(4, Fx::ratio(1, 4), 0);
    assert_eq!(w.parts[4].mass, 2, "the fighter keeps a quarter of the limb");
    assert!(w.parts[4].stump && w.parts[4].attached);
    assert_eq!(w.parts_mass(Owner::Piece(0)), 6, "the piece that drops is the other three quarters");
    assert_eq!(w.particles[foot as usize].owner, Owner::Piece(0), "and it is the far side, the one away from the heart");
}

/// H4 — a stump is replaced, not added. 1,000 ink; a forearm stump drains 10
/// a tick and an upper-arm stump 20; ink drains at the end of every tick,
/// including the tick a cut lands on. Forearm cut on tick 0: at the end of
/// tick 29 the fighter has 1000 − 10·30 = 700. Upper arm of the same arm cut on
/// tick 30, which removes the forearm stump: 700 − 20·(t − 29) = 0 at t = 64.
/// Adding the rates (30 a tick) would say tick 53.
#[test]
fn h4_a_stump_is_replaced_not_added() {
    let mut w = floating(ONE_SEAT, BodyMode::Practice, 1000);
    let ink = |w: &World| w.fighters[0].as_ref().unwrap().ink;
    w.cut(3, Fx::ratio(1, 2), 0); // tick 0
    for _ in 0..30 {
        w.step([Input::NONE; 2]);
    }
    assert_eq!(ink(&w), 700, "at the end of tick 29");
    w.cut(2, Fx::ratio(1, 2), 0); // tick 30
    let mut zero_at = None;
    while zero_at.is_none() && w.tick < 200 {
        w.step([Input::NONE; 2]);
        if ink(&w) == 0 {
            zero_at = Some(w.tick - 1);
        }
    }
    assert_eq!(zero_at, Some(64), "the fighter reaches zero at the end of tick 64, not 53");
}

#[test]
fn a_cut_on_a_piece_that_has_dropped_spills_nothing() {
    let mut w = floating(ONE_SEAT, BodyMode::Practice, 1000);
    w.cut(2, Fx::ratio(1, 2), 0); // the upper arm: the forearm drops with it
    let forearm = 3;
    assert!(!w.parts[forearm].attached);
    let before = w.fighters[0].as_ref().unwrap().spilled.clone();
    w.events.clear();
    w.cut(forearm, Fx::ratio(1, 2), 0);
    assert!(matches!(w.events[..], [Event::Cut { spilled: false, .. }]), "{:?}", w.events);
    w.step([Input::NONE; 2]);
    let after = &w.fighters[0].as_ref().unwrap().spilled;
    assert_eq!(after[forearm], before[forearm], "the dropped forearm spilled nothing");
}

#[test]
fn the_part_that_spilled_most_counts_removed_stumps() {
    // Forearm stump for 60 ticks (600), then the upper arm's for the rest
    // (400 at 20 a tick): the forearm spilled most, though its stump is gone.
    let both = [Some(Seat { body: 0, x: Fx::int(200) }), Some(Seat { body: 0, x: Fx::int(200) })];
    let mut w = floating(both, BodyMode::Match, 1000);
    w.cut(3, Fx::ratio(1, 2), 1);
    for _ in 0..60 {
        w.step([Input::NONE; 2]);
    }
    w.cut(2, Fx::ratio(1, 2), 1);
    while matches!(w.phase, Phase::Fight) && w.tick < 300 {
        w.step([Input::NONE; 2]);
    }
    match w.phase {
        Phase::RoundOver { result, .. } => {
            assert_eq!(result.loser, Some(0));
            assert_eq!(result.cause, Cause::Ink);
            assert_eq!(result.part, 3, "the forearm spilled 600, the upper arm 400");
        }
        p => panic!("the round did not end: {p:?}"),
    }
}

#[test]
fn a_neck_cut_ends_the_round_on_the_tick_it_lands() {
    let both = [Some(Seat { body: 0, x: Fx::int(200) }), Some(Seat { body: 0, x: Fx::int(200) })];
    let mut w = floating(both, BodyMode::Match, 1000);
    for _ in 0..10 {
        w.step([Input::NONE; 2]);
    }
    let neck = w.parts.iter().position(|p| p.fighter == 1 && p.def == 1).unwrap();
    w.cut(neck, Fx::ratio(1, 2), 0); // lands on tick 10
    w.step([Input::NONE; 2]);
    assert_eq!(w.tick, 11);
    match w.phase {
        Phase::RoundOver { result, .. } => {
            assert_eq!((result.loser, result.cause, result.part, result.by), (Some(1), Cause::Neck, 1, 0));
        }
        p => panic!("a neck cut did not end the round on its tick: {p:?}"),
    }
    assert_eq!(w.wins, [1, 0]);
    assert!(w.events.iter().any(|e| matches!(e, Event::RoundEnd { .. })));
}

#[test]
fn a_chest_cut_inside_the_heart_band_ends_the_round_and_one_below_it_does_not() {
    let both = [Some(Seat { body: 0, x: Fx::int(200) }), Some(Seat { body: 0, x: Fx::int(200) })];
    let mut w = floating(both, BodyMode::Match, 1000);
    w.cut(0, Fx::ratio(8, 10), 1); // below the band: the waist and the limb drop
    w.step([Input::NONE; 2]);
    assert!(matches!(w.phase, Phase::Fight), "a cut below the heart band drains, it does not end the round");
    let chest = 0;
    w.cut(chest, Fx::ratio(3, 10), 1);
    w.step([Input::NONE; 2]);
    assert!(matches!(w.phase, Phase::RoundOver { result, .. } if result.cause == Cause::Heart && result.loser == Some(0)));
}

#[test]
fn both_fighters_stopping_on_one_tick_is_a_draw_played_again() {
    let both = [Some(Seat { body: 0, x: Fx::int(200) }), Some(Seat { body: 0, x: Fx::int(200) })];
    let mut w = floating(both, BodyMode::Match, 1000);
    let n0 = w.parts.iter().position(|p| p.fighter == 0 && p.def == 1).unwrap();
    let n1 = w.parts.iter().position(|p| p.fighter == 1 && p.def == 1).unwrap();
    w.cut(n0, Fx::ratio(1, 2), 1);
    w.cut(n1, Fx::ratio(1, 2), 0);
    w.step([Input::NONE; 2]);
    assert!(matches!(w.phase, Phase::RoundOver { result, .. } if result.loser.is_none()));
    assert_eq!(w.wins, [0, 0]);
    // Both press ready: the same round number is played again.
    w.step([Input(Input::READY), Input::NONE]);
    assert!(matches!(w.phase, Phase::RoundOver { .. }), "one seat ready is not enough");
    w.step([Input::NONE, Input(Input::READY)]);
    assert!(matches!(w.phase, Phase::Fight));
    assert_eq!(w.round, 1);
    assert_eq!(w.fighters[0].as_ref().unwrap().ink, 1000, "both fighters are restored");
}

#[test]
fn the_match_ends_when_a_fighter_wins_enough_rounds() {
    let both = [Some(Seat { body: 0, x: Fx::int(200) }), Some(Seat { body: 0, x: Fx::int(200) })];
    let mut w = floating(both, BodyMode::Match, 1000);
    for r in 0..w.setup.rounds_to_win {
        let neck = w.parts.iter().position(|p| p.fighter == 1 && p.def == 1).unwrap();
        w.cut(neck, Fx::ratio(1, 2), 0);
        w.step([Input::NONE; 2]);
        if r + 1 < w.setup.rounds_to_win {
            assert!(matches!(w.phase, Phase::RoundOver { .. }));
            w.step([Input(Input::READY); 2]);
            assert_eq!(w.round, r + 2);
        }
    }
    assert!(matches!(w.phase, Phase::MatchOver { .. }), "{:?}", w.phase);
    assert_eq!(w.wins, [w.setup.rounds_to_win, 0]);
}
