//! The tutorial follows the knowledge-component graph (Sam, 2026-10-04;
//! analysis/kc/RESULTS.md): what it teaches, in what order, and whether each
//! drill's goal can be met with the keys.

use content::copy::copy;
use content::tutorial::{missions, Goal, Mission, Tracker, KC_GRAPH_JSON};
use serde_json::Value;
use sim::{Input, World};
use std::collections::{BTreeMap, BTreeSet};

fn graph() -> Value {
    serde_json::from_str(KC_GRAPH_JSON).unwrap()
}

/// Each mission's id, and every mission it requires, however indirectly.
fn before(ms: &[Mission]) -> BTreeMap<String, BTreeSet<String>> {
    let by: BTreeMap<&str, &Mission> = ms.iter().map(|m| (m.id.as_str(), m)).collect();
    let mut out = BTreeMap::new();
    for m in ms {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<&str> = m.requires.iter().map(String::as_str).collect();
        while let Some(r) = stack.pop() {
            assert!(r != m.id, "{} requires itself", m.id);
            if seen.insert(r.to_string()) {
                let x = by.get(r).unwrap_or_else(|| panic!("{} requires {r}, which is not a mission", m.id));
                stack.extend(x.requires.iter().map(String::as_str));
            }
            assert!(seen.len() <= ms.len(), "the missions' requirements go round in a circle through {}", m.id);
        }
        out.insert(m.id.clone(), seen);
    }
    out
}

#[test]
fn every_component_is_taught_and_after_each_one_it_builds_on() {
    let g = graph();
    let ms = missions();
    let pre = before(&ms);
    let kcs: Vec<&str> = g["nodes"].as_array().unwrap().iter().map(|n| n["id"].as_str().unwrap()).collect();
    let edges: Vec<(&str, &str)> = g["edges"].as_array().unwrap().iter().map(|e| (e["src"].as_str().unwrap(), e["dst"].as_str().unwrap())).collect();
    let taught_by = |k: &str| ms.iter().filter(|m| m.teaches.iter().any(|t| t == k)).map(|m| m.id.clone()).collect::<Vec<_>>();
    for k in &kcs {
        assert!(!taught_by(k).is_empty(), "no mission teaches {k}");
    }
    // Listed after what it requires (the map's rows are worked out in one
    // pass over the list), and named apart from the others.
    let mut names = Vec::new();
    for (i, m) in ms.iter().enumerate() {
        for r in &m.requires {
            assert!(ms[..i].iter().any(|x| &x.id == r), "{} is listed before {r}, which it requires", m.id);
        }
        let name = match (&m.name, m.tasks.len()) {
            (Some(k), _) => {
                let found = k.split('.').try_fold(copy(), |v, p| v.get(p).cloned());
                assert!(found.is_some_and(|v| v.is_string()), "{} is named by {k}, which is not in the copy", m.id);
                k.clone()
            }
            (None, 2) => format!("{}+{}", m.tasks[0].at, m.tasks[1].at),
            (None, _) => format!("kc.{}.name", m.teaches[0]),
        };
        assert!(!names.contains(&name), "two missions are named {name}");
        names.push(name);
    }
    let roots: Vec<&str> = ms.iter().filter(|m| m.requires.is_empty()).map(|m| m.id.as_str()).collect();
    assert_eq!(roots, ["m_shoulder"], "the map has one mission to start from");
    for m in &ms {
        for t in &m.teaches {
            assert!(kcs.contains(&t.as_str()), "{} teaches {t}, which is not in the graph", m.id);
            // Whatever it builds on is taught first: by a mission this one
            // requires, or by this one.
            // A strategy edge says which knowledge a strategy picks; the
            // strategy is learned in the comparisons, after it.
            for (src, _) in edges.iter().filter(|(s, d)| d == t && !s.starts_with('S')) {
                let here = m.teaches.iter().any(|x| x == src);
                let earlier = taught_by(src).iter().any(|x| pre[&m.id].contains(x));
                assert!(here || earlier, "{} teaches {t}, which builds on {src}, and no mission before it teaches {src}", m.id);
            }
        }
        for b in &m.builds_on {
            let (src, dst) = b.split_once('_').unwrap();
            assert!(edges.contains(&(src, dst)), "{} builds on {b}, which is not an edge", m.id);
            assert!(m.teaches.iter().any(|t| t == dst), "{} builds on {b} and does not teach {dst}", m.id);
            assert!(copy()["kc_edge"][b].is_string(), "kc_edge.{b} has no sentence");
        }
    }
}

#[test]
fn each_task_makes_concrete_what_its_mission_teaches() {
    // A drill repeats a practice step and a fight to the finish is the road's
    // fight: each must be an instance that requires the components taught.
    // A comparison needs each component in one of its two fights. Strategy
    // components are about the comparison, not either fight.
    let g = graph();
    let inst = g["instances"].as_object().unwrap();
    let pairs: Vec<(String, String)> = g["comparisons"].as_array().unwrap().iter().map(|c| (c["a"].as_str().unwrap().to_string(), c["b"].as_str().unwrap().to_string())).collect();
    for m in missions() {
        let instance = |t: &content::tutorial::Task| if t.goal == Goal::Win || t.at == "yard" { t.instance() } else { None };
        let found: Vec<Vec<String>> = m
            .tasks
            .iter()
            .filter_map(|t| instance(t))
            .map(|i| inst.get(&i).unwrap_or_else(|| panic!("{}: {i} is not an instance in the graph", m.id)).as_array().unwrap().iter().map(|k| k.as_str().unwrap().to_string()).collect())
            .collect();
        for k in m.teaches.iter().filter(|k| !k.starts_with('S')) {
            let seen = found.iter().filter(|kcs| kcs.contains(k)).count();
            if m.tasks.len() == 1 && !found.is_empty() {
                assert_eq!(seen, 1, "{}: its {} does not require {k}", m.id, m.tasks[0].at);
            } else if m.tasks.len() == 2 {
                assert!(seen >= 1, "{}: neither fight requires {k}", m.id);
            }
        }
        if m.tasks.len() == 2 {
            let (a, b) = (m.tasks[0].instance().unwrap(), m.tasks[1].instance().unwrap());
            assert!(pairs.contains(&(a.clone(), b.clone())), "{}: {a} and {b} are not a comparison in the graph", m.id);
            assert!(copy()["kc_compare"][&m.id].is_string(), "kc_compare.{} has no sentence", m.id);
        }
    }
}

#[test]
fn the_tutorial_practices_every_edge_the_game_leaves_unpracticed() {
    // The analysis (analysis/kc/RESULTS.md) found edges no drill or fight
    // asks a player to cross. A mission that builds on such an edge and
    // teaches its end is an item that requires both.
    let g = graph();
    let inst: Vec<Vec<&str>> = g["instances"].as_object().unwrap().values().map(|v| v.as_array().unwrap().iter().map(|k| k.as_str().unwrap()).collect()).collect();
    let ms = missions();
    for e in g["edges"].as_array().unwrap() {
        let (s, d) = (e["src"].as_str().unwrap(), e["dst"].as_str().unwrap());
        if inst.iter().any(|kcs| kcs.contains(&s) && kcs.contains(&d)) {
            continue;
        }
        let key = format!("{s}_{d}");
        assert!(ms.iter().any(|m| m.builds_on.contains(&key)), "no item practices {key}, in the game or in the tutorial");
    }
}

/// Play the yard with a plan of held keys, `(ticks, keys)`, and say whether
/// the goal was met.
fn yard(goal: Goal, plan: &[(u32, u16)]) -> bool {
    yard_on(goal, plan, content::maps::FLAT)
}

/// The same in a yard on the ground `map` (data/maps.json).
fn yard_on(goal: Goal, plan: &[(u32, u16)], map: &str) -> bool {
    let mut w = World::new(content::maps::on(content::setup::practice(1, sim::balance::DEFAULT_TUNING), map));
    let mut t = Tracker::new(goal);
    for &(n, keys) in plan {
        for _ in 0..n {
            let i = Input(keys);
            w.step([i, Input::NONE]);
            t.observe(&w, i);
        }
    }
    t.met()
}

#[test]
fn every_goal_in_the_yard_can_be_met_with_the_keys() {
    use Input as I;
    let ms = missions();
    let goal = |id: &str| ms.iter().find(|m| m.id == id).unwrap().tasks[0].goal.clone();
    let plant = [(30, I::ELBOW_IN), (30, I::SHOULDER_DOWN | I::ELBOW_IN), (60, I::SHOULDER_DOWN | I::ELBOW_OUT)];
    let cases: Vec<(&str, Vec<(u32, u16)>)> = vec![
        ("m_shoulder", vec![(200, I::SHOULDER_UP)]),
        ("m_elbow", vec![(20, I::ELBOW_OUT | I::SHOULDER_UP), (100, I::ELBOW_OUT)]),
        ("m_move", vec![(120, I::STEP_RIGHT)]),
        ("m_jump", vec![(1, I::JUMP), (20, 0)]),
        ("m_air_jump", vec![(1, I::JUMP), (12, 0), (1, I::JUMP), (10, 0)]),
        ("m_dodge", vec![(1, I::DODGE), (5, 0)]),
        ("m_roll", vec![(6, I::STEP_RIGHT | I::DODGE), (5, 0)]),
        ("m_air_dodge", vec![(1, I::JUMP), (10, 0), (1, I::DODGE), (5, 0)]),
        ("m_wait", vec![(1, I::DODGE), (80, 0), (1, I::DODGE), (5, 0)]),
        ("m_swing", vec![(300, I::SHOULDER_UP | I::ELBOW_OUT)]),
        ("m_plant", plant.to_vec()),
        // Swinging first, then lowering and straightening together.
        ("c_swing_plant", vec![(120, I::SHOULDER_UP | I::ELBOW_OUT), (60, I::SHOULDER_DOWN | I::ELBOW_OUT)]),
        // Swinging up from where the yard starts, letting go on the upswing.
        ("m_throw", vec![(40, I::SHOULDER_UP | I::ELBOW_OUT), (1, I::SHOULDER_UP | I::ELBOW_OUT | I::THROW), (120, 0)]),
        // Under the ledge where the yard starts: jump, then stand.
        ("m_ledge", vec![(30, 0), (1, I::JUMP), (15, 0), (1, I::STAND), (60, 0)]),
        // Walking into the post with the blade in front.
        ("m_cut", vec![(60, I::STEP_RIGHT), (20, I::STEP_RIGHT | I::SHOULDER_DOWN | I::ELBOW_IN)]),
    ];
    let map = |id: &str| ms.iter().find(|m| m.id == id).unwrap().tasks[0].map.clone().unwrap_or(content::maps::FLAT.to_string());
    let mut failed = Vec::new();
    for (id, plan) in &cases {
        if !yard_on(goal(id), plan, &map(id)) {
            failed.push(*id);
        }
    }
    assert!(failed.is_empty(), "no plan of keys met the goals of {failed:?}");
    // And each goal is not met by standing still, nor a plant by jumping.
    for (id, _) in &cases {
        assert!(!yard(goal(id), &[(300, 0)]), "standing still met {id}'s goal");
    }
    assert!(!yard(goal("m_plant"), &[(1, I::JUMP), (60, I::SHOULDER_DOWN | I::ELBOW_OUT)]), "a jump counted as a plant");
    // A cut with the sword in hand is not a throw's cut.
    assert!(!yard(goal("m_throw"), &[(60, I::STEP_RIGHT), (20, I::STEP_RIGHT | I::SHOULDER_DOWN | I::ELBOW_IN)]), "a held sword's cut counted as a thrown one");
    // Every goal in the yard has a case here.
    for m in &ms {
        if m.tasks.iter().any(|t| t.at == "yard") {
            assert!(cases.iter().any(|(id, _)| *id == m.id), "{} has no plan of keys here", m.id);
        }
    }
}

#[test]
fn the_goals_on_the_road_short_of_a_win_are_met_by_the_yardstick() {
    // A spilling cut on the scarecrow, and blades meeting at the
    // gatekeeper: the yardstick pilot playing the player's seat meets each
    // within a minute. (It does not attack a post, so the yard's cut has a
    // plan of keys above instead.)
    let ms = missions();
    for m in ms.iter().filter(|m| m.tasks.iter().any(|t| t.at != "yard" && matches!(t.goal, Goal::SpillCut | Goal::Clashes(_)))) {
        let t = &m.tasks[0];
        let setup = if t.at == "yard" { content::setup::practice(1, sim::balance::DEFAULT_TUNING) } else { content::setup::road(1, sim::balance::DEFAULT_TUNING, &t.at) };
        let mut w = World::new(setup);
        let mut me = pilot::build(&content::road::pilot("yardstick"));
        let mut them = (t.at != "yard").then(|| pilot::build(&content::road::pilot(&t.at)));
        let mut tr = Tracker::new(t.goal.clone());
        for _ in 0..3600 {
            let i = me.input(&w, 0);
            let o = them.as_mut().map(|p| p.input(&w, 1)).unwrap_or(Input::NONE);
            w.step([i, o]);
            tr.observe(&w, i);
            if tr.met() {
                break;
            }
        }
        assert!(tr.met(), "{}: the yardstick did not meet {:?} at {} in a minute", m.id, t.goal, t.at);
    }
}

#[test]
fn every_yard_goal_is_met_with_loose_timing() {
    // Sam, 2026-10-06: "some tutorial missions ... feel impossible, like
    // swing then plant, getting up to 400cm". A goal met only by exact
    // timing is one a person rarely meets. Each yard goal with a number,
    // played with every part of its plan 70 to 130 percent as long, forty
    // times: most of those must meet it. (At 400 cm, swing then plant met
    // none of forty; the plant at 150 met about a quarter; the reach at 145
    // about half.)
    use Input as I;
    let ms = missions();
    let goal = |id: &str| ms.iter().find(|m| m.id == id).unwrap().tasks[0].goal.clone();
    let cases: Vec<(&str, Vec<(u32, u16)>, u32)> = vec![
        ("m_elbow", vec![(20, I::ELBOW_OUT | I::SHOULDER_UP), (100, I::ELBOW_OUT)], 30),
        ("m_swing", vec![(300, I::SHOULDER_UP | I::ELBOW_OUT)], 30),
        ("m_plant", vec![(30, I::ELBOW_IN), (30, I::SHOULDER_DOWN | I::ELBOW_IN), (60, I::SHOULDER_DOWN | I::ELBOW_OUT)], 20),
        ("c_swing_plant", vec![(120, I::SHOULDER_UP | I::ELBOW_OUT), (60, I::SHOULDER_DOWN | I::ELBOW_OUT)], 30),
        ("m_move", vec![(120, I::STEP_RIGHT)], 30),
    ];
    let mut seed = 12345u64;
    for (id, plan, need) in &cases {
        let mut met = 0;
        for _ in 0..40 {
            let p: Vec<(u32, u16)> = plan
                .iter()
                .map(|&(n, k)| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    let f = 70 + (seed >> 33) % 61;
                    ((n as u64 * f / 100).max(1) as u32, k)
                })
                .collect();
            met += yard(goal(id), &p) as u32;
        }
        assert!(met >= *need, "{id}: loose timing met its goal {met} times of 40, fewer than {need}");
    }
}
