//! Watch mode (Sam, 2026-10-06: "an enemy vs enemy ai watching mode ... and
//! have it so each characters behavior tree is being shown above their head
//! as they battle").

use content::messages::{phase_text, Audience};
use sim::fight::Phase;
use sim::{Input, World};

#[test]
fn each_side_fights_in_the_body_it_fights_in_on_the_road() {
    let w = World::new(content::setup::exhibition(1, sim::balance::DEFAULT_TUNING, ["local_deity", "scarecrow"], "flat"));
    assert_eq!(w.swords_of(0).len(), 2, "the local deity came without its four arms");
    assert_eq!(w.swords_of(1).len(), 0, "the scarecrow came armed");
    let w = World::new(content::setup::exhibition(1, sim::balance::DEFAULT_TUNING, ["juggler", "harpooner"], "flat"));
    let road = World::new(content::setup::road(1, sim::balance::DEFAULT_TUNING, "harpooner"));
    let body = |w: &World, seat: usize| w.setup.bodies[w.fighters[seat].as_ref().unwrap().body as usize].clone();
    assert_eq!(body(&w, 1), body(&road, 1), "the harpooner's body is not its road body");
}

#[test]
fn two_opponents_play_a_match_to_its_end_and_each_tree_lights() {
    // Each pilot drives its own seat; between rounds both press ready.
    let ids = ["thresher", "juggler"];
    let mut w = World::new(content::setup::exhibition(4, sim::balance::DEFAULT_TUNING, ids, "flat"));
    let mut ps: Vec<_> = ids.iter().map(|id| pilot::build(&content::road::pilot(id))).collect();
    let mut last = [Input::NONE; sim::body::SEATS];
    let mut lit = [false; 2];
    while w.tick < 7200 && !matches!(w.phase, Phase::MatchOver { .. }) {
        let mut i = [Input::NONE; sim::body::SEATS];
        for (k, p) in ps.iter_mut().enumerate() {
            p.observe(last);
            i[k] = p.input(&w, k);
            lit[k] |= !p.trace().active.is_empty();
        }
        w.step_all(i);
        last = i;
    }
    assert!(matches!(w.phase, Phase::MatchOver { .. }), "the match was not over after {} ticks: {:?}", w.tick, w.wins);
    assert_eq!(lit, [true, true], "a side's tree never lit");
}

#[test]
fn the_sentences_name_each_side_by_its_opponent_and_a_mirror_by_color() {
    let copy: serde_json::Value = serde_json::from_str(content::copy::COPY_JSON).unwrap();
    let name = |k: &str| copy.pointer(k).unwrap().as_str().unwrap().to_string();
    // The thrower's sword ends the first round (crates/content/tests/
    // throw.rs), so there is a sentence to read.
    let play = |ids: [&str; 2]| {
        let mut w = World::new(content::setup::exhibition(1, sim::balance::DEFAULT_TUNING, ids, "flat"));
        for t in 0..400u32 {
            let k = match t { 0..=49 => Input::STEP_RIGHT, 50..=51 => Input::SHOULDER_UP, 52 => Input::SHOULDER_UP | Input::THROW, _ => 0 };
            w.step([Input(k), Input::NONE]);
            if !matches!(w.phase, Phase::Fight) {
                break;
            }
        }
        phase_text(&w, Audience::Exhibition { ids })["round"]["vars"].clone()
    };
    let v = play(["thresher", "courier"]);
    assert_eq!(v["winner"], name("/opponents/thresher/name"), "{v}");
    assert_eq!(v["loser"], name("/opponents/courier/name"), "{v}");
    let v = play(["thresher", "thresher"]);
    assert_eq!(v["winner"], name("/fighters/left/name"), "{v}");
    assert_eq!(v["loser"], name("/fighters/right/name"), "{v}");
}
