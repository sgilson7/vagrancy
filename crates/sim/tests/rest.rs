//! Sam, 2026-10-07: "after someone dies there needs to be like a samurai
//! pause moment to see who died". A round's end holds for REST_TICKS before
//! the next starts, however soon each seat says it is ready.

use sim::body::{Cause, SEATS};
use sim::fight::{Phase, RoundResult};
use sim::{Input, World};

#[test]
fn the_next_round_waits_out_the_rest_though_each_seat_is_ready_at_once() {
    let mut w = World::new(content::setup::versus(1, sim::balance::DEFAULT_TUNING));
    let result = RoundResult { loser: Some(1), seat: 1, cause: Cause::Heart, part: 0, by: 0, thrown: false };
    w.phase = Phase::RoundOver { result, ready: [false; SEATS] };
    let ready = [Input(Input::READY); SEATS];
    let mut waited = 0;
    while !matches!(w.phase, Phase::Fight) {
        w.step_all(ready);
        waited += 1;
        assert!(waited <= sim::balance::REST_TICKS + 2, "the round did not start after the rest");
    }
    assert!(waited >= sim::balance::REST_TICKS, "the next round started {waited} ticks after the last, inside the rest");
    assert_eq!(w.round, 2);
}
