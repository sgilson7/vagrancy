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
    // 8.5 cm/tick upward at 0.2725 cm/tick² rises 8.5² / 0.545 = 133 cm by
    // hand; the body is a frame of sticks, not a point, so allow some give.
    let mut w = alone();
    let top = run(&mut w, &[(0, 30), (Input::JUMP, 1), (0, 70)]);
    assert!(top > Fx::int(110) && top < Fx::int(155), "a jump rose {} cm", top.trunc());
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
    run(&mut again, &[(0, 30), (Input::JUMP, 1), (0, 20), (Input::JUMP, 1), (0, 220)]);
    // A landing from two jumps up can knock a fighter over; stand first.
    assert_eq!(again.fighters[0].as_ref().unwrap().air_jumps, 1, "landing did not give the air jump back");
    // (The landing can leave it propped high on its own sword, so stand, then
    // measure the jumps from where it stands. A rise takes one to two
    // seconds: wait for it.)
    run(&mut again, &[(Input::STAND, 1), (0, 130)]);
    let after = run(&mut again, &[(Input::JUMP, 1), (0, 20), (Input::JUMP, 1), (0, 80)]);
    assert!(after > single + Fx::int(30), "the air jump did not come back on landing: single {:?}, after {:?}, down {}", single.trunc(), after.trunc(), again.knocked_down(0));
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

/// The fighter's center-of-mass velocity, its body and held sword.
fn com_v(w: &World) -> sim::fx::V2 {
    let (mut x, mut y, mut m) = (0i64, 0i64, 0i64);
    for p in &w.particles {
        if p.m == 0 || !matches!(p.owner, sim::world::Owner::Body(0) | sim::world::Owner::Sword(0)) {
            continue;
        }
        let v = p.p - p.q;
        x += p.m as i64 * v.x.0 as i64;
        y += p.m as i64 * v.y.0 as i64;
        m += p.m as i64;
    }
    sim::fx::V2::new(Fx((x / m) as i32), Fx((y / m) as i32))
}

fn set_velocity(w: &mut World, v: sim::fx::V2) {
    for p in w.particles.iter_mut() {
        if matches!(p.owner, sim::world::Owner::Body(0) | sim::world::Owner::Sword(0)) {
            p.q = p.p - v;
        }
    }
}

#[test]
fn an_air_jump_reflects_a_fall_into_a_rise() {
    // High in the air and falling at 12 cm/tick, faster than the air jump's
    // own push of 8: the second jump bounces off an invisible floor, so the
    // fall comes back as a rise of 12 (elastic), less one tick of gravity.
    let mut w = alone();
    run(&mut w, &[(0, 30), (Input::JUMP, 1), (0, 12)]);
    set_velocity(&mut w, sim::fx::V2::new(Fx(0), -Fx::int(12)));
    w.step([Input(Input::JUMP), Input::NONE]);
    let after = com_v(&w);
    let expected = Fx::int(12) - w.setup.physics.gravity;
    assert!((after.y - expected).abs() < Fx::ratio(1, 2), "falling at 12, the air jump gave {:?}, expected about {:?}", after.y, expected);
    // Slower than the push, the push wins: falling at 3 becomes rising at 8.
    let mut w = alone();
    run(&mut w, &[(0, 30), (Input::JUMP, 1), (0, 12)]);
    set_velocity(&mut w, sim::fx::V2::new(Fx(0), -Fx::int(3)));
    w.step([Input(Input::JUMP), Input::NONE]);
    let after = com_v(&w);
    let expected = sim::balance::AIR_JUMP_SPEED - w.setup.physics.gravity;
    assert!((after.y - expected).abs() < Fx::ratio(1, 2), "falling at 3, the air jump gave {:?}, expected about {:?}", after.y, expected);
}

#[test]
fn an_air_jump_toward_a_side_turns_a_drift_round() {
    // Drifting left at 6 cm/tick in the air, push off toward the right: the
    // leftward motion is reflected, so the fighter moves right at 6.
    let mut w = alone();
    run(&mut w, &[(0, 30), (Input::JUMP, 1), (0, 8)]);
    set_velocity(&mut w, sim::fx::V2::new(-Fx::int(6), Fx::int(2)));
    w.step([Input(Input::JUMP | Input::STEP_RIGHT), Input::NONE]);
    let after = com_v(&w);
    assert!((after.x - Fx::int(6)).abs() < Fx::ratio(1, 2), "a leftward drift of 6 became {:?}, not about 6 to the right", after.x);
}

#[test]
fn a_fighters_own_blade_never_cuts_that_fighter() {
    // Sam removed self-cuts (2026-10-03).
    let w = World::new(content::setup::versus(1, sim::balance::DEFAULT_TUNING));
    for (pi, part) in w.parts.iter().enumerate() {
        let own = w.swords.iter().position(|s| s.fighter == part.fighter).unwrap();
        assert!(!w.may_cut(own, pi), "a fighter's own sword may cut its part {pi}");
    }
    for seed in 0..60u64 {
        let mut w = World::new(content::setup::versus(seed, sim::balance::DEFAULT_TUNING));
        let mut r = sim::rng::Rng::new(seed + 8);
        for _ in 0..600 {
            w.step([Input(r.below(512) as u16), Input(r.below(512) as u16)]);
            for e in &w.events {
                if let sim::fight::Event::Cut { seat, by, .. } = e {
                    assert_ne!(seat, by, "seed {seed}: a fighter's own blade cut it");
                }
            }
        }
    }
}

/// Lay the fighter down on its side, still: a quarter turn about the pelvis,
/// which keeps every length, then down onto the ground.
fn knock_down(w: &mut World) {
    let pel = pelvis(w);
    let ours = |p: &sim::world::Particle| matches!(p.owner, sim::world::Owner::Body(0) | sim::world::Owner::Sword(0));
    for p in w.particles.iter_mut().filter(|p| ours(p)) {
        let d = p.p - pel;
        p.p = sim::fx::V2::new(pel.x + d.y, pel.y - d.x);
    }
    let low = w.particles.iter().filter(|p| ours(p)).map(|p| p.p.y - p.rad).min().unwrap();
    for p in w.particles.iter_mut().filter(|p| ours(p)) {
        p.p.y -= low;
        p.q = p.p;
    }
}

#[test]
fn a_knocked_down_fighter_stands_up_on_the_key() {
    let mut w = alone();
    run(&mut w, &[(0, 20)]);
    knock_down(&mut w);
    // The key starts a rise: the feet stick and a spring brings the
    // fighter up over about a second (Sam, 2026-10-08; crates/content/tests/
    // rise.rs measures it). Twenty ticks in it is still coming up.
    run(&mut w, &[(0, 2)]);
    assert!(w.knocked_down(0), "the fighter should be down");
    run(&mut w, &[(Input::STAND, 1), (0, 90)]);
    assert!(!w.knocked_down(0), "the stand key did not stand the fighter up");
    assert!(pelvis(&w).y > Fx::int(80), "standing, the pelvis is only at {} cm", pelvis(&w).y.trunc());
}

#[test]
fn the_stand_key_does_nothing_to_a_fighter_on_its_feet() {
    let (mut a, mut b) = (alone(), alone());
    run(&mut a, &[(0, 30), (Input::STAND, 1), (0, 30)]);
    run(&mut b, &[(0, 30), (0, 1), (0, 30)]);
    let pa: Vec<_> = a.particles.iter().map(|p| p.p).collect();
    let pb: Vec<_> = b.particles.iter().map(|p| p.p).collect();
    assert_eq!(pa, pb, "pressing stand while standing moved the fighter");
}
