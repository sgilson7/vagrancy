//! The sword is solid against the ground and the walls, a planted sword lifts
//! the fighter, and nothing passes the cap or leaves the arena (D9, D10).

use sim::fx::Fx;
use sim::rng::Rng;
use sim::{Input, World};

fn alone(seed: u64) -> World {
    World::new(content::setup::alone(seed, sim::balance::DEFAULT_TUNING))
}

fn random_runs(mut check: impl FnMut(&World, u32, u64)) {
    for seed in 0..100u64 {
        let mut w = World::new(content::setup::versus(seed, sim::balance::DEFAULT_TUNING));
        let mut r = Rng::new(seed + 1000);
        let mut held = [Input::NONE; 2];
        for t in 0..600 {
            // Keys change about every quarter second, as a person's would.
            if t % 15 == 0 {
                held = [Input(r.below(64) as u16), Input(r.below(64) as u16)];
            }
            w.step(held);
            check(&w, t, seed);
        }
    }
}

#[test]
fn a_planted_sword_lifts_the_fighter() {
    // Bend the elbow, then hold shoulder-down with elbow-out: the tip comes
    // down in front, plants (ticks 45-55 in `lab trace "i:13,0:9,do:40,0:60" 2`),
    // and the push lifts the fighter instead (practice.step.plant). Found by
    // `lab pogo-search` on tuning 2, which measured a 591 cm peak rise of the
    // pelvis, about three body heights. Half of it is the floor here.
    let mut w = alone(1);
    let f = w.fighters[0].clone().unwrap();
    let pel = (f.base + w.setup.bodies[0].roles.pelvis.unwrap() as u16) as usize;
    let tip = w.swords[0].tip as usize;
    let y0 = w.particles[pel].p.y;
    let (mut planted_at, mut peak) = (None, Fx(0));
    let script = [(Input::ELBOW_IN, 13), (0, 9), (Input::SHOULDER_DOWN | Input::ELBOW_OUT, 40), (0, 60)];
    for (b, n) in script {
        for _ in 0..n {
            w.step([Input(b), Input::NONE]);
            if planted_at.is_none() && w.particles[tip].p.y <= Fx::int(1) {
                planted_at = Some(w.tick);
            }
            if planted_at.is_some() {
                peak = peak.max(w.particles[pel].p.y - y0);
            }
        }
    }
    assert!(planted_at.is_some(), "the tip never reached the ground");
    assert!(peak > Fx::int(295), "after the plant at tick {planted_at:?} the pelvis rose only {} cm", peak.trunc());
}

#[test]
fn speed_never_passes_the_cap() {
    random_runs(|w, t, seed| {
        let cap = w.setup.physics.tuning.cap;
        for (i, p) in w.particles.iter().enumerate() {
            let v = (p.p - p.q).len();
            assert!(v <= cap, "seed {seed}, tick {t}: point {i} moves {v:?}, past the cap {cap:?}");
        }
    });
}

#[test]
fn nothing_leaves_the_arena() {
    random_runs(|w, t, seed| {
        for (i, p) in w.particles.iter().enumerate() {
            assert!(p.p.x.abs() <= sim::balance::ARENA_HALF, "seed {seed}, tick {t}: point {i} is past a wall at {:?}", p.p.x);
            assert!(p.p.y >= Fx(0), "seed {seed}, tick {t}: point {i} is below the ground at {:?}", p.p.y);
        }
    });
}

#[test]
fn a_blade_never_ends_a_tick_below_the_ground() {
    random_runs(|w, t, seed| {
        for s in &w.swords {
            for end in [s.butt, s.tip] {
                assert!(w.particles[end as usize].p.y >= Fx(0), "seed {seed}, tick {t}: a sword end is under the ground");
            }
        }
    });
}
