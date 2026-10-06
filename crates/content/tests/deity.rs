//! The local deity and four arms (Sam, 2026-10-06): "a final boss that
//! requires you to have beaten all the fights in the last layer ... it has
//! 4 arms, and thus two weapons ... 1 behavior tree per set of arms, and one
//! behavior tree governing the legs ... after you defeat them you unlock the
//! cursed blade, which is the longsword, and you unlock 4 armed mode, where
//! your elbow controls are instead shoulder controls for your other arms,
//! and the elbows are free moving with no explicit control".

use content::road::{road, Best, Req};
use sim::body::{Motor, Seat};
use sim::world::Owner;
use sim::{Input, World};

/// One fighter alone with `body`, the plain sword in each pair of hands.
fn alone_with(body: sim::body::BodyDef) -> World {
    let mut s = content::setup::alone(1, sim::balance::DEFAULT_TUNING);
    s.bodies.push(body);
    s.seats[0] = Some(Seat::at((s.bodies.len() - 1) as u8, sim::balance::START_X));
    World::new(s)
}

fn tip(w: &World, k: usize) -> sim::fx::V2 {
    w.particles[w.swords[k].tip as usize].p
}

#[test]
fn the_local_deity_asks_for_every_fight_in_the_row_above_and_sits_alone_below_it() {
    let road = road();
    let last = content::road::last().expect("the road has a final fight");
    assert_eq!(last.id, "local_deity");
    assert!(last.four_arms);
    let above: Vec<&str> = road.iter().filter(|s| s.level() + 1 == last.level()).map(|s| s.id.as_str()).collect();
    let asked: Vec<&str> = last.requires.iter().map(|r| r.stop()).collect();
    assert!(!above.is_empty());
    for id in &above {
        assert!(asked.contains(id), "the local deity does not ask for the {id}");
    }
    assert_eq!(asked.len(), above.len(), "the local deity asks for a fight outside the row above");
    // Below it, the secret fight alone (crates/content/tests/guardian.rs).
    assert!(road.iter().all(|s| s.level() < last.level() || s.id == last.id || s.secret), "a fight sits in the final row or below it");
}

#[test]
fn each_pair_of_the_deitys_arms_turns_its_own_weapon() {
    let w = alone_with(content::body::four_armed(false));
    assert_eq!(w.swords_of(0).len(), 2, "four arms hold two weapons");
    assert!(w.held(0) && w.held(1));
    let pts = |k: usize| w.swords[k].points.clone();
    for (keys, mine, other) in [
        (Input::SHOULDER_UP, 0, 1),
        (Input::ELBOW_IN, 0, 1),
        (Input::SHOULDER2_UP, 1, 0),
        (Input::ELBOW2_IN, 1, 0),
    ] {
        let plan = w.motor_plan(0, Input(keys));
        assert_eq!(plan.len(), 1, "keys {keys:015b} ran {} motors", plan.len());
        let set = &plan[0].3;
        assert!(pts(mine).iter().all(|p| set.contains(p)), "keys {keys:015b} did not carry their own weapon");
        assert!(!pts(other).iter().any(|p| set.contains(p)), "keys {keys:015b} carried the other weapon");
    }
    // And the second pair's keys swing its weapon.
    let mut w = w;
    let before = tip(&w, 1);
    for _ in 0..20 {
        w.step([Input(Input::SHOULDER2_UP), Input::NONE]);
    }
    assert!((tip(&w, 1) - before).len().trunc() > 40, "the second pair's shoulder did not swing its weapon");
}

#[test]
fn the_players_elbow_keys_turn_the_second_pair_at_the_shoulder_and_bend_no_elbow() {
    let w = alone_with(content::body::four_armed(true));
    let f = w.fighters[0].as_ref().unwrap();
    let def = &w.setup.bodies[f.body as usize];
    let shoulder = f.base + def.roles.shoulder.unwrap() as u16;
    let second: Vec<u16> = w.swords[1].points.clone();
    for keys in [Input::ELBOW_IN, Input::ELBOW_OUT] {
        let plan = w.motor_plan(0, Input(keys));
        assert_eq!(plan.len(), 1, "{keys:b} ran {} motors", plan.len());
        let (motor, _, pivot, set) = &plan[0];
        assert_eq!((*motor, *pivot), (Motor::Shoulder2, shoulder), "{keys:b} did not turn the second pair at the shoulder");
        assert!(second.iter().all(|p| set.contains(p)), "{keys:b} did not carry the second weapon");
    }
    // No key bends an elbow: every key a player has drives a shoulder or
    // nothing.
    for b in 0..=Input::THROW | (Input::THROW - 1) {
        for (motor, ..) in w.motor_plan(0, Input(b)) {
            assert!(matches!(motor, Motor::Shoulder | Motor::Shoulder2), "input {b:011b} drove {motor:?}");
        }
    }
}

#[test]
fn on_a_four_armed_body_only_arm_bits_drive_a_motor() {
    for body in [content::body::four_armed(false), content::body::four_armed(true)] {
        let w = alone_with(body);
        let arms = Input::ARMS.iter().chain(Input::ARMS2.iter()).fold(0, |a, b| a | b);
        for b in 0..=0x7FFFu16 {
            let plan = w.motor_plan(0, Input(b));
            if b & arms == 0 {
                assert!(plan.is_empty(), "input {b:015b} holds no arm bit and drove {plan:?}");
            }
            for (_, _, _, set) in plan {
                assert!(set.iter().all(|&k| !w.particles[k as usize].foot), "input {b:015b} drove a foot");
                assert!(set.iter().all(|&k| matches!(w.particles[k as usize].owner, Owner::Body(0) | Owner::Sword(0))));
            }
        }
    }
}

#[test]
fn a_throw_lets_go_of_both_weapons() {
    let mut w = alone_with(content::body::four_armed(true));
    w.step([Input(Input::THROW), Input::NONE]);
    assert!(!w.held(0) && !w.held(1) && w.swords[0].flying && w.swords[1].flying);
}

#[test]
fn beating_the_local_deity_wins_the_cursed_blade_and_four_arms() {
    use content::save::{decode, encode, fresh};
    let mut s = fresh();
    for st in road().into_iter().filter(|st| st.id != "local_deity") {
        s.road.best.insert(st.id, Best::won(0, 1, "sword"));
    }
    s.four_arms = true;
    s.weapon = "longsword".into();
    let back = decode(&encode(&s)).unwrap();
    assert!(!back.four_arms && back.weapon == "sword", "four arms or the cursed blade came before the local deity was beaten");
    s.road.best.insert("local_deity".into(), Best::won(0, 1, "sword"));
    let back = decode(&encode(&s)).unwrap();
    assert!(back.four_arms && back.weapon == "longsword");
    assert_eq!(content::weapons::weapon("longsword").unwrap().unlock, Some(Req::Beat("local_deity".into())));
    // And the yard and the road give a four-armed player two weapons.
    let w = World::new(content::setup::road_with(1, sim::balance::DEFAULT_TUNING, "thresher", "longsword", true));
    assert_eq!(w.swords_of(0).len(), 2);
    assert_eq!(w.swords_of(1).len(), 1);
}

#[test]
fn the_deity_runs_three_trees_one_for_each_pair_of_arms_and_one_for_the_legs() {
    use pilot::view::{describe, walk, Kind};
    let spec = content::road::pilot("local_deity");
    let pilot::Spec::Many { arms, upper, legs } = &spec else { panic!("the local deity is not three trees") };
    for t in [arms, upper, legs] {
        assert!(matches!(**t, pilot::Spec::Tree { .. }), "one of the deity's pilots is not a tree");
    }
    let tree = describe(&spec);
    assert_eq!(tree.kind, Kind::Parallel);
    let names: Vec<&str> = tree.children.iter().map(|c| c.label.key.as_str()).collect();
    assert_eq!(names, ["tree.many.arms", "tree.many.upper", "tree.many.legs"]);
    let ids: Vec<u16> = walk(&tree).iter().map(|n| n.id).collect();
    assert_eq!(ids, (0..ids.len() as u16).collect::<Vec<_>>(), "the nodes are not numbered depth first");
    // Over a match against the yardstick: the deity presses both pairs'
    // keys and its legs', and what it lights is in its tree.
    let mut w = World::new(content::setup::road(3, sim::balance::DEFAULT_TUNING, "local_deity"));
    assert_eq!(w.swords_of(1).len(), 2);
    let mut ps = content::road::lineup(&content::road::pilot("yardstick"), "local_deity");
    let mut last = [Input::NONE; sim::body::SEATS];
    let (mut a1, mut a2, mut legs_pressed) = (false, false, false);
    while w.tick < 900 {
        let mut i = [Input::NONE; sim::body::SEATS];
        for (k, p) in ps.iter_mut().enumerate() {
            p.observe(last);
            i[k] = p.input(&w, k);
        }
        a1 |= Input::ARMS.iter().any(|&b| i[1].has(b));
        a2 |= Input::ARMS2.iter().any(|&b| i[1].has(b));
        legs_pressed |= i[1].has(Input::STEP_LEFT) || i[1].has(Input::STEP_RIGHT);
        for id in ps[1].trace().active {
            assert!((id as usize) < ids.len(), "the deity lit node {id}, which its tree does not have");
        }
        w.step_all(i);
        last = i;
    }
    assert!(a1 && a2 && legs_pressed, "lower arms {a1}, upper arms {a2}, legs {legs_pressed}");
}
