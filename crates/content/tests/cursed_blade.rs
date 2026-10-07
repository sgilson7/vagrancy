//! Sam, 2026-10-07: "the game vagrancy should not tell you where you get the
//! cursed blade anywhere until you find it", and then: "you should also not
//! be told how to find any of the weapons, and they should not even appear
//! as options in the bar besides the sword and the cursed blade at first
//! ... maybe there are rumors you acquire from defeating enemies that can
//! make the weapon boxes appear and tell you who to defeat to find them,
//! and you start with 1 rumor for the short sword".

use content::road::{road, Best};
use content::save::finished;
use content::weapons::{carried_shown, prize_found, prize_progress, rumored, shown, unlock_shown, weapons, Rumor, DEFAULT};
use std::collections::BTreeMap;

fn won(ids: &[&str]) -> BTreeMap<String, Best> {
    ids.iter().map(|id| (id.to_string(), Best::won(0, 3600, "sword"))).collect()
}

#[test]
fn at_first_the_weapon_list_shows_the_sword_the_cursed_blade_and_one_rumor() {
    let best = BTreeMap::new();
    let mut seen: Vec<String> = weapons().into_iter().filter(|w| shown(w, &best)).map(|w| w.id).collect();
    seen.sort();
    assert_eq!(seen, vec!["cursed_blade", "short_sword", DEFAULT], "a fresh player sees other weapons than these");
    let short = weapons().into_iter().find(|w| w.id == "short_sword").unwrap();
    assert!(rumored(&short, &best) && unlock_shown(&short, &best), "the short sword's rumor does not say who carries it");
}

#[test]
fn a_rumor_heard_from_a_beaten_villager_shows_its_weapon_and_who_carries_it() {
    for w in weapons().into_iter().filter(|w| matches!(w.rumor, Some(Rumor::Beat(_)))) {
        let Some(Rumor::Beat(from)) = &w.rumor else { unreachable!() };
        assert!(road().iter().any(|s| &s.id == from), "{}'s rumor comes from {from}, who is not on the road", w.id);
        assert!(!shown(&w, &BTreeMap::new()) && !unlock_shown(&w, &BTreeMap::new()), "{} shows before its rumor", w.id);
        let best = won(&[from]);
        assert!(shown(&w, &best) && unlock_shown(&w, &best), "beating {from} did not tell where the {} is", w.id);
    }
    // Each weapon a player can win has a way to be heard of, but the prize.
    for w in weapons().into_iter().filter(|w| !w.enemy_only && w.id != DEFAULT && !w.prize) {
        assert!(w.rumor.is_some(), "the {} has no rumor, so it would be shown only once won", w.id);
    }
}

#[test]
fn no_card_says_where_the_cursed_blade_is_until_it_is_found() {
    let prize = weapons().into_iter().find(|w| w.prize).expect("a weapon is the prize");
    for best in [BTreeMap::new(), finished(&["local_deity", "guardian_deity"]).road.best] {
        assert!(!prize_found(&best));
        assert!(shown(&prize, &best), "the cursed blade is not there to taunt the player");
        assert!(!unlock_shown(&prize, &best), "the cursed blade's card says how it is won before it is found");
        assert!(!carried_shown(&prize.id, &best), "a card says who carries the cursed blade before it is found");
    }
    let best = finished(&[]).road.best;
    assert!(prize_found(&best) && unlock_shown(&prize, &best) && carried_shown(&prize.id, &best));
}

#[test]
fn the_cursed_blade_gathers_with_each_fight_won_and_is_whole_when_each_is() {
    let (n0, total) = prize_progress(&BTreeMap::new());
    assert_eq!(n0, 0);
    let (n1, _) = prize_progress(&won(&["scarecrow"]));
    assert_eq!(n1, 1);
    let (all, t2) = prize_progress(&finished(&["guardian_deity"]).road.best);
    assert_eq!((all, t2), (total, total), "winning each fight but the secret one did not fill the blade");
}
