//! The chakram, the scythe and the twin blade (Sam, 2026-10-06: "3 new very
//! unique weapon types and shapes ... 1 of them being a boomerang type
//! weapon ... then make 1 enemy per layer per new weapon").

use content::road::{road, Req};
use std::collections::BTreeMap;
use content::weapons::weapon;
use sim::fight::Phase;
use sim::{Input, World};

const NEW: [&str; 3] = ["chakram", "scythe", "twin_blade"];

#[test]
fn each_new_weapon_has_one_fighter_in_each_row() {
    let road = road();
    let rows = road.iter().filter(|s| s.row.is_none()).map(|s| s.level()).max().unwrap();
    for w in NEW {
        let carriers: Vec<_> = road.iter().filter(|s| s.weapon.as_deref() == Some(w)).collect();
        for row in 0..=rows {
            assert_eq!(carriers.iter().filter(|s| s.level() == row).count(), 1, "the {w} has not one fighter in row {row}");
        }
    }
    // The cursed blade is the last weapon on the list.
    let last = content::weapons::weapons().last().unwrap().clone();
    assert!(last.prize && last.id == "cursed_blade", "the {} is last on the list, not the cursed blade", last.id);
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

#[test]
fn each_row_awards_a_weapon_from_a_villager_who_carries_it_the_first_row_two() {
    // Sam (2026-10-06): "each layer a new weapon should be awarded from a
    // fight, with more weapons stacked early rather than later ... you
    // should find blades from random villagers that arent the cursed blade".
    let road = road();
    let level = |id: &str| road.iter().find(|s| s.id == id).unwrap().level();
    let mut per_row: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for w in content::weapons::weapons().into_iter().filter(|w| !w.prize) {
        let Some(Req::Beat(at)) = &w.unlock else { continue };
        let st = road.iter().find(|s| &s.id == at).unwrap();
        assert_eq!(st.weapon.as_deref(), Some(w.id.as_str()), "the {} is won from the {}, who does not carry it", w.id, at);
        per_row.entry(level(at)).or_default().push(w.id.clone());
    }
    let rows = road.iter().filter(|s| s.row.is_none()).map(|s| s.level()).max().unwrap();
    let counts: Vec<usize> = (0..=rows).map(|r| per_row.get(&r).map_or(0, |v| v.len())).collect();
    assert!(counts.iter().all(|&n| n >= 1), "a row awards no weapon: {per_row:?}");
    assert!(counts.windows(2).all(|p| p[0] >= p[1]), "a later row awards more weapons than an earlier one: {counts:?}");
    // The list is in the order they are won, the sword first and the
    // cursed blade last.
    // The villagers' long blade is not won, so it is not in that order.
    let ids: Vec<String> = content::weapons::weapons().into_iter().filter(|w| !w.enemy_only).map(|w| w.id).collect();
    let won: Vec<String> = per_row.values().flatten().cloned().collect();
    assert_eq!(ids[1..ids.len() - 1], won[..], "the weapon list is not in the order the weapons are won");
}
