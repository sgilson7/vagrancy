//! Costumes and armor (Sam, 2026-10-08): "for some of these enemies, like
//! ones with plated helmets and stuff, it should act like a sword in the
//! sense that its rigid, and should defend the area beneath it from getting
//! cut". A plate meets a blade as another blade does (D10) and cuts nothing.

use sim::body::{BodyDef, Seat};
use sim::fight::Event;
use sim::fx::{Fx, V2};
use sim::world::Owner;
use sim::{Input, World};

const HEAD: u8 = 2;
const NECK: u8 = 1;

/// The watchman's body, as the road dresses it, with its sword taken away so
/// nothing but its head and its helmet meets the blade.
fn watchman(helmet: bool) -> BodyDef {
    let mut s = content::setup::road(1, sim::balance::DEFAULT_TUNING, "watchman");
    let mut b = s.bodies.swap_remove(s.seats[1].unwrap().body as usize);
    b.sword = None;
    b.more.clear();
    if !helmet {
        b.armor.clear();
    }
    b
}

/// Seat 1 rises head first, at `speed` cm a tick, into seat 0's still blade,
/// starting `gap` cm below it. What happened to its head, and whether a
/// blade met a plate.
fn rise_into_the_blade(helmet: bool, speed: i32, gap: i32) -> (bool, usize) {
    let mut s = content::setup::versus(1, sim::balance::DEFAULT_TUNING);
    s.bodies.push(watchman(helmet));
    s.seats[1] = Some(Seat::at((s.bodies.len() - 1) as u8, Fx::int(200)));
    s.physics.ground = false;
    let mut w = World::new(s);
    let f1 = w.fighters[1].clone().unwrap();
    let def = w.setup.bodies[f1.body as usize].clone();
    let head = (f1.base + def.roles.head.unwrap() as u16) as usize;
    // A point two thirds of the way up seat 0's blade.
    let k = w.swords_of(0)[0];
    let (butt, tip) = (w.particles[w.swords[k].butt as usize].p, w.particles[w.swords[k].tip as usize].p);
    let at = V2::lerp(butt, tip, Fx::ratio(2, 3));
    // Seat 1's head goes under it, the top of the head `gap` below.
    let shift = V2::new(at.x - w.particles[head].p.x, at.y - Fx::int(11 + gap) - w.particles[head].p.y);
    for p in w.particles.iter_mut() {
        if p.owner == Owner::Body(1) {
            p.p += shift;
            p.q = p.p - V2::new(Fx(0), Fx::int(speed));
        }
    }
    // Seat 0 stands still on nothing: hold it where it is.
    let mut cut = false;
    let mut clashes = 0;
    for _ in 0..20 {
        for p in w.particles.iter_mut() {
            if matches!(p.owner, Owner::Body(0) | Owner::Sword(0)) {
                p.q = p.p;
            }
        }
        w.step([Input::NONE, Input::NONE]);
        for e in &w.events {
            match *e {
                Event::Cut { seat: 1, part, .. } if part == HEAD || part == NECK => cut = true,
                Event::Clash { .. } => clashes += 1,
                _ => {}
            }
        }
    }
    (cut, clashes)
}

#[test]
fn a_blade_that_cuts_a_bare_head_is_stopped_by_a_helmet() {
    let (bare_cut, bare_clashes) = rise_into_the_blade(false, 6, 6);
    assert!(bare_cut, "the control failed: a bare head rising into the blade was not cut");
    assert_eq!(bare_clashes, 0, "a bare head met the blade as a blade does");
    let (cut, clashes) = rise_into_the_blade(true, 6, 6);
    assert!(clashes > 0, "the helmet never met the blade");
    assert!(!cut, "the head under the helmet was cut");
}

#[test]
fn the_watchman_wears_a_helmet_and_the_player_wears_nothing() {
    let s = content::setup::road(1, sim::balance::DEFAULT_TUNING, "watchman");
    let them = &s.bodies[s.seats[1].unwrap().body as usize];
    let me = &s.bodies[s.seats[0].unwrap().body as usize];
    assert_eq!(them.armor.len(), 1, "the watchman's plates");
    assert_eq!(them.costume, "watchman");
    assert!(me.armor.is_empty() && me.costume.is_empty(), "the player is dressed");
    let w = World::new(s);
    assert_eq!(w.armor_of(1).len(), 1);
    assert!(w.armor_of(0).is_empty());
    // A plate is no weapon: the watchman still has its one sword.
    assert_eq!(w.swords_of(1).len(), 1);
}

#[test]
fn every_costume_and_plate_names_something_that_exists() {
    let f = content::costumes::file();
    let ids: Vec<String> = content::road::road().iter().map(|s| s.id.clone()).collect();
    for (id, c) in &f.costumes {
        assert!(ids.contains(id), "costume for {id}, who is not on the road");
        for p in &c.armor {
            assert!(f.plates.contains_key(p), "costume {id}: no plate {p}");
        }
    }
    for (name, p) in &f.plates {
        assert!(content::body::part_index(&p.part).is_some(), "plate {name}: no part {}", p.part);
        assert!(p.points.len() >= 2, "plate {name} has no edge");
    }
    // Most of the road is dressed (Sam: "a majority of the enemies").
    assert!(f.costumes.len() * 10 >= ids.len() * 9, "{} costumes for {} opponents", f.costumes.len(), ids.len());
}

#[test]
fn every_opponent_wears_a_signature_piece_of_its_own() {
    // Sam, 2026-10-08: "a lot of them are wearing roughly the same japanese
    // peasant type garb ... give enemy a signature piece".
    let f = content::costumes::file();
    let mut seen = std::collections::BTreeMap::new();
    for (id, c) in &f.costumes {
        assert!(!c.sig.is_empty(), "{id} wears no signature piece");
        assert!(f.slots.contains_key(&c.sig_on), "{id}'s {} is drawn into a slot that does not exist: {}", c.sig, c.sig_on);
        if let Some(other) = seen.insert(c.sig.clone(), id.clone()) {
            panic!("{id} and {other} both wear the {}", c.sig);
        }
    }
}

#[test]
fn no_two_opponents_in_the_same_hat_and_top_share_a_color() {
    // Sam, 2026-10-08: "vary some of the costume colors to make some similar
    // looking enemies a bit more differentiatable".
    let f = content::costumes::file();
    let mut seen = std::collections::BTreeMap::new();
    for (id, c) in &f.costumes {
        let key = (c.head.clone(), c.chest.clone(), c.main.clone());
        if let Some(other) = seen.insert(key, id.clone()) {
            panic!("{id} and {other} wear the same {} and {}, both in {}", c.head, c.chest, c.main);
        }
    }
}

/// Seat 1, bare-handed and in the fighter's body, rises head first into
/// seat 0's still blade at `speed` cm a tick, with a shield or without.
/// Whether its head or neck was cut, and how many times a shield stopped
/// the blade.
fn rise_into_a_shield(shield: bool, speed: i32) -> (bool, usize) {
    let mut s = content::setup::versus(1, sim::balance::DEFAULT_TUNING);
    let mut b = s.bodies[0].clone();
    b.sword = None;
    b.shield = shield.then_some(sim::balance::SHIELD_SPEED);
    s.bodies.push(b);
    s.seats[1] = Some(Seat::at((s.bodies.len() - 1) as u8, Fx::int(200)));
    s.physics.ground = false;
    let mut w = World::new(s);
    let f1 = w.fighters[1].clone().unwrap();
    let head = (f1.base + w.setup.bodies[f1.body as usize].roles.head.unwrap() as u16) as usize;
    let k = w.swords_of(0)[0];
    let (butt, tip) = (w.particles[w.swords[k].butt as usize].p, w.particles[w.swords[k].tip as usize].p);
    let at = V2::lerp(butt, tip, Fx::ratio(2, 3));
    let shift = V2::new(at.x - w.particles[head].p.x, at.y - Fx::int(11 + 4) - w.particles[head].p.y);
    for p in w.particles.iter_mut() {
        if p.owner == Owner::Body(1) {
            p.p += shift;
            p.q = p.p - V2::new(Fx(0), Fx::int(speed));
        }
    }
    let (mut cut, mut stopped) = (false, 0);
    for _ in 0..20 {
        for p in w.particles.iter_mut() {
            if matches!(p.owner, Owner::Body(0) | Owner::Sword(0)) {
                p.q = p.p;
            }
        }
        w.step([Input::NONE, Input::NONE]);
        for e in &w.events {
            match *e {
                Event::Cut { seat: 1, part, .. } if part == HEAD || part == NECK => cut = true,
                Event::Shield { seat: 1, .. } => stopped += 1,
                _ => {}
            }
        }
    }
    (cut, stopped)
}

#[test]
fn a_shield_stops_a_fast_blade_and_lets_a_slow_one_through() {
    // Sam, 2026-10-08, after Dune: "if a blade moves too fast it gets like
    // locked in place". 14 cm a tick is over SHIELD_SPEED; 8 is between it
    // and MIN_CUT_SPEED.
    let (bare, _) = rise_into_a_shield(false, 14);
    assert!(bare, "the control failed: a fast blade did not cut a bare head");
    let (fast, stopped) = rise_into_a_shield(true, 14);
    assert!(stopped > 0 && !fast, "a shield let a fast blade through (stopped {stopped} times)");
    let (slow, _) = rise_into_a_shield(true, 8);
    assert!(slow, "a shield stopped a slow blade");
}

#[test]
fn a_beaten_opponents_outfit_can_be_worn_and_an_unbeaten_ones_cannot() {
    // Sam, 2026-10-08: "in arcade mode, you should obtain the outfit of
    // whoever you defeat ... set your player character to have that outfit".
    use content::save;
    let mut s = save::fresh();
    assert!(content::costumes::outfits(&s.road.best).is_empty(), "a fresh save has outfits");
    s.road.best.insert("herbalist".into(), content::road::Best::UNKNOWN);
    assert_eq!(content::costumes::outfits(&s.road.best), vec!["herbalist".to_string()]);
    // Worn, it survives the file; one not won does not.
    s.outfit = Some("herbalist".into());
    let back = save::decode(&save::encode(&s)).expect("the save reads back");
    assert_eq!(back.outfit.as_deref(), Some("herbalist"));
    s.outfit = Some("general".into());
    let back = save::decode(&save::encode(&s)).expect("the save reads back");
    assert_eq!(back.outfit, None, "an outfit not won was kept");
}
