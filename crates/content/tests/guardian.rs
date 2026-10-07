//! The guardian deity (Sam, 2026-10-06): "a secret final boss that only
//! appears on the chart / visualizations after you've beaten every single
//! fight without getting hit once ... 8 arms and each pair is holding a
//! cursed blade ... the bubble only appears on the chart after you've
//! defeated the local deity".

use content::road::{road, stop, visible, Best, Req};
use content::save::{decode, encode, finished};
use std::collections::BTreeMap;
use sim::World;

fn guardian() -> content::road::Stop {
    stop("guardian_deity").expect("the road has the guardian deity")
}

#[test]
fn the_guardian_deity_is_hidden_until_the_village_deity_is_beaten() {
    let g = guardian();
    assert!(g.secret);
    let mut best = finished(&["local_deity", "guardian_deity"]).road.best;
    assert!(!visible(&g, &best), "the guardian deity shows before the village deity is beaten");
    assert!(content::road::next_goal(&best).is_none_or(|(id, _)| id != g.id), "a hidden fight was named as the next goal");
    // Beaten with a round lost: the guardian shows, and stays locked.
    best.insert("local_deity".into(), Best::won(1, 3600, "sword"));
    assert!(visible(&g, &best));
    assert!(!content::road::open(&g, &best), "the guardian deity opened after a round was lost");
    // Every other fight shows from the start or not at all, as before.
    assert!(road().iter().filter(|s| !s.secret).all(|s| visible(s, &BTreeMap::new())));
}

#[test]
fn the_guardian_deity_opens_when_each_other_fight_is_won_without_losing_a_round() {
    let g = guardian();
    assert_eq!(g.requires, vec![Req::AllFlawless("local_deity".into())], "the chart draws one line into it, from the village deity");
    let mut best = finished(&["guardian_deity"]).road.best;
    assert!(content::road::open(&g, &best));
    for id in ["scarecrow", "local_deity", "wind_reader"] {
        let mut lost = best.clone();
        lost.insert(id.into(), Best::won(1, 3600, "sword"));
        assert!(!content::road::open(&g, &lost), "the guardian deity opened with a round lost at the {id}");
    }
    best.remove("thresher");
    assert!(!content::road::open(&g, &best), "the guardian deity opened with the thresher unbeaten");
}

#[test]
fn the_guardian_deity_has_eight_arms_and_a_cursed_blade_in_each_pair_of_hands() {
    let w = World::new(content::setup::road(1, sim::balance::DEFAULT_TUNING, "guardian_deity"));
    let mine = w.swords_of(1);
    assert_eq!(mine.len(), 4);
    let body = &w.setup.bodies[w.fighters[1].as_ref().unwrap().body as usize];
    assert_eq!(body.parts.iter().filter(|p| p.hand).count(), 8, "the guardian deity does not have eight hands");
    let player = w.swords_of(0)[0];
    let reach = |k: usize| w.swords[k].len.trunc();
    for k in mine {
        assert!(w.held(k));
        // The cursed blade is a sword in reach (data/weapons.json, Sam
        // 2026-10-07); the waist and neck pairs' rest blades are placed by
        // hand through both hands, so within a few cm. Each is cursed.
        let want = reach(player) * content::weapons::weapon("cursed_blade").unwrap().length_pct as i32 / 100;
        assert!((reach(k) - want).abs() <= 4, "blade {k} is {} cm, not a cursed blade's {want}", reach(k));
        assert!(w.swords[k].cursed, "blade {k} is not cursed");
    }
    // It stands: after five seconds with no keys its head is still high.
    let mut w = w;
    for _ in 0..300 {
        w.step_all([sim::Input::NONE; sim::body::SEATS]);
    }
    let head = pilot::head(&w, 1).expect("it has a head").y.trunc();
    assert!(head > 150, "the guardian deity fell: its head is at {head} cm");
}

#[test]
fn the_test_saves_load_and_hold_what_their_names_say() {
    // testing/saves/, written by `lab test-saves` from `content::save::finished`.
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testing/saves");
    for (name, unwon) in [("all-but-the-village-deity.save.json", vec!["local_deity", "guardian_deity"]), ("all-but-the-guardian-deity.save.json", vec!["guardian_deity"])] {
        let text = std::fs::read_to_string(format!("{dir}/{name}")).unwrap_or_else(|_| panic!("{name}: run `cargo run -p lab -- test-saves`"));
        let s = decode(&text).unwrap_or_else(|e| panic!("{name} does not load: {e:?}"));
        assert_eq!(text, encode(&finished(&unwon)), "{name} is stale: run `cargo run -p lab -- test-saves`");
        let g = guardian();
        let village = stop("local_deity").unwrap();
        for st in road() {
            assert_eq!(s.road.best.get(&st.id).map(|b| b.losses), (!unwon.contains(&st.id.as_str())).then_some(0), "{name}: the {}", st.id);
            if !unwon.contains(&st.id.as_str()) || st.id == village.id {
                assert!(content::road::open(&st, &s.road.best), "{name}: the {} is not open", st.id);
            }
        }
        let shown = visible(&g, &s.road.best);
        assert_eq!(shown, !unwon.contains(&"local_deity"), "{name}: the guardian deity shown is {shown}");
        assert_eq!(content::road::open(&g, &s.road.best), shown, "{name}");
    }
}
