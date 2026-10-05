//! The weapons (Sam, 2026-10-04, after Soul Calibur II's Weapon Master):
//! each keeps its shape and cuts, blades of any shape stay solid against
//! each other, each is unlocked on the road, and the sword is the strongest.

use content::road::{road, stops};
use content::weapons::{weapons, DEFAULT};
use sim::fight::{segments_cross, Event};
use sim::{Input, World};

fn swing_plan(t: u32) -> u16 {
    if t < 60 {
        Input::STEP_RIGHT
    } else if t < 300 {
        Input::SHOULDER_UP | Input::ELBOW_OUT
    } else {
        Input::SHOULDER_DOWN | Input::STEP_RIGHT
    }
}

#[test]
fn every_weapon_holds_its_shape_and_cuts() {
    // Ten seconds of swinging into the post and the ground. A weapon's
    // points are held to its butt and tip by sticks; under that load none
    // wanders from where it belongs by more than a few cm, and every weapon
    // cuts the post.
    for w in weapons() {
        let mut world = World::new(content::setup::practice_with(1, sim::balance::DEFAULT_TUNING, &w.id));
        let s = world.swords.iter().find(|s| s.fighter == 0).unwrap().clone();
        let d = |w: &World, a: u16, b: u16| (w.particles[a as usize].p - w.particles[b as usize].p).len().trunc();
        let rest: Vec<i32> = s.points.iter().map(|&p| d(&world, s.butt, p)).collect();
        let (mut worst, mut cuts) = (0, 0);
        for t in 0..600 {
            world.step([Input(swing_plan(t)), Input::NONE]);
            for (i, &p) in s.points.iter().enumerate() {
                worst = worst.max((d(&world, s.butt, p) - rest[i]).abs());
            }
            cuts += world.events.iter().filter(|e| matches!(e, Event::Cut { by: 0, .. })).count();
        }
        assert!(worst <= 40, "the {} bent {worst} cm out of shape", w.id);
        assert!(cuts > 0, "the {} never cut the post", w.id);
        let shape = match &w.shape {
            content::weapons::Shape::Straight {} => 1,
            content::weapons::Shape::Curve(b) => b.len() + 1,
            content::weapons::Shape::Prongs { prongs, .. } => prongs.len() + 1,
        };
        assert_eq!(s.edges.len(), shape, "the {} has the wrong number of edges", w.id);
    }
}

#[test]
fn blades_of_any_shape_stay_solid_against_each_other() {
    // Every pair of weapons, both seats flailing: at the end of every tick,
    // no edge of one weapon lies across an edge of the other (D10, now for
    // every edge), and every weapon meets another blade somewhere.
    // The player's weapons: an enemy's comes into a match only on the road,
    // and is measured there by the ladder.
    let ids: Vec<String> = weapons().into_iter().filter(|w| !w.enemy_only).map(|w| w.id).collect();
    let mut met = std::collections::BTreeSet::new();
    for a in &ids {
        for b in &ids {
            let mut w = World::new(content::setup::versus_with(3, sim::balance::DEFAULT_TUNING, [a, b], "flat"));
            let mut clashes = 0;
            for t in 0..900u32 {
                let x = (t / 9).wrapping_mul(2654435761);
                let i = [Input(((x >> 7) & 0b11_1111) as u16 | Input::STEP_RIGHT), Input(((x >> 13) & 0b11_1111) as u16 | Input::STEP_LEFT)];
                w.step(i);
                clashes += w.events.iter().filter(|e| matches!(e, Event::Clash { .. })).count();
                let p = |k: u16| w.particles[k as usize].p;
                for &(a0, a1, _) in &w.swords[0].edges {
                    for &(b0, b1, _) in &w.swords[1].edges {
                        assert!(!segments_cross(p(a0), p(a1), p(b0), p(b1)), "tick {t}: the {a} passed through the {b}");
                    }
                }
            }
            if clashes > 0 {
                met.insert(a.clone());
                met.insert(b.clone());
            }
        }
    }
    for id in &ids {
        assert!(met.contains(id), "the {id} never met another blade");
    }
}

#[test]
fn each_weapon_is_won_on_the_road_and_the_enemies_keep_theirs() {
    let ws = weapons();
    let ids = stops();
    for w in &ws {
        match (&w.unlock, w.enemy_only, w.id == DEFAULT) {
            (None, false, true) => {}
            (Some(r), false, false) => assert!(ids.iter().any(|s| s == r.stop()), "the {} is unlocked at {}, which is not on the road", w.id, r.stop()),
            (None, true, false) => {
                assert!(road().iter().any(|s| s.weapon.as_deref() == Some(&w.id)), "no opponent carries the {}, which is the enemies' alone", w.id);
            }
            _ => panic!("the {} is neither the sword, nor won on the road, nor an enemy's", w.id),
        }
    }
    for s in road() {
        if let Some(w) = &s.weapon {
            assert!(ws.iter().any(|x| &x.id == w), "{} carries {w}, which is not a weapon", s.id);
        }
    }
}

#[test]
fn the_sword_is_the_strongest_weapon() {
    // Sam: "the intent is the normal sword is the most powerful / balanced
    // sword". `make weapons` plays the yardstick carrying each weapon a
    // player can carry against a panel of opponents; the sword wins most.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../analysis/weapons.md");
    let md = std::fs::read_to_string(path).expect("analysis/weapons.md: run `make weapons`");
    assert!(md.contains(&format!("fingerprint {}", content::weapons::fingerprint())), "analysis/weapons.md is stale: run `make weapons`");
    let wins = |id: &str| -> u32 {
        let row = md.lines().find(|l| l.starts_with(&format!("| {id} |"))).unwrap_or_else(|| panic!("no row for {id}"));
        row.split('|').nth(2).unwrap().split_whitespace().next().unwrap().parse().unwrap()
    };
    let sword = wins(DEFAULT);
    for w in weapons().iter().filter(|w| !w.enemy_only && w.id != DEFAULT) {
        assert!(wins(&w.id) < sword, "the {} won {} where the sword won {sword}", w.id, wins(&w.id));
    }
}
