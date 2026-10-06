//! The throw (Sam's friend, 2026-10-05: swords "throwable ... with a button
//! to release the sword; once its hit the ground, then its no longer
//! dangerous").

use sim::fight::{Event, Phase};
use sim::{Input, World};

fn versus() -> World {
    World::new(content::setup::versus(1, sim::balance::DEFAULT_TUNING))
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
fn a_thrown_sword_flies_on_and_cuts_until_it_first_touches_the_ground() {
    // From well out of reach (50 ticks of walking leaves the fighters more
    // than a blade apart), a throw on the upswing crosses the gap and cuts.
    let mut w = versus();
    let (mut thrown_at, mut flying_cuts, mut landed) = (None, 0, None);
    for t in 0..400u32 {
        let was = w.swords[0].flying;
        w.step([Input(plan(t, 50, 2)), Input::NONE]);
        if t == 52 {
            assert!(!w.held(0) && w.swords[0].flying, "the throw did not let go of the sword");
            thrown_at = Some(t);
        }
        if was || w.swords[0].flying {
            flying_cuts += w.events.iter().filter(|e| matches!(e, Event::Cut { by: 0, seat: 1, .. })).count();
        }
        if thrown_at.is_some() && landed.is_none() && !w.swords[0].flying {
            landed = Some(t);
        }
    }
    assert!(flying_cuts > 0, "the thrown sword never cut the other fighter");
    assert!(landed.is_some(), "the thrown sword never came down");
}

#[test]
fn a_thrown_sword_that_has_touched_the_ground_cuts_nothing() {
    // A throw that comes down at once, near the thrower; then the other
    // fighter walks over it and on, and nothing of theirs is cut.
    let mut w = versus();
    for t in 0..80u32 {
        w.step([Input(plan(t, 40, 30)), Input::NONE]);
    }
    assert!(!w.held(0) && !w.swords[0].flying, "the sword is not down after the throw");
    for pi in 0..w.parts.len() {
        assert!(!w.may_cut(0, pi), "a sword on the ground may still cut part {pi}");
    }
    for _ in 0..360 {
        w.step([Input::NONE, Input(Input::STEP_LEFT)]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::Cut { by: 0, .. })), "a sword on the ground cut at tick {}", w.tick);
    }
}

#[test]
fn the_throw_key_without_a_sword_does_nothing_and_the_next_round_brings_the_sword_back() {
    // The throw that cuts in the first test ends the round; at the next,
    // every fighter holds its sword again.
    let mut w = versus();
    for t in 0..400u32 {
        w.step([Input(plan(t, 50, 2)), Input::NONE]);
        if !matches!(w.phase, Phase::Fight) {
            break;
        }
    }
    assert!(matches!(w.phase, Phase::RoundOver { .. }), "the thrown sword did not end the round");
    while !matches!(w.phase, Phase::Fight) {
        w.step([Input(Input::READY), Input(Input::READY)]);
    }
    assert!(w.held(0) && !w.swords[0].flying, "the next round did not give the thrower its sword back");
    // And pressing throw with the sword gone changes nothing more.
    w.step([Input(Input::THROW), Input::NONE]);
    w.step([Input::NONE, Input::NONE]);
    w.step([Input(Input::THROW), Input::NONE]);
    assert!(!w.held(0), "the throw did not let go");
    let before = w.cons.len();
    w.step([Input::NONE, Input::NONE]);
    w.step([Input(Input::THROW), Input::NONE]);
    assert_eq!(w.cons.len(), before, "a throw with no sword in hand changed the world's constraints");
}

#[test]
fn with_both_swords_thrown_and_down_the_round_is_a_draw_four_seconds_later() {
    // Sam: "if both people throw the weapon, then there is no way to
    // conclude the fight, so it should end in a draw if no one dies after 4
    // seconds of throwing their weapons". Both let go at once, far apart,
    // so neither sword reaches anybody.
    let mut w = versus();
    w.step([Input(Input::THROW), Input(Input::THROW)]);
    let mut bladeless = None;
    let mut ended = None;
    for _ in 0..1200 {
        w.step([Input::NONE, Input::NONE]);
        if bladeless.is_none() && !w.swords.iter().any(|s| s.flying) {
            bladeless = Some(w.tick - 1);
        }
        if let Some(Event::RoundEnd { result }) = w.events.iter().find(|e| matches!(e, Event::RoundEnd { .. })) {
            ended = Some((w.tick - 1, *result));
            break;
        }
    }
    let (at, result) = ended.expect("the round never ended with both swords down");
    let since = bladeless.unwrap();
    assert_eq!(result.loser, None, "a disarmed round was won: {result:?}");
    assert_eq!(result.cause, sim::body::Cause::Disarmed);
    assert_eq!(at - since, sim::balance::DISARMED_DRAW_TICKS, "the draw came {} ticks after the last blade was down", at - since);
    assert_eq!(w.wins, [0, 0]);
}

#[test]
fn while_one_fighter_still_holds_a_sword_the_round_goes_on() {
    let mut w = versus();
    w.step([Input(Input::THROW), Input::NONE]);
    for _ in 0..(3 * sim::balance::DISARMED_DRAW_TICKS) {
        w.step([Input::NONE, Input::NONE]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::RoundEnd { result } if result.cause == sim::body::Cause::Disarmed)), "a round with a sword in hand was drawn");
    }
    assert!(matches!(w.phase, Phase::Fight));
}
