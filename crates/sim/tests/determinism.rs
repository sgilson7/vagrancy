//! The same inputs give the same world, every time and on every machine
//! (D3, D4). The native half is here; the browser half is the gate's
//! "native checksum equals wasm checksum" check.

use sim::rng::Rng;
use sim::fight::Event;
use sim::{Input, World};

fn versus(seed: u64) -> World {
    World::new(content::setup::versus(seed, sim::balance::DEFAULT_TUNING))
}

fn random_input(r: &mut Rng) -> Input {
    Input(r.below(64) as u8)
}

#[test]
fn two_worlds_fed_the_same_inputs_agree_for_ten_thousand_ticks() {
    for seed in [1u64, 42, 0xC0FFEE] {
        let (mut a, mut b) = (versus(seed), versus(seed));
        let mut r = Rng::new(seed ^ 0x55);
        for t in 0..10_000 {
            let i = [random_input(&mut r), random_input(&mut r)];
            a.step(i);
            b.step(i);
            if t % 500 == 0 {
                assert_eq!(a.checksum(), b.checksum(), "seed {seed}: the worlds parted by tick {t}");
            }
        }
        assert_eq!(a, b, "seed {seed}: equal checksums but different worlds");
    }
}

#[test]
fn the_checksum_would_actually_catch_a_divergence() {
    // After Floodline's determinism.rs:104. A checksum that cannot tell two
    // worlds apart would let every test above pass.
    let a = versus(3);
    let mut b = a.clone();
    assert_eq!(a.checksum(), b.checksum());
    b.particles[5].p.x.0 += 1;
    assert_ne!(a.checksum(), b.checksum(), "one raw unit in one particle must change the checksum");
    let mut c = a.clone();
    c.fighters[1].as_mut().unwrap().ink -= 1;
    assert_ne!(a.checksum(), c.checksum(), "and so must one drop of ink");
}

#[test]
fn a_mirrored_world_stays_a_mirror_until_the_fighters_touch() {
    // D3: products round toward zero, which is odd-symmetric, so the right
    // seat fed the mirror of the left seat's input stays its exact mirror,
    // self-cuts and a drawn round included. It holds until the fighters touch
    // each other: blade against blade and cut against cut are resolved in seat
    // order, which is fair to within a raw unit or two but not an exact mirror
    // (SECOND-ORDER-M3). So each run stops at the first event that couples
    // them, and the runs together must cover enough ticks to mean something.
    let mut checked = 0;
    for seed in 0..20u64 {
        let mut w = versus(seed);
        let mut r = Rng::new(77 + seed);
        let n = w.fighters[0].as_ref().unwrap().n as usize;
        for t in 0..2_000 {
            let i = random_input(&mut r);
            w.step([i, i.mirror()]);
            let coupled = w.events.iter().any(|e| match e {
                Event::Clash { .. } => true,
                Event::Cut { seat, by, .. } => seat != by,
                _ => false,
            });
            if coupled {
                break;
            }
            for k in 0..n {
                let (l, rt) = (w.particles[k].p, w.particles[n + k].p);
                assert_eq!(rt, l.mirror(), "seed {seed}, tick {t}: point {k} of the right fighter is not the left one's mirror");
            }
            checked += 1;
        }
    }
    assert!(checked >= 5_000, "only {checked} mirrored ticks were checked before the fighters touched");
}

#[test]
fn different_seeds_give_different_rounds() {
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..20u64 {
        seen.insert(versus(seed).checksum());
    }
    assert!(seen.len() > 10, "the spawn jitter should make most seeds differ, got {} of 20", seen.len());
}
