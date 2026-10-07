//! Sam, 2026-10-07: "the game vagrancy should not tell you where you get the
//! cursed blade anywhere until you find it ... that is the point of your
//! rampage to some extent".

use content::road::road;
use content::save::finished;
use content::weapons::{carried_name_key, prize_found, unlock_shown, weapons};
use std::collections::BTreeMap;

fn copy() -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/copy.en.json")).unwrap()).unwrap()
}

#[test]
fn no_card_says_where_the_cursed_blade_is_until_it_is_found() {
    let prize = weapons().into_iter().find(|w| w.prize).expect("a weapon is the prize");
    let cursed = format!("weapons.{}.name", prize.id);
    // From the start, and with each fight won but the one that gives it.
    for best in [BTreeMap::new(), finished(&["local_deity", "guardian_deity"]).road.best] {
        assert!(!prize_found(&best));
        assert!(!unlock_shown(&prize, &best), "the cursed blade's card says how it is won before it is found");
        for st in road().iter().filter(|s| content::road::visible(s, &best)) {
            if let Some(w) = &st.weapon {
                assert_ne!(carried_name_key(w, st.eight_arms, &best), cursed, "the {} card names the cursed blade before it is found", st.id);
            }
        }
    }
    // Found: its card may say so, and the guardian deity's blades are cursed.
    let best = finished(&[]).road.best;
    assert!(prize_found(&best) && unlock_shown(&prize, &best));
    let g = road().into_iter().find(|s| s.eight_arms).expect("the guardian deity");
    assert_eq!(carried_name_key(g.weapon.as_deref().unwrap(), true, &best), cursed);
}

#[test]
fn each_name_a_card_can_give_a_carried_weapon_is_in_the_copy_file() {
    let c = copy();
    for st in road() {
        if let Some(w) = &st.weapon {
            for best in [BTreeMap::new(), finished(&[]).road.best] {
                let key = carried_name_key(w, st.eight_arms, &best);
                let found = key.split('.').try_fold(&c, |v, k| v.get(k)).and_then(|v| v.as_str());
                assert!(found.is_some(), "no copy string at {key}, for the {} card", st.id);
            }
        }
    }
}
