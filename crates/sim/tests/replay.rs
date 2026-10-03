//! Replay v1 round-trips, refuses what it cannot play, and the golden replay
//! ends where it always has (D15; PLANNING-BRIEF 0.3 item 8).

use sim::replay::{self, Recording, ReplayError};
use sim::rng::Rng;
use sim::{Input, SIM_VERSION};

fn recorded(ticks: u32) -> Recording {
    let mut rec = Recording::new(content::setup::versus(5, sim::balance::DEFAULT_TUNING));
    let mut r = Rng::new(11);
    for _ in 0..ticks {
        rec.step([Input(r.below(64) as u8), Input(r.below(64) as u8)]);
    }
    rec
}

#[test]
fn a_replay_played_back_ends_on_the_recorded_checksum() {
    let rec = recorded(1_000);
    let r = replay::load(&rec.bytes()).expect("its own replay loads");
    let w = replay::verify(&r).expect("and plays back to the recorded end");
    assert_eq!(w.checksum(), rec.world.checksum());
    assert_eq!(w.tick, 1_000);
}

#[test]
fn a_replay_from_another_sim_version_is_refused_with_a_sentence() {
    let mut r = recorded(120).replay();
    r.sim_version = SIM_VERSION + 1;
    let bytes = postcard::to_allocvec(&r).unwrap();
    assert_eq!(replay::load(&bytes), Err(ReplayError::Sim { theirs: SIM_VERSION + 1, ours: SIM_VERSION }));
}

#[test]
fn a_damaged_replay_is_refused_rather_than_half_loaded() {
    let bytes = recorded(300).bytes();
    for cut in [bytes.len() - 1, bytes.len() / 2, 40] {
        assert_eq!(replay::load(&bytes[..cut]), Err(ReplayError::Damaged), "cut at {cut}");
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert_eq!(replay::load(&extra), Err(ReplayError::Damaged), "a trailing byte");
    let mut r = recorded(300).replay();
    r.ticks += 1;
    assert_eq!(replay::load(&postcard::to_allocvec(&r).unwrap()), Err(ReplayError::Damaged), "a tick count that disagrees");
    assert_eq!(replay::load(b"this is not a replay at all"), Err(ReplayError::Format));
    assert_eq!(replay::load(&[]), Err(ReplayError::Format));
}

#[test]
fn a_replay_with_a_spare_bit_set_is_refused() {
    let mut r = recorded(60).replay();
    r.inputs[10][1] |= Input::SPARE;
    assert_eq!(replay::load(&postcard::to_allocvec(&r).unwrap()), Err(ReplayError::Damaged));
}

#[test]
fn a_replay_that_drifts_names_the_first_second_that_differs() {
    let mut r = recorded(600).replay();
    // A single flipped arm bit can change nothing (a servo already at its
    // speed limit ignores a held key), so the edit is the left seat running
    // the other way for a quarter second, which always moves it.
    // Early, while the round is still being fought: between rounds input is
    // ignored, and an edit there changes nothing.
    for pair in &mut r.inputs[30..45] {
        pair[0] = Input::STEP_LEFT;
    }
    let d = match replay::verify(&r) {
        Ok(_) => panic!("an edited replay verified"),
        Err(d) => d,
    };
    assert_eq!(d.tick, 60, "the first checkpoint after the edit at tick 30 is tick 60");
}

/// The golden replay was recorded by `lab golden` and is committed. If it
/// stops verifying, the simulation changed: bump SIM_VERSION, re-record it
/// with `cargo run -p lab -- golden`, and say so in the commit (CLAUDE.md).
#[test]
fn a_golden_replay_ends_where_it_always_has() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testing/replays/golden.replay");
    let bytes = std::fs::read(path).expect("testing/replays/golden.replay exists; run `cargo run -p lab -- golden`");
    let r = replay::load(&bytes).expect("the golden replay loads");
    assert!(r.ticks >= 1_200, "the golden replay is {} ticks long", r.ticks);
    replay::verify(&r).expect("the golden replay ends where it always has");
}
