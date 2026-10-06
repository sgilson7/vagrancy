//! A pilot as a behavior tree, for people to read (Sam, 2026-10-06: an
//! encyclopedia, and a floating tree above an opponent "where the state
//! lights up as they enter it").
//!
//! `describe` turns a pilot's data into a tree of nodes; each pilot reports,
//! tick by tick, which of those nodes ran (`Trace`). Both come from here, so
//! the drawing cannot say one thing while the pilot does another. Labels are
//! copy keys with their values (`tree.*` in data/copy.en.json); the page and
//! the figure generator fill them, and neither writes a word of its own.
//!
//! The shapes follow the usual behavior tree notation: a selector (`?`)
//! runs the first child that can run, a sequence (`→`) runs its children in
//! order and stops at the first that fails, a parallel (`⇉`) runs its
//! children together, a repeat loops its child.

use crate::{Cond, Rule, Spec};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Selector,
    Sequence,
    Parallel,
    Repeat,
    /// A test of what the pilot sees.
    Condition,
    /// A move: a few ticks of keys.
    Action,
    /// The short look-ahead search.
    Search,
}

/// A copy key and the values its placeholders take.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Label {
    pub key: String,
    pub vars: BTreeMap<String, String>,
}

impl Label {
    fn of(key: &str) -> Label {
        Label { key: key.to_string(), vars: BTreeMap::new() }
    }
    fn with(key: &str, vars: &[(&str, String)]) -> Label {
        Label { key: key.to_string(), vars: vars.iter().map(|(k, v)| (k.to_string(), v.clone())).collect() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Node {
    /// Numbered depth first from 0 at the root; a `Trace` names nodes by it.
    pub id: u16,
    pub kind: Kind,
    pub label: Label,
    /// The icon to draw for it (web/icons/<icon>.png).
    pub icon: String,
    /// Checked every tick, so it can cut a running move short.
    pub interrupt: bool,
    pub children: Vec<Node>,
}

/// What ran on the last tick: the nodes on the path to the running move,
/// and the conditions checked on the last decision with what each found.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Trace {
    pub active: Vec<u16>,
    pub held: Vec<u16>,
    pub failed: Vec<u16>,
}

/// Ticks to milliseconds, rounded up, as the introductions state them.
fn ms(ticks: u32) -> String {
    (ticks * 1000).div_ceil(sim::balance::TICKS_PER_SECOND).to_string()
}

/// Ticks to seconds with one decimal where it needs one.
fn secs(ticks: u32) -> String {
    let tenths = (ticks * 10).div_ceil(sim::balance::TICKS_PER_SECOND);
    if tenths % 10 == 0 { format!("{}", tenths / 10) } else { format!("{}.{}", tenths / 10, tenths % 10) }
}

/// Builds nodes with ids in depth-first order.
struct Ids(u16);
impl Ids {
    fn next(&mut self) -> u16 {
        self.0 += 1;
        self.0 - 1
    }
}

fn leaf(ids: &mut Ids, kind: Kind, label: Label, icon: &str) -> Node {
    Node { id: ids.next(), kind, label, icon: icon.to_string(), interrupt: false, children: Vec::new() }
}

fn branch(id: u16, kind: Kind, label: Label, icon: &str, children: Vec<Node>) -> Node {
    Node { id, kind, label, icon: icon.to_string(), interrupt: false, children }
}

pub fn cond_label(c: &Cond) -> (Label, &'static str) {
    let cm = |k: &str, v: i32| Label::with(k, &[("cm", v.to_string())]);
    match c {
        Cond::GapAbove(v) => (cm("tree.cond.gap_above", *v), "far"),
        Cond::GapBelow(v) => (cm("tree.cond.gap_below", *v), "near"),
        Cond::TipNearHead(v) => (cm("tree.cond.tip_near_head", *v), "threat_head"),
        Cond::TipNearBody(v) => (cm("tree.cond.tip_near_body", *v), "threat_body"),
        Cond::MeAirborne => (Label::of("tree.cond.me_airborne"), "airborne"),
        Cond::MeGrounded => (Label::of("tree.cond.me_grounded"), "grounded"),
        Cond::OppAirborne => (Label::of("tree.cond.opp_airborne"), "opp_airborne"),
        Cond::MeDown => (Label::of("tree.cond.me_down"), "down"),
        Cond::OppDodging => (Label::of("tree.cond.opp_dodging"), "opp_dodging"),
        Cond::MyInkBelow(p) => (Label::with("tree.cond.my_ink_below", &[("pct", p.to_string())]), "ink"),
        Cond::Chance(p) => (Label::with("tree.cond.chance", &[("pct", p.to_string())]), "chance"),
        Cond::AllyEngaged(v) => (cm("tree.cond.ally_engaged", *v), "ally"),
        Cond::OppAbove(v) => (cm("tree.cond.opp_above", *v), "opp_above"),
        Cond::OppBelow(v) => (cm("tree.cond.opp_below", *v), "opp_below"),
        Cond::LedgeOverhead => (Label::of("tree.cond.ledge_overhead"), "ledge"),
        Cond::OnLedge => (Label::of("tree.cond.on_ledge"), "on_ledge"),
        Cond::Armed => (Label::of("tree.cond.armed"), "armed"),
        Cond::Unarmed => (Label::of("tree.cond.unarmed"), "unarmed"),
        Cond::OppDown => (Label::of("tree.cond.opp_down"), "opp_down"),
    }
}

/// The search a tree's engage move runs (`Tree::play`'s `Search::new`).
pub const TREE_SEARCH: (u32, u32, u32) = (18, 11, 3);

fn search_node(ids: &mut Ids, horizon: u32, reaction: u32, branches: u32) -> Node {
    leaf(
        ids,
        Kind::Search,
        Label::with("tree.search", &[("horizon_ms", ms(horizon)), ("reaction_ms", ms(reaction)), ("branches", branches.to_string())]),
        "search",
    )
}

/// The ids a tree's rules take, for its trace.
#[derive(Clone, Debug, Default)]
pub struct RuleIds {
    pub seq: Option<u16>,
    pub conds: Vec<u16>,
    pub act: u16,
}

/// A pilot as a tree.
pub fn describe(spec: &Spec) -> Node {
    let mut ids = Ids(0);
    match spec {
        Spec::Still { .. } => leaf(&mut ids, Kind::Action, Label::of("tree.act.still"), "still"),
        Spec::Pose { .. } => leaf(&mut ids, Kind::Action, Label::of("tree.act.hold_pose"), "guard"),
        Spec::Loop { pattern, pause_ticks, drift } => {
            let root = ids.next();
            let rep = ids.next();
            let seq = ids.next();
            let steps: Vec<Node> = match pattern.as_str() {
                "overhead" => vec![
                    leaf(&mut ids, Kind::Action, Label::of("tree.act.raise"), "raise"),
                    leaf(&mut ids, Kind::Action, Label::of("tree.act.cut_down"), "overhead"),
                    leaf(&mut ids, Kind::Action, Label::with("tree.act.pause", &[("pause_s", secs((*pause_ticks).max(1)))]), "wait"),
                ],
                "spin" => vec![leaf(&mut ids, Kind::Action, Label::of("tree.act.spin"), "spin")],
                _ => vec![leaf(&mut ids, Kind::Action, Label::of("tree.act.wait"), "wait")],
            };
            let walk = if *drift {
                leaf(&mut ids, Kind::Action, Label::of("tree.act.drift"), "approach")
            } else {
                let s = ids.next();
                let c = leaf(&mut ids, Kind::Condition, Label::with("tree.cond.gap_above", &[("cm", "200".into())]), "far");
                let a = leaf(&mut ids, Kind::Action, Label::of("tree.act.approach"), "approach");
                branch(s, Kind::Sequence, Label::of("tree.kind.sequence"), "sequence", vec![c, a])
            };
            let seqn = branch(seq, Kind::Sequence, Label::of("tree.kind.sequence"), "sequence", steps);
            let repn = branch(rep, Kind::Repeat, Label::of("tree.kind.repeat"), "repeat", vec![seqn]);
            branch(root, Kind::Parallel, Label::of("tree.kind.parallel"), "parallel", vec![repn, walk])
        }
        Spec::Machine { script } => {
            let root = ids.next();
            let seq = ids.next();
            let states: Vec<Node> = match script.as_str() {
                "late_heavy" => vec![
                    leaf(&mut ids, Kind::Action, Label::with("tree.act.wait_in_guard", &[("cm", "230".into())]), "guard"),
                    leaf(&mut ids, Kind::Action, Label::of("tree.act.heavy_stroke"), "overhead"),
                    leaf(&mut ids, Kind::Action, Label::of("tree.act.recover"), "retreat"),
                ],
                "vault" => vec![
                    leaf(&mut ids, Kind::Action, Label::with("tree.act.close_to", &[("cm", "210".into())]), "approach"),
                    leaf(&mut ids, Kind::Action, Label::of("tree.act.pogo"), "pogo"),
                    leaf(&mut ids, Kind::Action, Label::of("tree.act.cut_down"), "overhead"),
                ],
                _ => vec![leaf(&mut ids, Kind::Action, Label::of("tree.act.wait"), "wait")],
            };
            let seqn = branch(seq, Kind::Sequence, Label::of("tree.kind.sequence"), "sequence", states);
            branch(root, Kind::Repeat, Label::of("tree.kind.repeat"), "repeat", vec![seqn])
        }
        Spec::Replayer { .. } => {
            let root = ids.next();
            let seq = ids.next();
            let c = leaf(&mut ids, Kind::Condition, Label::of("tree.cond.first_round"), "first_round");
            let a = leaf(&mut ids, Kind::Action, Label::of("tree.act.watch"), "watch");
            let m = leaf(&mut ids, Kind::Action, Label::of("tree.act.mirror"), "mirror");
            let s = branch(seq, Kind::Sequence, Label::of("tree.kind.sequence"), "sequence", vec![c, a]);
            branch(root, Kind::Selector, Label::of("tree.kind.selector"), "selector", vec![s, m])
        }
        Spec::Search { horizon_ticks, reaction_ticks, branches, .. } => search_node(&mut ids, *horizon_ticks, *reaction_ticks, *branches),
        Spec::Tree { reaction_ticks, rules, .. } => tree_nodes(rules, *reaction_ticks).0,
        Spec::Many { arms, upper, legs } => {
            // One parallel root over the three trees, each relabelled with
            // what it runs and renumbered after the one before.
            let offsets = many_offsets(arms, upper);
            let kids = [(arms, "tree.many.arms"), (upper, "tree.many.upper"), (legs, "tree.many.legs")]
                .into_iter()
                .zip(offsets)
                .map(|((spec, key), off)| {
                    let mut n = describe(spec);
                    n.label = Label::of(key);
                    shift(&mut n, off);
                    n
                })
                .collect();
            branch(ids.next(), Kind::Parallel, Label::of("tree.kind.parallel"), "parallel", kids)
        }
    }
}

/// Where each of a `Spec::Many`'s three trees starts numbering, after the
/// root.
pub fn many_offsets(arms: &Spec, upper: &Spec) -> [u16; 3] {
    let n = |s: &Spec| walk(&describe(s)).len() as u16;
    [1, 1 + n(arms), 1 + n(arms) + n(upper)]
}

fn shift(n: &mut Node, by: u16) {
    n.id += by;
    for c in &mut n.children {
        shift(c, by);
    }
}

/// A tree pilot's nodes, and the ids each rule took.
pub fn tree_nodes(rules: &[Rule], reaction: u32) -> (Node, Vec<RuleIds>) {
    let mut ids = Ids(0);
    let root = ids.next();
    let mut kids = Vec::new();
    let mut map = Vec::new();
    for r in rules {
        let seq = if r.when.is_empty() { None } else { Some(ids.next()) };
        let conds: Vec<Node> = r
            .when
            .iter()
            .map(|c| {
                let (l, icon) = cond_label(c);
                leaf(&mut ids, Kind::Condition, l, icon)
            })
            .collect();
        let act = if r.act == "search" {
            search_node(&mut ids, TREE_SEARCH.0, reaction, TREE_SEARCH.1)
        } else {
            leaf(&mut ids, Kind::Action, Label::of(&format!("tree.act.{}", r.act)), &r.act)
        };
        map.push(RuleIds { seq, conds: conds.iter().map(|n| n.id).collect(), act: act.id });
        let mut node = match seq {
            Some(s) => {
                let mut children = conds;
                children.push(act);
                branch(s, Kind::Sequence, Label::of("tree.kind.sequence"), "sequence", children)
            }
            None => act,
        };
        node.interrupt = r.interrupt;
        kids.push(node);
    }
    (branch(root, Kind::Selector, Label::of("tree.kind.selector"), "selector", kids), map)
}

/// Every node, depth first.
pub fn walk(n: &Node) -> Vec<&Node> {
    let mut out = vec![n];
    for c in &n.children {
        out.extend(walk(c));
    }
    out
}
