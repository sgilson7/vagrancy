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
    World::new(Setup { seed: 1, mode: Mode::Practice, rounds_to_win: 3, physics: ph, bodies, seats })
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
