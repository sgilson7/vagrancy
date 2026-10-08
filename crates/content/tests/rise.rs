//! The stand key's rise (Sam, 2026-10-08): "instead of automatically
//! resetting you, it gives you sticky feet, which act as a spring trying to
//! get you upright enough to move using a and d like normal again; the
//! spring should take like a second to get you stood up again, up to 2
//! seconds based on how much momentum you have".

use sim::fx::{Fx, V2};
use sim::world::Owner;
use sim::{Input, World};

/// Seat 0 laid on its back on the ground, then sliding along it at `speed`
/// cm a tick, as a fighter knocked flat on the run does; then the stand key
/// held (the rise lasts as long as it is: Sam, 2026-10-08). The ticks until it stands
/// straight and still, and whether a point ever jumped (more than 30 cm in
/// a tick).
fn rise_from_the_ground(speed: i32) -> (Option<u32>, bool) {
    let mut s = content::setup::practice(1, sim::balance::DEFAULT_TUNING);
    s.seats[1] = None;
    let mut w = World::new(s);
    // Lay the body down on its side, still: a quarter turn about the pelvis,
    // which keeps every length (as crates/sim/tests/moves.rs does), then down
    // onto the ground.
    let f = w.fighters[0].clone().unwrap();
    let pel = w.particles[(f.base + w.setup.bodies[f.body as usize].roles.pelvis.unwrap() as u16) as usize].p;
    let ours = |p: &sim::world::Particle| matches!(p.owner, Owner::Body(0) | Owner::Sword(0));
    for p in w.particles.iter_mut().filter(|p| ours(p)) {
        let d = p.p - pel;
        p.p = V2::new(pel.x + d.y, pel.y - d.x);
    }
    let low = w.particles.iter().filter(|p| ours(p)).map(|p| p.p.y - p.rad).min().unwrap();
    for p in w.particles.iter_mut().filter(|p| ours(p)) {
        p.p.y -= low;
        p.q = p.p;
    }
    for _ in 0..30 {
        w.step([Input::NONE, Input::NONE]);
    }
    for i in f.base as usize..(f.base + f.n) as usize {
        w.particles[i].q = w.particles[i].p - V2::new(Fx::int(speed), Fx(0));
    }
    assert!(w.knocked_down(0), "the setup did not knock the fighter down");
    let mut jumped = false;
    for t in 0..240u32 {
        let before: Vec<V2> = w.particles.iter().map(|p| p.p).collect();
        w.step([Input(Input::STAND), Input::NONE]);
        for (i, p) in w.particles.iter().enumerate() {
            if p.owner == Owner::Body(0) && (p.p - before[i]).len() > Fx::int(30) {
                jumped = true;
            }
        }
        if w.fighters[0].as_ref().unwrap().rising == 0 && t > 0 {
            return (Some(t + 1), jumped);
        }
    }
    (None, jumped)
}

#[test]
fn the_stand_key_springs_a_fighter_up_in_one_to_two_seconds_without_a_jump() {
    let (still, jumped) = rise_from_the_ground(0);
    let still = still.expect("a fighter lying still never got up");
    assert!(!jumped, "the rise moved a point more than 30 cm in a tick: a reset, not a spring");
    assert!((45..=85).contains(&still), "from rest the rise took {still} ticks; about a second is 60");
    let (moving, _) = rise_from_the_ground(14);
    let moving = moving.expect("a sliding fighter never got up");
    assert!(moving > still + 10 && moving <= 130, "knocked sliding at 14 cm a tick, three and a half times the run speed, the rise took {moving} ticks against {still} from rest; it should take longer, up to about two seconds");
    eprintln!("rise: {still} ticks from rest, {moving} sliding");
}

#[test]
fn a_tree_pilot_holds_the_stand_key_until_it_is_up_and_then_lets_go() {
    // Sam, 2026-10-08: the rise lasts as long as the key is held, and the
    // trees' stand move has to work with that. The smith's tree stands
    // when it is down (data/pilots.json).
    let mut s = content::setup::practice(1, sim::balance::DEFAULT_TUNING);
    s.seats[1] = None;
    let mut w = World::new(s);
    let f = w.fighters[0].clone().unwrap();
    let pel = w.particles[(f.base + w.setup.bodies[f.body as usize].roles.pelvis.unwrap() as u16) as usize].p;
    let ours = |p: &sim::world::Particle| matches!(p.owner, Owner::Body(0) | Owner::Sword(0));
    for p in w.particles.iter_mut().filter(|p| ours(p)) {
        let d = p.p - pel;
        p.p = V2::new(pel.x + d.y, pel.y - d.x);
    }
    let low = w.particles.iter().filter(|p| ours(p)).map(|p| p.p.y - p.rad).min().unwrap();
    for p in w.particles.iter_mut().filter(|p| ours(p)) {
        p.p.y -= low;
        p.q = p.p;
    }
    let mut pilot = pilot::build(&content::road::pilot("smith"));
    let reaction = match content::road::pilot("smith") {
        pilot::Spec::Tree { reaction_ticks, .. } => reaction_ticks,
        _ => 0,
    };
    let mut last = [Input::NONE; sim::body::SEATS];
    let (mut held, mut up_at, mut let_go) = (0u32, None, None);
    for t in 0..360u32 {
        pilot.observe(last);
        let i = pilot.input(&w, 0);
        if i.has(Input::STAND) && up_at.is_none() {
            held += 1;
        }
        w.step([i, Input::NONE]);
        last = [i, Input::NONE, Input::NONE];
        if up_at.is_none() && !w.knocked_down(0) && w.fighters[0].as_ref().unwrap().rising == 0 {
            up_at = Some(t);
        }
        if up_at.is_some() && let_go.is_none() && !i.has(Input::STAND) {
            let_go = Some(t);
        }
    }
    // The smith's tree vaults on its sword and can fall again after: what is
    // checked is the first time up, and the key let go after it.
    let up = up_at.expect("the smith never got up");
    assert!(held >= 30, "the smith held the stand key only {held} ticks: the rise needs it held");
    assert!(up <= 200, "the smith took {up} ticks to get up");
    // A tree sees the world its reaction time late (data/pilots.json), so it
    // lets go that long after standing, and a little.
    let gone = let_go.expect("the smith never let go of the stand key");
    assert!(gone <= up + 20 + reaction, "the smith let go of the stand key {} ticks after standing", gone - up);
}
