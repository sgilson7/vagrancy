//! The one generator.
//!
//! There is exactly one of these and it lives in `World`. Nothing else in
//! `sim` may produce a random number, and `sim` may not reach for `rand`,
//! because the seed is the only thing that decides what a run contains and
//! every peer has to draw from it in the same order.
//!
//! xorshift64*, ported from Floodline's `crates/sim/src/rng.rs`, which took it
//! from gear-master. Here it decides only the spawn jitter of a round (D16).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // A zero state would stick at zero forever.
        Rng { state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed } }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        // The high bits of a xorshift* are the well-mixed ones.
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `0..n`. Returns 0 when `n` is 0, rather than dividing by it.
    ///
    /// Modulo, not rejection sampling: the bias is one part in 2^64/n, which
    /// for any n this game asks for is far below anything a player could
    /// notice, and rejection sampling would make the number of draws depend on
    /// the values drawn. That is a bad trade here — a variable number of draws
    /// is exactly the shape of bug that makes two peers diverge if one of them
    /// ever gets a slightly different n.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as u32
    }

    /// Uniform in `lo..=hi`. Swaps them rather than panicking if they arrive
    /// the wrong way round.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
        lo + self.below((hi - lo + 1) as u32) as i32
    }

    /// One in `n`.
    pub fn chance(&mut self, n: u32) -> bool {
        self.below(n) == 0
    }

    /// Fisher-Yates, so drawing without replacement is "shuffle and take".
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = self.below(i as u32 + 1) as usize;
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Floodline's golden values for `Rng::new(1)`, which were computed by a
    /// separate implementation of xorshift64* and not copied out of a run of
    /// the code they test. Changing the generator is a decision.
    #[test]
    fn the_sequence_is_pinned() {
        let mut r = Rng::new(1);
        let got: Vec<u64> = (0..4).map(|_| r.next_u64()).collect();
        assert_eq!(got, vec![5180492295206395165, 12380297144915551517, 13389498078930870103, 5599127315341312413]);
    }

    #[test]
    fn range_is_inclusive_and_takes_one_draw() {
        let mut a = Rng::new(9);
        let mut b = Rng::new(9);
        for _ in 0..1000 {
            let v = a.range(-3, 3);
            assert!((-3..=3).contains(&v));
            b.next_u64();
        }
        assert_eq!(a.next_u64(), b.next_u64(), "one draw per call, whatever was drawn");
    }
}
