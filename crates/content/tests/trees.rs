//! Each opponent as a behavior tree (Sam, 2026-10-06: an encyclopedia, and
//! a floating tree above an opponent "where the state lights up as they
//! enter it"). The tree comes from the pilot's own data, and the trace from
//! the pilot as it plays, so the picture says what the pilot does.

use content::road::{lineup, pilot, pilots, stops};
use pilot::view::{describe, walk, Kind};
use serde_json::Value;
use sim::fight::Phase;
use sim::{Input, World};

fn copy() -> Value {
    serde_json::from_str(content::copy::COPY_JSON).unwrap()
}

fn fill(c: &Value, l: &pilot::view::Label) -> String {
    let mut s = l.key.split('.').fold(c, |o, k| &o[k]).as_str().unwrap_or_else(|| panic!("no copy string at {}", l.key)).to_string();
    for (k, v) in &l.vars {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

#[test]
fn every_opponents_tree_numbers_its_nodes_and_names_them_in_the_copy() {
    let c = copy();
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/icons/");
    for (id, spec) in pilots() {
        let t = describe(&spec);
        let nodes = walk(&t);
        for (i, n) in nodes.iter().enumerate() {
            assert_eq!(n.id as usize, i, "{id}: node ids are not depth first from 0");
            let text = fill(&c, &n.label);
            assert!(!text.contains('{'), "{id}: node {} reads {text:?}, a placeholder unfilled", n.id);
            assert!(std::path::Path::new(&format!("{root}{}.png", n.icon)).exists(), "{id}: no icon web/icons/{}.png", n.icon);
            let leaf = matches!(n.kind, Kind::Condition | Kind::Action | Kind::Search);
            assert_eq!(leaf, n.children.is_empty(), "{id}: node {} is a {:?} with {} children", n.id, n.kind, n.children.len());
        }
    }
}

#[test]
fn the_trace_lights_only_nodes_the_tree_has_and_the_move_the_pilot_is_running() {
    // Every opponent against the yardstick, two seconds of a round: the
    // trace names nodes of its tree; only conditions are found to hold or
    // fail; the lit path runs from the root; and at least one move lights.
    for id in stops() {
        let mut w = World::new(content::setup::road(0, sim::balance::DEFAULT_TUNING, &id));
        let mut ps = lineup(&pilot("yardstick"), &id);
        let tree = describe(&pilot(&id));
        let nodes = walk(&tree);
        let mut last = [Input::NONE; sim::body::SEATS];
        let mut lit = 0;
        while w.tick < 600 && matches!(w.phase, Phase::Fight) {
            let mut i = [Input::NONE; sim::body::SEATS];
            for (k, p) in ps.iter_mut().enumerate() {
                p.observe(last);
                i[k] = p.input(&w, k);
            }
            let t = ps[1].trace();
            for &n in t.active.iter().chain(&t.held).chain(&t.failed) {
                assert!((n as usize) < nodes.len(), "{id}: the trace names node {n}, which the tree does not have");
            }
            for &n in t.held.iter().chain(&t.failed) {
                assert_eq!(nodes[n as usize].kind, Kind::Condition, "{id}: node {n} found to hold or fail is not a condition");
            }
            if let Some(&first) = t.active.first() {
                assert_eq!(first, 0, "{id}: the lit path does not start at the root");
                let leaf = nodes[*t.active.last().unwrap() as usize];
                assert!(matches!(leaf.kind, Kind::Action | Kind::Search), "{id}: the lit path ends on a {:?}", leaf.kind);
                lit += 1;
            }
            w.step_all(i);
            last = i;
        }
        assert!(lit > 0, "{id}: nothing in its tree ever lit");
    }
}

#[test]
fn a_tree_pilots_lit_move_is_the_move_it_runs() {
    // The archivist and the harpooner against the yardstick: on every tick a
    // move is running, the lit leaf is that move's node.
    for id in ["archivist", "harpooner", "drover"] {
        let pilot::Spec::Tree { rules, reaction_ticks, salt, .. } = pilot(id) else { panic!("{id} is a tree") };
        let mut me = pilot::Tree::new(rules, reaction_ticks, salt);
        let mut them = pilot::build(&pilot("yardstick"));
        let tree = describe(&pilot(id));
        let nodes = walk(&tree);
        let mut w = World::new(content::setup::road(0, sim::balance::DEFAULT_TUNING, id));
        let mut last = [Input::NONE; sim::body::SEATS];
        let mut checked = 0;
        use pilot::Pilot;
        while w.tick < 1800 && !matches!(w.phase, Phase::MatchOver { .. }) {
            me.observe(last);
            them.observe(last);
            let i = [them.input(&w, 0), me.input(&w, 1)];
            if let (Some((name, _)), Phase::Fight) = (me.current_move(), &w.phase) {
                let leaf = nodes[*me.trace().active.last().expect("a move runs and nothing is lit") as usize];
                let want = if name == "search" { "tree.search".to_string() } else { format!("tree.act.{name}") };
                assert_eq!(leaf.label.key, want, "{id} at tick {}", w.tick);
                checked += 1;
            }
            w.step(i);
            last = [i[0], i[1], Input::NONE];
        }
        assert!(checked > 100, "{id}: only {checked} ticks with a move");
    }
}

#[test]
fn the_drawn_trees_are_current() {
    // web/trees and web/icons are drawn by analysis/trees/render.py from what
    // `lab trees` prints (`make trees`); TeX is not on CI, so the images are
    // committed, and this fails when the pilots, the labels, the icons or the
    // renderer have changed since they were drawn.
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let mut bytes = content::trees::json().into_bytes();
    bytes.extend(std::fs::read(format!("{root}analysis/trees/icons.tex")).unwrap());
    bytes.extend(std::fs::read(format!("{root}analysis/trees/render.py")).unwrap());
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(format!("{root}web/trees/manifest.json")).expect("web/trees/manifest.json: run `make trees`")).unwrap();
    assert_eq!(manifest["fingerprint"].as_str().unwrap(), content::trees::fingerprint(&bytes), "the drawn trees are stale: run `make trees`");
    for id in pilots().keys() {
        assert!(std::path::Path::new(&format!("{root}web/trees/{id}.png")).exists(), "no drawn tree for {id}");
    }
}
