//! The player (Sam, 2026-10-07): an agent that plays arcade mode for a
//! video, "a basic version of the searcher that slowly improves its gameplay
//! every fight", through "the first 20 fights of the game, with the last one
//! being a searcher like the reader". It looks a little way ahead and weighs
//! a few key combinations, a little further and a few more with each fight
//! it plays, until it plans as the reader does. Which fight it plays next is
//! decided here from the save's own results, by the road's own rules: a fight
//! is played only when it is open, and a requirement it has not met (a win
//! without losing a round, a quick win) sends it back to that fight.

use crate::road::{open, road, stop, Best, Req};
use std::collections::BTreeMap;

/// The fight the run ends on: a searcher.
pub const LAST: &str = "reader";
/// How many fights the run sets out to win, the last one included.
pub const FIGHTS: usize = 20;
/// The level at which the player plans as the reader does.
pub const TOP: u32 = 19;

/// The player at `level` (the fights it has played so far): from a short,
/// slow look ahead at a few combinations, to the reader's own.
pub fn player(level: u32) -> pilot::Spec {
    let pilot::Spec::Search { horizon_ticks, reaction_ticks, branches, period } = crate::road::pilot(LAST) else {
        panic!("the run's last fight is not a searcher");
    };
    let k = level.min(TOP) as i64;
    let lerp = |from: u32, to: u32| (from as i64 + (to as i64 - from as i64) * k / TOP as i64) as u32;
    pilot::Spec::Search {
        horizon_ticks: lerp(horizon_ticks / 3, horizon_ticks),
        reaction_ticks: lerp(reaction_ticks * 5 / 2, reaction_ticks),
        branches: lerp(3, branches),
        period,
    }
}

/// The fights the run sets out to win, in an order the road allows: the
/// ones the last fight needs, before it, and others open along the way
/// that ask only for wins, to make up the count; the last fight last.
pub fn targets() -> Vec<String> {
    let road = road();
    let mut need: Vec<String> = Vec::new();
    fn close(id: &str, need: &mut Vec<String>) {
        if need.iter().any(|n| n == id) {
            return;
        }
        need.push(id.to_string());
        for r in stop(id).map(|s| s.requires).unwrap_or_default() {
            close(r.stop(), need);
        }
    }
    close(LAST, &mut need);
    let mut out: Vec<String> = Vec::new();
    let done = |out: &Vec<String>, id: &str| out.iter().any(|o| o == id);
    while out.len() + 1 < FIGHTS.max(need.len()) {
        // Each pass takes the first fight, in the road's order, whose
        // requirements are among those already taken: a needed one first,
        // else one that asks only for wins.
        let ready = |s: &&crate::road::Stop, needed: bool| {
            s.id != LAST && !s.secret && !done(&out, &s.id) && needed == need.contains(&s.id)
                && s.requires.iter().all(|r| done(&out, r.stop()))
                && (needed || s.requires.iter().all(|r| matches!(r, Req::Beat(_))))
        };
        let pick = road.iter().find(|s| ready(s, true)).or_else(|| {
            let left = need.iter().filter(|n| n.as_str() != LAST && !done(&out, n)).count();
            if out.len() + 1 + left < FIGHTS { road.iter().find(|s| ready(s, false)) } else { None }
        });
        match pick {
            Some(s) => out.push(s.id.clone()),
            None => break,
        }
    }
    out.push(LAST.to_string());
    out
}

/// The fight to play next with these results, or none once the last is won.
pub fn next(best: &BTreeMap<String, Best>) -> Option<String> {
    for t in targets() {
        if best.contains_key(&t) {
            continue;
        }
        let st = stop(&t)?;
        if open(&st, best) {
            return Some(t);
        }
        // A requirement not yet met: play the fight it names again.
        return st.requires.iter().find(|r| !r.met(best)).map(|r| r.stop().to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_run_is_twenty_fights_the_road_allows_ending_on_a_searcher() {
        let t = targets();
        assert_eq!(t.len(), FIGHTS, "{t:?}");
        assert_eq!(t.last().map(String::as_str), Some(LAST));
        assert_eq!(crate::road::pilot(LAST).kind(), "search");
        // Winning each, flawless and quick, in order: each is open when its
        // turn comes.
        let mut best = BTreeMap::new();
        for id in &t {
            assert!(open(&stop(id).unwrap(), &best), "{id} is not open in its turn");
            best.insert(id.clone(), Best { losses: 0, ticks: 600, ..Best::won(0, 600, "sword") });
        }
        assert_eq!(next(&best), None);
    }

    #[test]
    fn an_unmet_requirement_sends_the_player_back_to_the_fight_it_names() {
        let t = targets();
        let mut best = BTreeMap::new();
        // Each target before the drover won, the scarecrow with a round lost.
        for id in t.iter().take_while(|id| id.as_str() != "drover") {
            best.insert(id.clone(), Best::won(if id == "scarecrow" { 1 } else { 0 }, 600, "sword"));
        }
        if t.iter().any(|id| id == "drover") {
            assert_eq!(next(&best).as_deref(), Some("scarecrow"), "the player went on without the scarecrow won flawless");
        }
    }

    #[test]
    fn the_player_plans_further_with_each_fight_until_it_plans_as_the_reader() {
        let at = |l| match player(l) { pilot::Spec::Search { horizon_ticks, branches, .. } => (horizon_ticks, branches), _ => unreachable!() };
        assert!(at(0).0 < at(10).0 && at(10).0 <= at(TOP).0 && at(0).1 < at(TOP).1);
        assert_eq!(format!("{:?}", player(TOP)), format!("{:?}", crate::road::pilot(LAST)));
        assert_eq!(format!("{:?}", player(TOP + 5)), format!("{:?}", player(TOP)));
    }
}
