//! Which copy string says what, for outcomes core reports.
//!
//! Choosing the sentence is a rule, so it lives here where a test reaches
//! it, not in the page. The page only fills the placeholders it is given.

use serde_json::{json, Value};
use sim::body::Cause;
use sim::fight::{Phase, RoundResult};
use sim::replay::ReplayError;
use sim::World;

/// Whose eyes a result is written for.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Audience<'a> {
    /// Two people at one keyboard or online: the fighters by name, no "you".
    Versus,
    /// The road: seat 0 is "you", seat 1 is the opponent with this id.
    Road { opponent: &'a str },
}

fn copy_at<'a>(copy: &'a Value, key: &str) -> &'a str {
    key.split('.').fold(copy, |o, k| &o[k]).as_str().unwrap_or_else(|| panic!("no copy string at {key}"))
}

fn fighter_name(copy: &Value, seat: u8) -> String {
    copy_at(copy, if seat == 0 { "fighters.left.name" } else { "fighters.right.name" }).to_string()
}

/// The `parts.<id>` word for a body part, by the `copy` its definition names.
fn part_word(copy: &Value, w: &World, seat: u8, part: u8) -> String {
    let body = w.setup.seats[seat as usize].map(|s| s.body as usize).unwrap_or(0);
    let id = &w.setup.bodies[body].parts[part as usize].copy;
    copy_at(copy, &format!("parts.{id}")).to_string()
}

/// The sentence for how a round ended (`results.*`), chosen here, where a
/// test reaches it, and not in the page.
pub fn round_result(w: &World, r: &RoundResult, who: Audience) -> Value {
    let copy = crate::copy::copy();
    let Some(loser) = r.loser else { return json!({ "key": "results.draw", "vars": {} }) };
    let winner = 1 - loser;
    let own = r.by == loser && r.cause != Cause::Ink;
    let part = part_word(&copy, w, loser, r.part);
    match who {
        Audience::Versus => {
            let names = json!({ "winner": fighter_name(&copy, winner), "loser": fighter_name(&copy, loser), "part": part });
            let key = match (own, r.cause) {
                (true, _) => "results.versus.self",
                (false, Cause::Neck) => "results.versus.neck",
                (false, Cause::Heart) => "results.versus.heart",
                (false, Cause::Ink) => "results.versus.ink",
            };
            json!({ "key": key, "vars": names })
        }
        Audience::Road { opponent } => {
            let name = copy_at(&copy, &format!("opponents.{opponent}.name")).to_string();
            let mid = copy_at(&copy, &format!("opponents.{opponent}.name_mid")).to_string();
            let won = loser == 1;
            let key = match (won, own, r.cause) {
                (true, true, _) => "results.road.win_self",
                (true, false, Cause::Neck) => "results.road.win_neck",
                (true, false, Cause::Heart) => "results.road.win_heart",
                (true, false, Cause::Ink) => "results.road.win_ink",
                (false, true, _) => "results.road.lose_self",
                (false, false, Cause::Neck) => "results.road.lose_neck",
                (false, false, Cause::Heart) => "results.road.lose_heart",
                (false, false, Cause::Ink) => "results.road.lose_ink",
            };
            json!({ "key": key, "vars": { "opponent": name, "opponent_mid": mid, "part": part } })
        }
    }
}

/// The sentence for the end of a match.
pub fn match_result(w: &World, who: Audience) -> Value {
    let copy = crate::copy::copy();
    let winner = if w.wins[0] >= w.wins[1] { 0u8 } else { 1u8 };
    let (wins, losses) = (w.wins[winner as usize], w.wins[1 - winner as usize]);
    match who {
        Audience::Versus => json!({ "key": "results.versus.match", "vars": { "winner": fighter_name(&copy, winner), "wins": wins, "losses": losses } }),
        Audience::Road { opponent } => {
            let name = copy_at(&copy, &format!("opponents.{opponent}.name")).to_string();
            if winner == 0 {
                json!({ "key": "results.road.match_win", "vars": { "wins": wins, "losses": losses } })
            } else {
                json!({ "key": "results.road.match_lose", "vars": { "wins": w.wins[0], "losses": w.wins[1], "opponent": name } })
            }
        }
    }
}

/// What the page should say about the world's phase right now: nothing in a
/// fight, the round's sentence between rounds, the match's at the end.
pub fn phase_text(w: &World, who: Audience) -> Value {
    match w.phase {
        Phase::Fight => Value::Null,
        Phase::RoundOver { result, ready } => {
            json!({ "round": round_result(w, &result, who), "ready": ready, "popup": popup(w, &result), "focus": focus(w, &result) })
        }
        Phase::MatchOver { result } => {
            json!({ "round": round_result(w, &result, who), "match": match_result(w, who), "popup": popup(w, &result), "focus": focus(w, &result) })
        }
    }
}

/// Whether the round's deciding cut crossed the head (Sam: "if someone gets
/// headshot"), as against the neck, which the same cause covers.
fn headshot(w: &World, r: &RoundResult) -> bool {
    let Some(loser) = r.loser else { return false };
    let body = w.setup.seats[loser as usize].map(|s| s.body as usize).unwrap_or(0);
    r.cause == Cause::Neck && w.setup.bodies[body].parts.get(r.part as usize).is_some_and(|p| p.copy == "head")
}

/// The heading of the card that comes up when a round ends: how the loser
/// went down (Sam: "pop a popup on the screen about how someone died").
pub fn popup(w: &World, r: &RoundResult) -> Value {
    let key = match (r.loser, r.cause) {
        (None, _) => "results.popup.draw",
        (Some(_), Cause::Ink) => "results.popup.ink",
        (Some(_), Cause::Heart) => "results.popup.heart",
        (Some(_), Cause::Neck) if headshot(w, r) => "results.popup.head",
        (Some(_), Cause::Neck) => "results.popup.neck",
    };
    json!({ "key": key, "vars": {} })
}

/// For a cut across the head, where the blade landed, in the world's raw
/// fixed-point units like the frame's points: the page stops the clock and
/// looks there (Sam: "stop the simulation to focus on it"). Read on the tick
/// the round ended, which is the tick of the cut.
pub fn focus(w: &World, r: &RoundResult) -> Value {
    if !headshot(w, r) {
        return Value::Null;
    }
    let loser = r.loser.unwrap();
    let at = w
        .events
        .iter()
        .find_map(|e| match e {
            sim::fight::Event::Cut { seat, part, at, .. } if *seat == loser && *part == r.part => Some(*at),
            _ => None,
        })
        .or_else(|| pilot::head(w, loser as usize));
    match at {
        Some(p) => json!([p.x.0, p.y.0]),
        None => Value::Null,
    }
}

/// `{ "key": "...", "vars": { ... } }`, for the page to look up and fill.
pub fn replay_error(e: ReplayError) -> Value {
    match e {
        ReplayError::Format => json!({ "key": "replay.error.format", "vars": {} }),
        ReplayError::Sim { theirs, ours } => json!({ "key": "replay.error.sim", "vars": { "theirs": theirs, "ours": ours } }),
        ReplayError::Damaged => json!({ "key": "replay.error.damaged", "vars": {} }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fill a copy string the way the page does, for a test to read.
    fn fill(v: &Value) -> String {
        let copy = crate::copy::copy();
        let mut s = v["key"].as_str().unwrap().split('.').fold(&copy, |o, k| &o[k]).as_str().unwrap().to_string();
        s = s.replace("{game}", copy["game"]["name"].as_str().unwrap());
        for (k, val) in v["vars"].as_object().unwrap() {
            let text = val.as_str().map(str::to_string).unwrap_or_else(|| val.to_string());
            s = s.replace(&format!("{{{k}}}"), &text);
        }
        assert!(!s.contains('{'), "an unfilled placeholder in {s}");
        s
    }

    fn result(loser: Option<u8>, cause: Cause, part: u8, by: u8) -> RoundResult {
        RoundResult { loser, cause, part, by }
    }

    fn world() -> World {
        World::new(crate::setup::versus(1, sim::balance::DEFAULT_TUNING))
    }

    #[test]
    fn the_result_names_the_cut_that_ended_the_round() {
        let w = world();
        // Part 1 of the fighter body is the neck, 0 the chest, 6 a hand.
        let cases = [
            (result(Some(1), Cause::Neck, 1, 0), "Indigo won the round. Indigo's blade cut across Ochre's neck."),
            (result(Some(0), Cause::Heart, 0, 1), "Ochre won the round. Ochre's blade cut across Indigo's heart."),
            (result(Some(1), Cause::Ink, 6, 1), "Indigo won the round. Ochre ran out of ink."),
            (result(Some(1), Cause::Neck, 2, 1), "Indigo won the round. Ochre's own blade made the cut that ended it."),
            (result(None, Cause::Neck, 1, 0), "Nobody won the round. Both fighters stopped at the same moment, so the round is played again."),
        ];
        for (r, want) in cases {
            assert_eq!(fill(&round_result(&w, &r, Audience::Versus)), want);
        }
        let road = Audience::Road { opponent: "gatekeeper" };
        assert_eq!(fill(&round_result(&w, &result(Some(0), Cause::Neck, 1, 1), road)),
            "You lost the round. The Gatekeeper's blade cut across your neck.");
        assert_eq!(fill(&round_result(&w, &result(Some(0), Cause::Ink, 6, 0), road)),
            "You lost the round. You ran out of ink, and the cut to your hand spilled the most.");
        assert_eq!(fill(&round_result(&w, &result(Some(1), Cause::Heart, 0, 0), road)),
            "You won the round. Your blade cut across the gatekeeper's heart.");
    }

    #[test]
    fn a_replay_from_another_sim_version_is_refused_with_a_sentence() {
        let s = fill(&replay_error(ReplayError::Sim { theirs: 3, ours: 1 }));
        assert_eq!(
            s,
            "This replay was recorded with simulation version 3, and this build runs version 1, \
             so the replay was not loaded."
        );
        assert!(fill(&replay_error(ReplayError::Damaged)).starts_with("This replay file is incomplete"));
        assert!(fill(&replay_error(ReplayError::Format)).contains("is not a Vagrancy replay"));
    }

    #[test]
    fn a_round_ends_with_a_heading_for_how_it_ended_and_a_headshot_stops_the_clock_where_it_landed() {
        let mut w = world();
        let parts = &w.setup.bodies[0].parts;
        let idx = |name: &str| parts.iter().position(|p| p.copy == name).unwrap() as u8;
        let (head, neck, chest) = (idx("head"), idx("neck"), idx("chest"));
        let key = |w: &World, r: &RoundResult| popup(w, r)["key"].as_str().unwrap().to_string();
        assert_eq!(key(&w, &result(Some(1), Cause::Neck, head, 0)), "results.popup.head");
        assert_eq!(key(&w, &result(Some(1), Cause::Neck, neck, 0)), "results.popup.neck");
        assert_eq!(key(&w, &result(Some(1), Cause::Heart, chest, 0)), "results.popup.heart");
        assert_eq!(key(&w, &result(Some(0), Cause::Ink, chest, 0)), "results.popup.ink");
        assert_eq!(key(&w, &result(None, Cause::Neck, head, 0)), "results.popup.draw");
        // Only a cut across the head stops the clock, and it looks where the
        // blade landed on that tick.
        let at = sim::fx::V2::cm(212, 151);
        w.events.push(sim::fight::Event::Cut { seat: 1, part: head, by: 0, at, spilled: true });
        assert_eq!(focus(&w, &result(Some(1), Cause::Neck, head, 0)), json!([at.x.0, at.y.0]));
        assert_eq!(focus(&w, &result(Some(1), Cause::Neck, neck, 0)), Value::Null);
        assert_eq!(focus(&w, &result(Some(1), Cause::Heart, chest, 0)), Value::Null);
        // Without the cut among this tick's events, it looks at the head.
        w.events.clear();
        let h = pilot::head(&w, 1).unwrap();
        assert_eq!(focus(&w, &result(Some(1), Cause::Neck, head, 0)), json!([h.x.0, h.y.0]));
    }
}
