//! Ledges (Sam, 2026-10-05: "platforms you can jump on top of, that when you
//! execute the s button stand motion it sets you on top of the platform
//! thats immediately below you").

use content::maps::{maps, on};
use sim::{Input, World};

/// One fighter alone on map `id`.
fn alone_on(id: &str) -> World {
    World::new(on(content::setup::alone(1, sim::balance::DEFAULT_TUNING), id))
}

fn pelvis(w: &World) -> (i32, i32) {
    let p = pilot::pelvis(w, 0).unwrap();
    (p.x.trunc(), p.y.trunc())
}

/// Walk under the middle of `[x0, x1]` and stand still there.
fn walk_under(w: &mut World, x0: i32, x1: i32) {
    let mid = (x0 + x1) / 2;
    for _ in 0..900 {
        let (x, _) = pelvis(w);
        let key = if x < mid - 15 { Input::STEP_RIGHT } else if x > mid + 15 { Input::STEP_LEFT } else { 0 };
        w.step([Input(key), Input::NONE]);
        if key == 0 && (w.tick % 30) == 0 {
            break;
        }
    }
    for _ in 0..60 {
        w.step([Input::NONE, Input::NONE]);
    }
    let (x, _) = pelvis(w);
    assert!(x > x0 && x < x1, "the fighter did not get under the ledge from {x0} to {x1}: it stopped at {x}");
}

#[test]
fn every_ledge_is_reached_with_one_jump_and_the_stand_key() {
    // Under each ledge: a jump, sixteen ticks of rising, the stand key, and
    // a second later the fighter stands on the ledge, not the ground.
    for m in maps() {
        for &[x0, x1, y] in &m.platforms {
            let mut w = alone_on(&m.id);
            walk_under(&mut w, x0.max(-560), x1.min(560));
            for t in 0..80 {
                let b = match t {
                    0 => Input::JUMP,
                    16 => Input::STAND,
                    _ => 0,
                };
                w.step([Input(b), Input::NONE]);
            }
            assert!(pilot::on_ledge(&w, 0), "{}: the fighter is not standing on the ledge at {y} cm (pelvis at {:?})", m.id, pelvis(&w));
            let (_, py) = pelvis(&w);
            assert!(py > y, "{}: the pelvis is at {py} cm, under the ledge's top at {y}", m.id);
            // And it stays there: the ledge holds a fighter standing still.
            for _ in 0..180 {
                w.step([Input::NONE, Input::NONE]);
            }
            assert!(pilot::on_ledge(&w, 0), "{}: the fighter sank through the ledge at {y} cm", m.id);
        }
    }
}

#[test]
fn a_ledge_is_passed_through_from_below_and_the_sides() {
    // The bridge's top is lower than a standing fighter's shoulders. Walking
    // in under it from the side, nothing lifts the fighter onto it or stops
    // it; jumping there, the pelvis rises well past the top, which a ledge
    // solid from below would stop.
    let m = content::maps::map("bridge").unwrap();
    let [x0, x1, y] = m.platforms[0];
    let mut w = alone_on("bridge");
    for _ in 0..120 {
        w.step([Input::NONE, Input::NONE]);
    }
    let mut highest_walking = 0;
    while pelvis(&w).0 < (x0 + x1) / 2 && w.tick < 2000 {
        w.step([Input(Input::STEP_RIGHT), Input::NONE]);
        highest_walking = highest_walking.max(pelvis(&w).1);
    }
    assert!(pelvis(&w).0 >= (x0 + x1) / 2, "the fighter could not walk in under the bridge: stopped at {:?}", pelvis(&w));
    assert!(highest_walking < y - 10, "walking under the bridge lifted the pelvis to {highest_walking} cm, near its top at {y}");
    walk_under(&mut w, x0, x1);
    let mut top = 0;
    for t in 0..40 {
        w.step([Input(if t == 0 { Input::JUMP } else { 0 }), Input::NONE]);
        top = top.max(pelvis(&w).1);
    }
    assert!(top > y + 40, "the pelvis rose to {top} cm under a ledge at {y}: the ledge stopped it from below");
}

#[test]
fn over_open_ground_the_stand_key_does_nothing_in_the_air() {
    // The stand sets a fighter on the ledge below it; with no ledge below,
    // a jump goes on as it did (SECOND-ORDER-M5 row 48).
    let mut w = alone_on("flat");
    for _ in 0..120 {
        w.step([Input::NONE, Input::NONE]);
    }
    let mut ys = Vec::new();
    for t in 0..20 {
        w.step([Input(match t {
            0 => Input::JUMP,
            10 => Input::STAND,
            _ => 0,
        }), Input::NONE]);
        ys.push(pelvis(&w).1);
    }
    assert!(ys[19] > ys[10] + 10, "the stand key cut the jump short over open ground: pelvis {ys:?}");
}
