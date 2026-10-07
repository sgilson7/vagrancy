//! The chakram, the scythe and the twin blade (Sam, 2026-10-06: "3 new very
//! unique weapon types and shapes ... 1 of them being a boomerang type
//! weapon ... then make 1 enemy per layer per new weapon").

use content::road::{road, Req};
use content::weapons::weapon;
use sim::fight::Phase;
use sim::{Input, World};

const NEW: [&str; 3] = ["chakram", "scythe", "twin_blade"];

#[test]
fn each_new_weapon_has_one_fighter_in_each_row_and_is_won_from_the_first() {
    let road = road();
    let rows = road.iter().filter(|s| s.row.is_none()).map(|s| s.level()).max().unwrap();
    for w in NEW {
        let carriers: Vec<_> = road.iter().filter(|s| s.weapon.as_deref() == Some(w)).collect();
        for row in 0..=rows {
            assert_eq!(carriers.iter().filter(|s| s.level() == row).count(), 1, "the {w} has not one fighter in row {row}");
        }
        let first = carriers.iter().find(|s| s.level() == 0).unwrap();
        assert_eq!(weapon(w).unwrap().unlock, Some(Req::Beat(first.id.clone())), "the {w} is not won from its first fighter");
    }
    // The cursed blade is the last weapon on the list.
    assert_eq!(content::weapons::weapons().last().unwrap().id, "longsword");
}

/// The player's weapon in the yard, as the world holds it.
fn held(id: &str) -> (World, usize) {
    let w = World::new(content::setup::practice_with(1, sim::balance::DEFAULT_TUNING, id, false));
    let k = w.swords_of(0)[0];
    (w, k)
}

#[test]
fn the_scythes_pole_does_not_cut_and_the_twin_blade_has_a_blade_behind_the_hands() {
    // The scythe's line from butt to tip cuts only in its last part; the
    // hooked blade cuts along its length.
    let (w, k) = held("scythe");
    let edges = &w.swords[k].edges;
    let pole = edges.iter().find(|e| e.0 == w.swords[k].butt && e.1 == w.swords[k].tip).unwrap();
    assert!(pole.2 > sim::fx::Fx::ratio(8, 10), "the scythe's pole cuts from {:?} of its length", pole.2);
    assert!(edges.iter().any(|e| e.0 != w.swords[k].butt && e.2 == sim::fx::Fx(0)), "the scythe has no blade");
    // The twin blade's hands are well along it from the butt: the second
    // blade is behind them.
    let (w, k) = held("twin_blade");
    let hand_at = w.cons.iter().find_map(|c| match c.con {
        sim::world::Con::Pin { b, at, .. } if b == w.swords[k].butt => Some(at),
        _ => None,
    });
    let at = hand_at.expect("the twin blade is held");
    assert!(at > sim::fx::Fx::ratio(3, 10), "the twin blade's hands sit at {at:?} of it, with no blade behind them");
    assert_eq!(w.swords[k].edges.len(), 2);
    // The chakram comes back to the hand, as the boomerang does.
    assert!(weapon("chakram").unwrap().returns);
}

/// Seat 1's keys over a match against the yardstick.
fn keys(id: &str) -> Vec<Input> {
    let mut out = Vec::new();
    let mut w = World::new(content::setup::road(2, sim::balance::DEFAULT_TUNING, id));
    let mut ps = content::road::lineup(&content::road::pilot("yardstick"), id);
    let mut last = [Input::NONE; sim::body::SEATS];
    while w.tick < 2400 && !matches!(w.phase, Phase::MatchOver { .. }) {
        let mut i = [Input::NONE; sim::body::SEATS];
        for (k, p) in ps.iter_mut().enumerate() {
            p.observe(last);
            i[k] = p.input(&w, k);
        }
        if matches!(w.phase, Phase::Fight) {
            out.push(i[1]);
        }
        w.step_all(i);
        last = i;
    }
    out
}

#[test]
fn the_new_fighters_use_the_moves_their_introductions_name() {
    // "sometimes throws it": a chakram fighter, row 1 and row 5.
    for id in ["coin_minter", "quoit_thrower"] {
        assert!(keys(id).iter().any(|i| i.has(Input::THROW)), "the {id} never threw");
    }
    // "swings the blade over or low": both shoulder keys.
    for id in ["gleaner", "sheaf_binder"] {
        let k = keys(id);
        assert!(k.iter().any(|i| i.has(Input::SHOULDER_UP)) && k.iter().any(|i| i.has(Input::SHOULDER_DOWN)), "the {id} did not swing over and low");
    }
    // "spins both blades round": the shoulder held up longer than an
    // overhead swing holds it (16 ticks, pilot::Tree::play), the one other
    // move that raises it. A dodge can cut a 24-tick spin short.
    for id in ["acrobat", "fan_dancer"] {
        let k = keys(id);
        let longest = k.split(|i| !i.has(Input::SHOULDER_UP)).map(|r| r.len()).max().unwrap_or(0);
        assert!(longest > 16, "the {id}'s longest spin was {longest} ticks");
    }
}
