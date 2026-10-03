//! Only arm bits drive a joint motor; the step bits move a fighter only
//! through the ground (D6; PLAN.md §8 Q9; D8 as Sam revised it).

use sim::body::Motor;
use sim::world::Owner;
use sim::{Input, World};

fn alone() -> World {
    World::new(content::setup::alone(1, sim::balance::DEFAULT_TUNING))
}

#[test]
fn only_arm_bits_drive_a_joint_motor() {
    let w = alone();
    let f = w.fighters[0].as_ref().unwrap();
    let def = &w.setup.bodies[f.body as usize];
    let root = |i: Option<u8>| f.base + i.unwrap() as u16;
    let forbidden = [root(def.roles.pelvis), root(def.roles.head), root(def.roles.shoulder)]
        .into_iter()
        .chain(def.roles.feet.iter().map(|&i| f.base + i as u16))
        .collect::<Vec<_>>();
    let arm = Input::SHOULDER_UP | Input::SHOULDER_DOWN | Input::ELBOW_IN | Input::ELBOW_OUT;
    for b in 0..=0x1FFu16 {
        let plan = w.motor_plan(0, Input(b));
        let arm_axes = Input(b).axis(Input::SHOULDER_UP, Input::SHOULDER_DOWN) != 0
            || Input(b).axis(Input::ELBOW_IN, Input::ELBOW_OUT) != 0;
        assert_eq!(!plan.is_empty(), arm_axes, "input {b:08b}: a motor ran iff an arm key asked for one");
        if b & arm == 0 {
            assert!(plan.is_empty(), "input {b:08b} holds no arm bit and drove {plan:?}");
        }
        for (motor, _, pivot, set) in plan {
            assert!(matches!(motor, Motor::Shoulder | Motor::Elbow));
            for k in set.iter().chain([&pivot]) {
                let is_sword = matches!(w.particles[*k as usize].owner, Owner::Sword(_));
                assert!(is_sword || !forbidden.contains(k) || *k == pivot, "input {b:08b} pushed point {k}");
                assert!(!w.particles[*k as usize].foot, "input {b:08b} drove a foot");
            }
        }
    }
}

#[test]
fn the_step_bits_move_a_fighter_only_through_the_ground() {
    // On the ground, the step key carries the fighter.
    let mut w = alone();
    let pel = |w: &World| {
        let f = w.fighters[0].as_ref().unwrap();
        w.particles[(f.base + w.setup.bodies[0].roles.pelvis.unwrap() as u16) as usize].p
    };
    let x0 = pel(&w).x;
    for _ in 0..60 {
        w.step([Input(Input::STEP_RIGHT), Input::NONE]);
    }
    assert!((pel(&w).x - x0).trunc() > 100, "a second of step-right moved the pelvis {:?}", pel(&w).x - x0);
    assert!(pel(&w).y.trunc() > 80, "and the fighter is still standing");

    // In the air, with nothing to push against, the step keys change nothing
    // at all: the world is the same to the last bit as one given no input.
    let mut setup = content::setup::alone(1, sim::balance::DEFAULT_TUNING);
    setup.physics.ground = false;
    setup.physics.gravity = sim::fx::Fx(0);
    let (mut a, mut b) = (World::new(setup.clone()), World::new(setup));
    for _ in 0..120 {
        a.step([Input(Input::STEP_LEFT), Input::NONE]);
        b.step([Input::NONE, Input::NONE]);
    }
    assert_eq!(a.checksum(), b.checksum(), "in the air, a step key moved something");
}
