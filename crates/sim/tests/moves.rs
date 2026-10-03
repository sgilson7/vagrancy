//! One jump from the ground, one in the air, and a Melee-style dodge (Sam,
//! 2026-10-03).

use sim::fx::Fx;
use sim::{Input, World};

fn alone() -> World {
    World::new(content::setup::alone(1, sim::balance::DEFAULT_TUNING))
}

fn pelvis(w: &World) -> sim::fx::V2 {
    let f = w.fighters[0].as_ref().unwrap();
    w.particles[(f.base + w.setup.bodies[0].roles.pelvis.unwrap() as u16) as usize].p
}

fn run(w: &mut World, script: &[(u16, u32)]) -> Fx {
    let y0 = pelvis(w).y;
    let mut top = Fx(0);
    for &(b, n) in script {
        for _ in 0..n {
            w.step([Input(b), Input::NONE]);
            top = top.max(pelvis(w).y - y0);
        }
    }
    top
}

#[test]
fn a_jump_from_the_ground_lifts_the_fighter() {
    // 7 cm/tick upward at 0.2725 cm/tick² rises 7² / 0.545 = 90 cm by hand;
    // the body is a frame of sticks, not a point, so allow some give.
    let mut w = alone();
    let top = run(&mut w, &[(0, 30), (Input::JUMP, 1), (0, 60)]);
    assert!(top > Fx::int(70) && top < Fx::int(110), "a jump rose {} cm", top.trunc());
}

#[test]
fn a_held_jump_jumps_once() {
    let (mut a, mut b) = (alone(), alone());
    let once = run(&mut a, &[(0, 30), (Input::JUMP, 1), (0, 80)]);
    let held = run(&mut b, &[(0, 30), (Input::JUMP, 81)]);
    assert_eq!(once, held, "holding the key should not jump again");
}

#[test]
fn there_is_one_jump_in_the_air_and_no_second() {
    let mut one = alone();
    let mut two = alone();
    let mut three = alone();
    // Ground jump, then an air jump at the top: higher than a single jump.
    let single = run(&mut one, &[(0, 30), (Input::JUMP, 1), (0, 80)]);
    let double = run(&mut two, &[(0, 30), (Input::JUMP, 1), (0, 20), (Input::JUMP, 1), (0, 80)]);
    // A third press in the air does nothing more.
    let triple = run(&mut three, &[(0, 30), (Input::JUMP, 1), (0, 20), (Input::JUMP, 1), (0, 10), (Input::JUMP, 1), (0, 70)]);
    assert!(double > single + Fx::int(30), "the air jump added only {} cm", (double - single).trunc());
    assert_eq!(triple, double, "a second air jump was allowed");
    // Landing gives the air jump back.
    let mut again = alone();
    run(&mut again, &[(0, 30), (Input::JUMP, 1), (0, 20), (Input::JUMP, 1), (0, 120)]);
    let after = run(&mut again, &[(Input::JUMP, 1), (0, 20), (Input::JUMP, 1), (0, 80)]);
    assert!(after > single + Fx::int(30), "the air jump did not come back on landing");
}

#[test]
fn a_dodge_makes_a_fighter_uncuttable_and_its_blade_harmless_for_a_moment() {
    let mut w = World::new(content::setup::versus(1, sim::balance::DEFAULT_TUNING));
    let neck = w.parts.iter().position(|p| p.fighter == 1 && p.def == 1).unwrap();
    let own_neck = w.parts.iter().position(|p| p.fighter == 0 && p.def == 1).unwrap();
    assert!(w.may_cut(0, neck) && w.may_cut(1, own_neck));
    w.step([Input::NONE, Input(Input::DODGE)]);
    assert!(w.dodging(1));
    assert!(!w.may_cut(0, neck), "a dodging fighter was cuttable");
    assert!(!w.may_cut(1, own_neck), "a dodging fighter's blade could cut");
    for _ in 0..sim::balance::DODGE_TICKS {
        w.step([Input::NONE, Input::NONE]);
    }
    assert!(!w.dodging(1) && w.may_cut(0, neck), "the dodge did not end");
}

#[test]
fn a_dodge_cannot_be_repeated_until_its_cooldown_ends() {
    let mut w = alone();
    w.step([Input(Input::DODGE), Input::NONE]);
    for _ in 0..sim::balance::DODGE_TICKS + 2 {
        w.step([Input::NONE, Input::NONE]);
    }
    w.step([Input(Input::DODGE), Input::NONE]);
    assert!(!w.dodging(0), "a second dodge started inside the cooldown");
    for _ in 0..sim::balance::DODGE_COOLDOWN {
        w.step([Input::NONE, Input::NONE]);
    }
    w.step([Input(Input::DODGE), Input::NONE]);
    assert!(w.dodging(0), "a dodge after the cooldown did not start");
}

#[test]
fn a_roll_carries_a_fighter_along_the_ground() {
    let mut w = alone();
    let x0 = pelvis(&w).x;
    w.step([Input(Input::DODGE | Input::STEP_RIGHT), Input::NONE]);
    for _ in 0..sim::balance::DODGE_TICKS {
        w.step([Input::NONE, Input::NONE]);
    }
    let moved = (pelvis(&w).x - x0).trunc();
    assert!(moved > 60, "a roll moved the fighter {moved} cm");
}

#[test]
fn an_air_dodge_spends_the_air_jump() {
    // Jump, then air-dodge while still rising: the air jump is gone until the
    // fighter lands. (A press of jump later in the run can land on the ground
    // first, where a jump is allowed, so the count is read directly.)
    let mut w = alone();
    let air = |w: &World| w.fighters[0].as_ref().unwrap().air_jumps;
    run(&mut w, &[(0u16, 30), (Input::JUMP, 1), (0, 6)]);
    assert_eq!(air(&w), 1, "in the air after a jump, one air jump is left");
    run(&mut w, &[(Input::DODGE, 1)]);
    assert_eq!(air(&w), 0, "an air dodge did not spend the air jump");
    run(&mut w, &[(0, 120)]);
    assert_eq!(air(&w), 1, "landing did not give it back");
}
