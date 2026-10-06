//! The boomerang (Sam, 2026-10-06): "after you throw it it returns to you
//! after 2 seconds, but is only deadly until it hits the ground for the
//! first time, then resets when it returns to your hands ... it should be
//! deflectable by other weapons when its flying such that if it is
//! deflected, it can become deadly to the one who threw the weapon".

use content::road::{Best, Feats, Req};
use sim::balance::RETURN_TICKS;
use sim::fight::{Event, Phase};
use sim::{Input, World};
use std::collections::BTreeMap;

fn boomerang_against(other: &str) -> World {
    World::new(content::setup::versus_with(1, sim::balance::DEFAULT_TUNING, ["boomerang", other], "flat"))
}

/// Walk in for `walk` ticks, raise the arm for `wind`, and let go on the
/// next tick, arm still rising; then press nothing.
fn plan(t: u32, walk: u32, wind: u32) -> u16 {
    if t < walk {
        Input::STEP_RIGHT
    } else if t < walk + wind {
        Input::SHOULDER_UP
    } else if t == walk + wind {
        Input::SHOULDER_UP | Input::THROW
    } else {
        0
    }
}

#[test]
fn a_thrown_boomerang_is_back_in_the_hand_two_seconds_after_it_leaves_it() {
    // Thrown where it stands, far from the other fighter: nothing meets it.
    let mut w = boomerang_against("sword");
    for t in 0..=2u32 {
        w.step([Input(plan(t, 0, 2)), Input::NONE]);
    }
    let thrown = w.tick - 1;
    assert!(!w.held(0) && w.swords[0].flying, "the throw did not let go");
    assert_eq!(w.swords[0].back_at, Some(thrown + RETURN_TICKS));
    while w.tick <= thrown + RETURN_TICKS {
        assert!(!w.held(0), "the boomerang was back at tick {}, before its time", w.tick);
        w.step([Input::NONE, Input::NONE]);
    }
    let s = &w.swords[0];
    assert!(w.held(0) && !s.flying && !s.turned && s.back_at.is_none(), "the boomerang is not in the hand two seconds on: {s:?}");
}

#[test]
fn a_boomerang_that_has_touched_the_ground_cuts_nothing_on_its_way_home() {
    // The first test's throw from a few steps out goes over the other
    // fighter and lands beyond; it comes home past them.
    let mut w = boomerang_against("boomerang");
    let mut landed = None;
    for t in 0..240u32 {
        w.step([Input(plan(t, 50, 2)), Input::NONE]);
        if t > 52 && landed.is_none() && !w.swords[0].flying {
            landed = Some(t);
        }
        if landed.is_some() && w.swords[0].back_at.is_some() {
            for pi in 0..w.parts.len() {
                assert!(!w.may_cut(0, pi), "a downed boomerang may cut part {pi} at tick {t}");
            }
            assert!(!w.events.iter().any(|e| matches!(e, Event::Cut { by: 0, .. })), "a downed boomerang cut at tick {t}");
        }
    }
    let landed = landed.expect("the boomerang never came down");
    assert!(landed < 52 + RETURN_TICKS, "it came down at tick {landed}, after it was due back");
    assert!(w.held(0), "the boomerang did not come home after it landed");
}

#[test]
fn a_boomerang_another_blade_meets_in_flight_turns_and_cuts_its_own_thrower() {
    // Thrown from a few steps out at a fighter who lifts its sword into the
    // flight: the boomerang is turned, flies back, and the round ends with
    // the thrower cut by its own throw.
    let mut w = boomerang_against("sword");
    let mut turned = None;
    let mut end = None;
    for t in 0..300u32 {
        let lift = if (52..56).contains(&t) { Input::SHOULDER_UP } else { 0 };
        w.step([Input(plan(t, 40, 2)), Input(lift)]);
        if turned.is_none() && w.swords[0].turned {
            turned = Some(t);
        }
        if let Some(Event::RoundEnd { result }) = w.events.iter().find(|e| matches!(e, Event::RoundEnd { .. })) {
            end = Some(*result);
            break;
        }
    }
    assert!(turned.is_some(), "the lifted sword never turned the boomerang");
    let r = end.expect("the round never ended");
    assert_eq!((r.loser, r.by, r.thrown), (Some(0), 0, true), "the thrower was not cut by its own turned throw: {r:?}");
}

#[test]
fn both_boomerangs_thrown_and_down_the_round_is_not_drawn_because_each_comes_back() {
    // Both thrown and down, the round is not drawn: each is back in its
    // hand well before DISARMED_DRAW_TICKS.
    let mut w = boomerang_against("boomerang");
    w.step([Input(Input::THROW), Input(Input::THROW)]);
    for _ in 0..(2 * sim::balance::DISARMED_DRAW_TICKS) {
        w.step([Input::NONE, Input::NONE]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::RoundEnd { .. })), "a round with both boomerangs coming back ended at tick {}", w.tick);
    }
    assert!(matches!(w.phase, Phase::Fight));
}

#[test]
fn a_round_a_throw_ends_counts_toward_the_thrown_requirements() {
    // A boomerang thrown from a few steps out, past a fighter who turns its
    // elbow out for a moment, cuts its neck on the way home: the result
    // says the cut was thrown, and the feats carry it into a best result
    // that meets the thrown requirements. (A thrown sword's cut that ends
    // the round by ink is no thrown kill.)
    let mut w = boomerang_against("sword");
    let mut feats = Feats::default();
    let mut result = None;
    for t in 0..400u32 {
        let elbow = if (68..72).contains(&t) { Input::ELBOW_OUT } else { 0 };
        w.step([Input(plan(t, 50, 4)), Input(elbow)]);
        feats.observe(&w);
        if let Some(Event::RoundEnd { result: r }) = w.events.iter().find(|e| matches!(e, Event::RoundEnd { .. })) {
            result = Some(*r);
            break;
        }
    }
    let r = result.expect("the throw did not end the round");
    assert_eq!((r.loser, r.by, r.thrown), (Some(1), 0, true), "{r:?}");
    assert!(feats.thrown && feats.won == 1 && feats.won_thrown == 1, "{feats:?}");
    let mut best = BTreeMap::new();
    best.insert("scarecrow".to_string(), Best::won(0, w.tick, "sword").with_feats(feats));
    assert!(Req::Thrown("scarecrow".into()).met(&best) && Req::AllThrown("scarecrow".into()).met(&best));
    // One won round more, not by a throw: still a thrown win, no longer
    // every round thrown.
    let mut mixed = feats;
    mixed.won += 1;
    best.insert("scarecrow".to_string(), Best::won(0, w.tick, "sword").with_feats(mixed));
    assert!(Req::Thrown("scarecrow".into()).met(&best) && !Req::AllThrown("scarecrow".into()).met(&best));
}

#[test]
fn the_boomerang_is_smaller_than_the_short_sword_and_comes_back() {
    let w = |id: &str| content::weapons::weapon(id).unwrap();
    assert!(w("boomerang").length_pct < w("short_sword").length_pct);
    assert!(w("boomerang").returns && !w("short_sword").returns);
}

#[test]
fn each_row_below_the_first_has_one_boomerang_fight_a_thrown_win_in_the_row_above_opens() {
    let road = content::road::road();
    let level = |id: &str| road.iter().find(|s| s.id == id).unwrap().level();
    // Every row but the final fight's, which holds the local deity alone.
    let rows = road.iter().filter(|s| s.row.is_none()).map(|s| s.level()).max().unwrap();
    for row in 1..=rows {
        let here: Vec<_> = road.iter().filter(|s| s.level() == row && s.weapon.as_deref() == Some("boomerang")).collect();
        assert_eq!(here.len(), 1, "row {row} has {} boomerang fights", here.len());
        let thrown = here[0].requires.iter().find(|r| matches!(r, Req::Thrown(_) | Req::AllThrown(_)));
        let r = thrown.unwrap_or_else(|| panic!("the {} asks for no thrown win", here[0].id));
        assert_eq!(level(r.stop()), row - 1, "the {}'s thrown win is not in the row above", here[0].id);
    }
    let first = road.iter().find(|s| s.level() == 1 && s.weapon.as_deref() == Some("boomerang")).unwrap();
    assert_eq!(content::weapons::weapon("boomerang").unwrap().unlock, Some(Req::Beat(first.id.clone())));
}
