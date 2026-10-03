//! Who can cut what, and blades that are solid against each other (D10, D11;
//! PLAN.md §8 Q14, Q15).

use sim::fight::Event;
use sim::fx::V2;
use sim::rng::Rng;
use sim::{Input, World};

fn versus(seed: u64) -> World {
    World::new(content::setup::versus(seed, sim::balance::DEFAULT_TUNING))
}

fn hand_parts(w: &World, seat: u8) -> Vec<usize> {
    let defs = &w.setup.bodies[0].parts;
    (0..w.parts.len()).filter(|&i| w.parts[i].fighter == seat && defs[w.parts[i].def as usize].hand).collect()
}

#[test]
fn the_hands_on_the_hilt_are_safe_from_their_own_blade() {
    let w = versus(1);
    for seat in 0..2u8 {
        let hands = hand_parts(&w, seat);
        assert_eq!(hands.len(), 2, "two hands per fighter");
        for &h in &hands {
            assert!(!w.may_cut(seat as usize, h), "seat {seat}'s own sword may cut its hand {h}");
        }
    }
    // And over many runs of random input, no fighter's blade ever cuts a hand
    // of that fighter's that is holding it.
    for seed in 0..40u64 {
        let mut w = versus(seed);
        let mut r = Rng::new(seed + 3);
        for _ in 0..600 {
            w.step([Input(r.below(64) as u16), Input(r.below(64) as u16)]);
            for e in &w.events {
                if let Event::Cut { seat, by, part, spilled: true, .. } = *e {
                    let is_hand = w.setup.bodies[0].parts[part as usize].hand;
                    assert!(!(is_hand && seat == by), "seed {seed}: a fighter's own blade cut its hand {part}");
                }
            }
        }
    }
}

#[test]
fn an_opponents_blade_cuts_a_hand() {
    let w = versus(1);
    for &h in &hand_parts(&w, 1) {
        assert!(w.may_cut(0, h), "the left sword may not cut the right fighter's hand {h}");
    }
}

#[test]
fn two_blades_never_pass_through_each_other() {
    // An independent check, not the swept test the build uses: at the end of
    // every tick the two swords' center lines do not cross.
    let crosses = |a0: V2, a1: V2, b0: V2, b1: V2| {
        let side = |p: V2, q0: V2, q1: V2| (q1 - q0).cross_raw(p - q0).signum();
        let (d1, d2) = (side(b0, a0, a1), side(b1, a0, a1));
        let (d3, d4) = (side(a0, b0, b1), side(a1, b0, b1));
        d1 * d2 < 0 && d3 * d4 < 0
    };
    let mut clashes = 0;
    for seed in 0..60u64 {
        let mut w = versus(seed);
        let mut r = Rng::new(seed + 70);
        let mut held = [Input::NONE; 2];
        for t in 0..900 {
            if t % 12 == 0 {
                // Both fighters close in and swing.
                let toward = [Input::STEP_RIGHT, Input::STEP_LEFT];
                held = [0, 1].map(|s| Input((r.below(16) as u16) | if r.below(3) > 0 { toward[s] } else { 0 }));
            }
            w.step(held);
            clashes += w.events.iter().filter(|e| matches!(e, Event::Clash { .. })).count();
            let s = &w.swords;
            if s.len() == 2 {
                let p = |i: u16| w.particles[i as usize].p;
                assert!(
                    !crosses(p(s[0].butt), p(s[0].tip), p(s[1].butt), p(s[1].tip)),
                    "seed {seed}, tick {t}: the two swords cross"
                );
            }
        }
    }
    assert!(clashes > 20, "only {clashes} clashes in 60 runs: the blades hardly met, so this test proves little");
}
