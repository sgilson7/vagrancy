//! Trees written in the BT Lab's editor (content::custom).

use content::custom::{parse, CONDITIONS, MAX_RULES};
use pilot::Pilot;
use sim::{Input, World};

fn written(id: &str) -> String {
    let all: serde_json::Value = serde_json::from_str(content::road::PILOTS_JSON).unwrap();
    let p = &all[id];
    serde_json::json!({ "name": "test", "reaction_ticks": p["reaction_ticks"], "rules": p["rules"] }).to_string()
}

#[test]
fn a_written_tree_presses_the_keys_the_same_tree_from_the_data_presses() {
    // The lamplighter's tree, written out as the editor writes one: against
    // the same still opponent, tick for tick the same keys.
    let mut keys = Vec::new();
    for spec in [content::road::pilot("lamplighter"), parse(&written("lamplighter")).expect("the lamplighter's tree is accepted")] {
        let mut w = World::new(content::custom::watch(3, sim::balance::DEFAULT_TUNING, "scarecrow", "flat"));
        let mut p = pilot::build(&spec);
        let mut run = Vec::new();
        for _ in 0..600 {
            let i = p.input(&w, 0);
            run.push(i.0);
            w.step([i, Input::NONE]);
        }
        keys.push(run);
    }
    assert_eq!(keys[0], keys[1], "the written tree pressed different keys");
    assert!(keys[0].iter().any(|&k| k != 0), "the lamplighter pressed nothing");
}

#[test]
fn a_tree_that_cannot_run_is_refused_with_its_reason() {
    let refused = |j: &str| parse(j).err().map(|r| r.key);
    assert_eq!(refused("not json"), Some("btlab.editor.refuse.shape"));
    assert_eq!(refused(r#"{"reaction_ticks": 1, "rules": [{"do": "approach"}]}"#), Some("btlab.editor.refuse.reaction"));
    assert_eq!(refused(r#"{"reaction_ticks": 10, "rules": []}"#), Some("btlab.editor.refuse.rules"));
    let many = format!(r#"{{"reaction_ticks": 10, "rules": [{}]}}"#, vec![r#"{"do": "guard"}"#; MAX_RULES + 1].join(","));
    assert_eq!(refused(&many), Some("btlab.editor.refuse.rules"));
    assert_eq!(refused(r#"{"reaction_ticks": 10, "rules": [{"do": "fly"}]}"#), Some("btlab.editor.refuse.move"));
    assert_eq!(refused(r#"{"reaction_ticks": 10, "rules": [{"if": [{"chance": 140}], "do": "guard"}]}"#), Some("btlab.editor.refuse.number"));
    assert_eq!(refused(r#"{"reaction_ticks": 10, "rules": [{"if": ["me_down","armed","unarmed","opp_down","on_ledge"], "do": "guard"}]}"#), Some("btlab.editor.refuse.conditions"));
    assert!(parse(r#"{"reaction_ticks": 10, "rules": [{"if": [{"gap_above": 200}], "do": "approach"}, {"do": "guard"}]}"#).is_ok());
}

#[test]
fn each_condition_the_editor_offers_is_one_a_tree_can_hold_and_has_words() {
    let copy: serde_json::Value = serde_json::from_str(content::copy::COPY_JSON).unwrap();
    for (name, unit) in CONDITIONS {
        let c = match unit {
            Some(_) => serde_json::json!({ name: 50 }),
            None => serde_json::json!(name),
        };
        let tree = serde_json::json!({ "reaction_ticks": 10, "rules": [{ "if": [c], "do": "guard" }] }).to_string();
        assert!(parse(&tree).is_ok(), "{name} is offered and not accepted");
        assert!(copy["tree"]["cond"][name].is_string(), "{name} has no words");
    }
}
