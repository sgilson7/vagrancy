//! The BT Lab (Sam, 2026-10-06: a classroom demo of "how the high level
//! behaviors and nodes get converted into controls execution, like how does
//! the behavior tree high guard get converted to inputs that control the
//! shoulder and elbows"). What a pilot says it pressed, and why, must be
//! what it pressed: the lab shows `Pilot::explain` beside the keys.

use pilot::moves::{recipe, Recipe};
use pilot::Pilot;
use sim::fight::Phase;
use sim::{Input, World};

const ARM: u16 = Input::SHOULDER_UP | Input::SHOULDER_DOWN | Input::ELBOW_IN | Input::ELBOW_OUT;
const LEG: u16 = Input::STEP_LEFT | Input::STEP_RIGHT | Input::JUMP | Input::DODGE | Input::STAND;

/// Each tick of a match against the yardstick: the opponent's keys and its
/// explanation.
fn ticks(id: &str, n: u32) -> Vec<(Input, Vec<pilot::moves::Explain>)> {
    let mut w = World::new(content::setup::road(1, sim::balance::DEFAULT_TUNING, id));
    let mut ps = content::road::lineup(&content::road::pilot("yardstick"), id);
    let mut last = [Input::NONE; sim::body::SEATS];
    let mut out = Vec::new();
    while w.tick < n && !matches!(w.phase, Phase::MatchOver { .. }) {
        let mut i = [Input::NONE; sim::body::SEATS];
        for (k, p) in ps.iter_mut().enumerate() {
            p.observe(last);
            i[k] = p.input(&w, k);
        }
        if matches!(w.phase, Phase::Fight) {
            out.push((i[1], ps[1].explain()));
        }
        w.step_all(i);
        last = i;
    }
    out
}

#[test]
fn what_a_tree_says_it_pressed_is_what_it_pressed_and_comes_from_its_recipe() {
    let mut checked = [0; 4];
    for id in ["harpooner", "archivist", "shepherd", "woodcutter", "general", "acrobat"] {
        for (keys, ex) in ticks(id, 1500) {
            let e = &ex[0];
            let Some(mv) = &e.mv else { continue };
            assert_eq!(e.keys, keys.0, "{id}: said {:#b}, pressed {:#b}, running {mv}", e.keys, keys.0);
            match recipe(mv).unwrap_or_else(|| panic!("{id} ran {mv}, which has no recipe")) {
                Recipe::Script { beats } => {
                    let b = beats[e.beat.expect("a script names its beat")];
                    assert!(b.from <= e.t && e.t <= b.to, "{id}: {mv} at tick {} said beat {b:?}", e.t);
                    assert_eq!(keys.0, b.keys | e.step_keys, "{id}: {mv}'s keys are not its beat's");
                    checked[0] += 1;
                }
                Recipe::Pose { .. } => {
                    let p = e.pose.expect("a pose gives its joints");
                    assert_eq!(keys.0, p.shoulder.key | p.elbow.key, "{id}: {mv}'s keys are not its joints'");
                    checked[1] += 1;
                }
                Recipe::Search { .. } => {
                    assert_eq!(keys.0, e.search.as_ref().expect("a search gives its choice").chosen, "{id}: the search pressed what it did not choose");
                    checked[2] += 1;
                }
                Recipe::Throw => {
                    assert!(e.throw_step.is_some(), "{id}: a throw without its step");
                    checked[3] += 1;
                }
            }
        }
    }
    assert!(checked.iter().all(|&c| c > 0), "a kind of move never ran: scripts, poses, searches, throws {checked:?}");
}

#[test]
fn the_village_deitys_keys_are_each_trees_keys_under_its_mask() {
    let arm2 = Input::ARMS2.iter().fold(0, |a, b| a | b);
    let mut checked = 0;
    for (keys, ex) in ticks("local_deity", 900) {
        assert_eq!(ex.len(), 3);
        let parts: Vec<&str> = ex.iter().map(|e| e.part.unwrap()).collect();
        assert_eq!(parts, ["tree.many.arms", "tree.many.upper", "tree.many.legs"]);
        let want = (ex[0].keys & ARM) | (ex[1].keys & arm2) | (ex[2].keys & LEG);
        assert_eq!(keys.0, want, "the deity pressed {:#b}, its trees said {:#b}", keys.0, want);
        checked += 1;
    }
    assert!(checked > 100);
}

#[test]
fn each_move_a_pilot_names_has_a_recipe_and_words() {
    let copy: serde_json::Value = serde_json::from_str(content::copy::COPY_JSON).unwrap();
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/pilots.json")).unwrap();
    for name in text.split("\"do\": \"").skip(1).map(|r| r.split('"').next().unwrap()) {
        assert!(recipe(name).is_some(), "a tree runs {name}, which has no recipe");
        assert!(copy["tree"]["act"][name].is_string() || name == "search", "no words for {name}");
    }
    for m in pilot::moves::MOVES {
        assert!(recipe(m).is_some(), "{m} is listed and has no recipe");
    }
}
