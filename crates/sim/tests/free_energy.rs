//! A swing adds speed from nowhere, and that is the design (D9; CLAUDE.md).
//!
//! H5 is the hand-computed case. H5c is the agent's control (PLAN.md §4):
//! without it H5 could pass because something other than the motor creates
//! momentum, which is the shortcut E.2 warns about.

use sim::fx::Fx;
use sim::{Input, World};

/// Gravity off, no ground, no walls, no drag, the cap out of reach.
fn floating() -> World {
    let mut setup = content::setup::alone(1, sim::balance::DEFAULT_TUNING);
    setup.physics.gravity = Fx(0);
    setup.physics.ground = false;
    setup.physics.walls = false;
    setup.physics.tuning.drag = Fx(0);
    setup.physics.tuning.cap = Fx::int(100_000);
    World::new(setup)
}

/// H5 — momentum from nowhere. A fighter at rest holds one shoulder key for
/// 60 ticks. The textbook says total momentum stays zero; here it does not,
/// and the kinetic energy is above where it started (which, from rest, any
/// motion would give: the momentum half is the half that decides).
#[test]
fn h5_a_held_shoulder_key_gives_the_fighter_momentum_from_nowhere() {
    let mut w = floating();
    assert_eq!(w.momentum(), (0, 0), "the fighter starts at rest");
    let ke0 = w.kinetic();
    for _ in 0..60 {
        w.step([Input(Input::SHOULDER_UP), Input::NONE]);
    }
    let (px, py) = w.momentum();
    let mag = ((px as f64).powi(2) + (py as f64).powi(2)).sqrt() / 4096.0;
    // `cargo run -p lab -- h5` measured (22.7, 138.6) mass·cm/tick at tick 60,
    // |p| = 140.5, nearly straight up: the arm hangs at rest and the key
    // swings it up. The servo injects momentum only while the joint spins up,
    // so |p| is flat after tick 10. Half the measured value is the floor; a
    // motor that pushed back on the torso gives exactly zero (see H5c).
    assert!(mag > 70.0, "total momentum after 60 ticks is only {mag:.1} mass·cm/tick");
    assert!(py > 0, "the momentum points down ({px}, {py}); a held shoulder-up should lift");
    assert!(w.kinetic() > ke0, "the kinetic energy did not rise");
}

/// H5c — the control. In the same world, the step keys and then no keys at
/// all leave total momentum exactly zero: the balance rule, the solver and
/// the rounding create none, so H5's momentum came from the motor.
#[test]
fn h5c_nothing_but_the_arm_motor_creates_momentum() {
    let mut w = floating();
    for t in 0..120 {
        let i = if t < 60 { Input(Input::STEP_RIGHT) } else { Input::NONE };
        w.step([i, Input::NONE]);
        assert_eq!(w.momentum(), (0, 0), "tick {t}: momentum appeared without a motor");
    }
}
