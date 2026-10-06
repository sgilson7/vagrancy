//! Flanked stops (Sam, 2026-10-05: "all of these fights to involve an enemy
//! on each side of you"): seat 0 against seats 1 and 2.

use content::road::{lineup, pilot, road, Best, Req};
use sim::body::side;
use sim::fight::{Event, Phase};
use sim::{Input, World};
use std::collections::BTreeMap;

/// The flanked stops.
fn flanked() -> Vec<String> {
    road().into_iter().filter(|s| s.companion.is_some()).map(|s| s.id).collect()
}

/// Play the yardstick against a flanked stop and hand every tick's world to
/// `see`, with every seat's input that tick.
fn play(id: &str, seed: u64, ticks: u32, mut see: impl FnMut(&World, &World)) {
    let mut w = World::new(content::setup::road(seed, sim::balance::DEFAULT_TUNING, id));
    let mut ps = lineup(&pilot("yardstick"), id);
    let mut last = [Input::NONE; sim::body::SEATS];
    while w.tick < ticks && !matches!(w.phase, Phase::MatchOver { .. }) {
        let mut i = [Input::NONE; sim::body::SEATS];
        for (k, p) in ps.iter_mut().enumerate() {
            p.observe(last);
            i[k] = p.input(&w, k);
        }
        let before = w.clone();
        w.step_all(i);
        last = i;
        see(&before, &w);
    }
}

#[test]
fn ten_fights_put_an_opponent_on_each_side_of_the_player() {
    let ids = flanked();
    assert_eq!(ids.len(), 10, "the flanked stops are {ids:?}");
    for id in &ids {
        let w = World::new(content::setup::road(0, sim::balance::DEFAULT_TUNING, id));
        let x = |s: usize| pilot::pelvis(&w, s).unwrap().x.trunc();
        assert!(x(2) < x(0) && x(0) < x(1), "{id}: the fighters stand at {}, {} and {}, not one on each side of the player", x(2), x(0), x(1));
        assert_eq!((side(1), side(2)), (1, 1));
    }
    // "a couple of fights that have platforms".
    let on_ledges = road().into_iter().filter(|s| s.companion.is_some() && s.map.as_deref().is_some_and(|m| m != content::maps::FLAT)).count();
    assert!(on_ledges >= 2, "{on_ledges} flanked fights have ledges");
}

#[test]
fn a_round_goes_on_until_both_opponents_are_out_and_nobody_cuts_their_own_side() {
    // Over one match at each flanked stop: a round the player wins ends only once
    // both opponents are out, at least one round goes on with one of them
    // already out, a fighter who is out neither cuts nor is cut, and no
    // blade cuts its own side.
    let mut went_on = false;
    let mut wins = 0;
    for id in flanked() {
        for seed in 0..1 {
            play(&id, seed, 3000, |before, w| {
                let out = |s: usize| w.fighters[s].as_ref().is_some_and(|f| f.out_at.is_some());
                if matches!(w.phase, Phase::Fight) && (out(1) != out(2)) {
                    went_on = true;
                }
                for e in &w.events {
                    match *e {
                        Event::Cut { seat, by, .. } => {
                            assert_ne!(side(seat as usize), side(by as usize), "{id}: seat {by} cut seat {seat}, on its own side");
                            assert!(!before.out(seat as usize), "{id}: seat {seat} was cut at tick {} after it was out", w.tick);
                            assert!(!before.out(by as usize), "{id}: seat {by} cut at tick {} after it was out", w.tick);
                        }
                        Event::RoundEnd { result } if result.loser == Some(1) => {
                            wins += 1;
                            assert!(out(1) && out(2), "{id}: the player won a round at tick {} with an opponent still in it", w.tick);
                        }
                        _ => {}
                    }
                }
            });
        }
    }
    assert!(went_on, "no round went on with one opponent out");
    assert!(wins > 0, "the yardstick won no round, so the rule for winning one was never checked");
}

#[test]
fn the_flanked_fights_open_on_a_headshot_or_an_untouched_win() {
    // Sam: "the new enemies should be locked behind new conditions on the
    // current tree".
    for s in road().into_iter().filter(|s| s.companion.is_some()) {
        assert!(
            s.requires.iter().any(|r| matches!(r, Req::Headshot(_) | Req::Untouched(_))),
            "{} opens without either new condition",
            s.id
        );
    }
    // At the roofer: every other requirement met, then the headshot the
    // roofer asks for, read from a won match's feats.
    let roofer = content::road::stop("roofer").unwrap();
    let Some(Req::Headshot(at)) = roofer.requires.iter().find(|r| matches!(r, Req::Headshot(_))).cloned() else {
        panic!("the roofer asks for no headshot")
    };
    let mut best = BTreeMap::new();
    let mut untouched = content::road::Feats::default();
    untouched.untouched = true;
    for r in roofer.requires.iter().filter(|r| r.stop() != at) {
        content::road::record(&mut best, r.stop(), Best::won(0, 1, "sword").with_feats(untouched));
    }
    content::road::record(&mut best, &at, Best::won(0, 900, "sword"));
    assert!(!content::road::open(&roofer, &best), "the roofer opened without a headshot at the {at}");
    let mut feats = content::road::Feats::default();
    feats.headshot = true;
    let opened = content::road::record(&mut best, &at, Best::won(2, 4000, "sword").with_feats(feats));
    assert!(opened.contains(&"roofer".to_string()), "a headshot at the {at} did not open the roofer: {opened:?}");
    assert!(best[&at].headshot && !best[&at].untouched && best[&at].losses == 0, "the best at the {at} is {:?}", best[&at]);
}

#[test]
fn feats_are_read_from_the_rounds_a_match_had() {
    // The yardstick against the scarecrow, which does not fight back: every
    // round the yardstick wins, it wins untouched, and the feats say whether
    // one of those rounds ended on a cut across the scarecrow's head, as the
    // round's own result does.
    let mut heads = 0;
    for seed in 0..3 {
        let mut w = World::new(content::setup::road(seed, sim::balance::DEFAULT_TUNING, "scarecrow"));
        let mut ps = lineup(&pilot("yardstick"), "scarecrow");
        let mut feats = content::road::Feats::default();
        let (mut won, mut head) = (false, false);
        while w.tick < 4000 && !matches!(w.phase, Phase::MatchOver { .. }) {
            let i = [ps[0].input(&w, 0), ps[1].input(&w, 1)];
            w.step(i);
            feats.observe(&w);
            for e in &w.events {
                if let Event::RoundEnd { result } = e {
                    if result.loser == Some(1) {
                        won = true;
                        head |= result.by == 0 && content::messages::headshot(&w, result);
                    }
                }
            }
        }
        assert_eq!(feats.untouched, won, "seed {seed}: the feats say untouched {}, and a round was won {won}", feats.untouched);
        assert_eq!(feats.headshot, head, "seed {seed}: the feats say headshot {}, and the rounds {head}", feats.headshot);
        heads += head as u32;
    }
    assert!(heads > 0, "no match had a headshot, so the feat was never read");
}

#[test]
fn every_fighter_on_a_flanked_stop_stands_on_its_own() {
    // Sam: "the lhs enemy always falls over ... and goes limp". With nobody
    // pressing anything, all three fighters are on their feet five seconds
    // in, at every flanked stop.
    for id in flanked() {
        let mut w = World::new(content::setup::road(1, sim::balance::DEFAULT_TUNING, &id));
        for _ in 0..300 {
            w.step_all([Input::NONE; sim::body::SEATS]);
        }
        for seat in 0..sim::body::SEATS {
            assert!(!w.knocked_down(seat), "{id}: seat {seat} fell over with nobody touching it");
        }
    }
}
