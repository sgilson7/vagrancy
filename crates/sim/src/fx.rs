//! Fixed point, 12 fractional bits; one unit is a centimeter.
//!
//! Ported from Floodline's `crates/sim/src/fx.rs` (8 bits there). A float is
//! the one thing that can make two machines running the same code disagree,
//! so every quantity that would want to be one is an `i32` in units of
//! 1/4096 cm. The range is ±524,287 cm, and the arena is about 1,200.
//!
//! Three rules the rest of `sim` relies on (PLAN.md D3):
//!
//! * **Every multiply and divide goes through `i64`**, `Fx * i32` included.
//!   Floodline's `Mul<i32>` did not widen (PLAN.md §7 item 2).
//! * **`Fx` products and quotients round toward zero.** Rounding toward zero
//!   is odd-symmetric, `trunc(-a) == -trunc(a)`, so a world mirrored left to
//!   right stays an exact mirror and the two seats are exactly fair. This is a
//!   deviation from Floodline, where `*` floored and `/` truncated.
//! * **A right shift of a raw value floors**, as Rust's `>>` does on a
//!   signed integer. H6 pins it. `Fx` arithmetic does not use it.
//!
//! `[profile.release]` keeps overflow checks on: a panic is the same loud
//! failure everywhere, and a wrap is a silent desync.

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};
use serde::{Deserialize, Serialize};

/// How many of an `Fx`'s low bits are the fraction.
pub const FRAC_BITS: u32 = 12;
/// One centimeter.
pub const ONE: Fx = Fx(1 << FRAC_BITS);
pub const ZERO: Fx = Fx(0);

/// A fixed-point number: `raw / 4096` centimeters (or of whatever unit).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, Serialize, Deserialize)]
pub struct Fx(pub i32);

impl Fx {
    /// A whole number of units.
    pub const fn int(n: i32) -> Fx {
        Fx(n << FRAC_BITS)
    }
    /// `num / den` as a fixed-point fraction, rounded toward zero.
    pub const fn ratio(num: i64, den: i64) -> Fx {
        Fx(((num << FRAC_BITS) / den) as i32)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
    /// Toward zero, to a whole unit.
    pub const fn trunc(self) -> i32 {
        self.0 / (1 << FRAC_BITS)
    }
    pub const fn abs(self) -> Fx {
        Fx(if self.0 < 0 { -self.0 } else { self.0 })
    }
    pub const fn signum(self) -> i32 {
        if self.0 > 0 {
            1
        } else if self.0 < 0 {
            -1
        } else {
            0
        }
    }
    pub const fn min(self, o: Fx) -> Fx {
        if self.0 < o.0 { self } else { o }
    }
    pub const fn max(self, o: Fx) -> Fx {
        if self.0 > o.0 { self } else { o }
    }
    pub const fn clamp(self, lo: Fx, hi: Fx) -> Fx {
        self.max(lo).min(hi)
    }
    /// `self * num / den` in one widening step, rounded toward zero.
    pub const fn scale(self, num: i64, den: i64) -> Fx {
        Fx((self.0 as i64 * num / den) as i32)
    }
}

/// Floor of the square root, by Newton's method on integers (Floodline's,
/// widened to `u64` so that `isqrt(2^62) = 2^31` has somewhere to go: H6).
pub const fn isqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

impl Add for Fx {
    type Output = Fx;
    fn add(self, r: Fx) -> Fx {
        Fx(self.0 + r.0)
    }
}
impl Sub for Fx {
    type Output = Fx;
    fn sub(self, r: Fx) -> Fx {
        Fx(self.0 - r.0)
    }
}
impl Neg for Fx {
    type Output = Fx;
    fn neg(self) -> Fx {
        Fx(-self.0)
    }
}
impl AddAssign for Fx {
    fn add_assign(&mut self, r: Fx) {
        self.0 += r.0;
    }
}
impl SubAssign for Fx {
    fn sub_assign(&mut self, r: Fx) {
        self.0 -= r.0;
    }
}
impl Mul for Fx {
    type Output = Fx;
    /// Rounds toward zero (D3), through `i64`.
    fn mul(self, r: Fx) -> Fx {
        Fx(((self.0 as i64 * r.0 as i64) / (1i64 << FRAC_BITS)) as i32)
    }
}
impl Div for Fx {
    type Output = Fx;
    /// Rounds toward zero, through `i64`. Panics on zero, identically everywhere.
    fn div(self, r: Fx) -> Fx {
        Fx((((self.0 as i64) << FRAC_BITS) / r.0 as i64) as i32)
    }
}
impl Mul<i32> for Fx {
    type Output = Fx;
    fn mul(self, r: i32) -> Fx {
        Fx((self.0 as i64 * r as i64) as i32)
    }
}
impl Div<i32> for Fx {
    type Output = Fx;
    fn div(self, r: i32) -> Fx {
        Fx((self.0 as i64 / r as i64) as i32)
    }
}

/// A point or a displacement. `y` is up; the ground is `y = 0`; `x = 0` is
/// the middle of the arena, so a mirror is a negation.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct V2 {
    pub x: Fx,
    pub y: Fx,
}

impl V2 {
    pub const ZERO: V2 = V2 { x: ZERO, y: ZERO };
    pub const fn new(x: Fx, y: Fx) -> V2 {
        V2 { x, y }
    }
    pub const fn cm(x: i32, y: i32) -> V2 {
        V2 { x: Fx::int(x), y: Fx::int(y) }
    }
    /// Squared length in raw units squared (`i64`, no rounding).
    pub const fn len_sq_raw(self) -> i64 {
        let (x, y) = (self.x.0 as i64, self.y.0 as i64);
        x * x + y * y
    }
    pub const fn len(self) -> Fx {
        Fx(isqrt(self.len_sq_raw() as u64) as i32)
    }
    /// Dot product, as `Fx`.
    pub const fn dot(self, o: V2) -> Fx {
        Fx(((self.x.0 as i64 * o.x.0 as i64 + self.y.0 as i64 * o.y.0 as i64) / (1i64 << FRAC_BITS)) as i32)
    }
    /// The z of the cross product, as `Fx`.
    pub const fn cross(self, o: V2) -> Fx {
        Fx(((self.x.0 as i64 * o.y.0 as i64 - self.y.0 as i64 * o.x.0 as i64) / (1i64 << FRAC_BITS)) as i32)
    }
    /// Cross product without rounding, in raw units squared.
    pub const fn cross_raw(self, o: V2) -> i64 {
        self.x.0 as i64 * o.y.0 as i64 - self.y.0 as i64 * o.x.0 as i64
    }
    pub const fn dot_raw(self, o: V2) -> i64 {
        self.x.0 as i64 * o.x.0 as i64 + self.y.0 as i64 * o.y.0 as i64
    }
    /// A quarter turn counterclockwise.
    pub const fn perp(self) -> V2 {
        V2 { x: Fx(-self.y.0), y: self.x }
    }
    pub fn scale(self, num: i64, den: i64) -> V2 {
        V2 { x: self.x.scale(num, den), y: self.y.scale(num, den) }
    }
    /// This vector at length `l`; zero stays zero.
    pub fn with_len(self, l: Fx) -> V2 {
        let n = self.len().0 as i64;
        if n == 0 {
            return V2::ZERO;
        }
        self.scale(l.0 as i64, n)
    }
    /// `a + (b - a) * f`.
    pub fn lerp(a: V2, b: V2, f: Fx) -> V2 {
        a + (b - a) * f
    }
    pub const fn mirror(self) -> V2 {
        V2 { x: Fx(-self.x.0), y: self.y }
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, r: V2) -> V2 {
        V2 { x: self.x + r.x, y: self.y + r.y }
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, r: V2) -> V2 {
        V2 { x: self.x - r.x, y: self.y - r.y }
    }
}
impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        V2 { x: -self.x, y: -self.y }
    }
}
impl AddAssign for V2 {
    fn add_assign(&mut self, r: V2) {
        self.x += r.x;
        self.y += r.y;
    }
}
impl SubAssign for V2 {
    fn sub_assign(&mut self, r: V2) {
        self.x -= r.x;
        self.y -= r.y;
    }
}
impl Mul<Fx> for V2 {
    type Output = V2;
    fn mul(self, r: Fx) -> V2 {
        V2 { x: self.x * r, y: self.y * r }
    }
}
impl Mul<i32> for V2 {
    type Output = V2;
    fn mul(self, r: i32) -> V2 {
        V2 { x: self.x * r, y: self.y * r }
    }
}
impl Div<i32> for V2 {
    type Output = V2;
    fn div(self, r: i32) -> V2 {
        V2 { x: self.x / r, y: self.y / r }
    }
}

// Generated by packaging/gen-sine.py; do not edit by hand.
const SIN_DEG: [i32; 91] = [
    0, 71, 143, 214, 286, 357, 428, 499, 570, 641,
    711, 782, 852, 921, 991, 1060, 1129, 1198, 1266, 1334,
    1401, 1468, 1534, 1600, 1666, 1731, 1796, 1860, 1923, 1986,
    2048, 2110, 2171, 2231, 2290, 2349, 2408, 2465, 2522, 2578,
    2633, 2687, 2741, 2793, 2845, 2896, 2946, 2996, 3044, 3091,
    3138, 3183, 3228, 3271, 3314, 3355, 3396, 3435, 3474, 3511,
    3547, 3582, 3617, 3650, 3681, 3712, 3742, 3770, 3798, 3824,
    3849, 3873, 3896, 3917, 3937, 3956, 3974, 3991, 4006, 4021,
    4034, 4046, 4056, 4065, 4074, 4080, 4086, 4090, 4094, 4095,
    4096,
];

/// Sine of a whole number of degrees, as `Fx` (4096 is one). Any integer
/// angle; negative and large ones wrap.
pub const fn sin_deg(d: i32) -> Fx {
    let d = d.rem_euclid(360);
    Fx(match d {
        0..=90 => SIN_DEG[d as usize],
        91..=180 => SIN_DEG[(180 - d) as usize],
        181..=270 => -SIN_DEG[(d - 180) as usize],
        _ => -SIN_DEG[(360 - d) as usize],
    })
}

pub const fn cos_deg(d: i32) -> Fx {
    sin_deg(d + 90)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// H6 — integer helpers. The integer square root rounds down; the sine
    /// table is exact at 0°, 30° and 90°; a right shift of a negative number
    /// rounds toward negative infinity.
    #[test]
    fn h6_integer_helpers_give_the_hand_computed_values() {
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(16), 4);
        assert_eq!(isqrt(17), 4);
        assert_eq!(isqrt(1u64 << 62), 1u64 << 31);
        assert_eq!(sin_deg(0), Fx(0));
        assert_eq!(sin_deg(90), ONE, "exactly one at 90°");
        assert_eq!(sin_deg(30), Fx(ONE.0 / 2), "exactly one half at 30°");
        assert_eq!(-5i32 >> 1, -3, "a right shift floors");
        assert_eq!(-1i32 >> 12, -1);
    }

    #[test]
    fn products_and_quotients_round_toward_zero_on_both_sides() {
        let a = Fx(5);
        let half = Fx(ONE.0 / 2);
        assert_eq!(a * half, Fx(2));
        assert_eq!((-a) * half, Fx(-2), "odd-symmetric, not floored to -3");
        assert_eq!(Fx(-7) / 2, Fx(-3));
        assert_eq!(Fx(-7).scale(1, 2), Fx(-3));
        assert_eq!(V2::new(Fx(-3), Fx(9)).scale(1, 2), V2::new(Fx(-1), Fx(4)));
    }

    #[test]
    fn the_sine_table_is_symmetric_and_bounded() {
        for d in -720..720 {
            assert_eq!(sin_deg(d), -sin_deg(-d), "odd at {d}");
            assert_eq!(sin_deg(d), sin_deg(180 - d), "mirror at {d}");
            let s = sin_deg(d).0 as i64;
            let c = cos_deg(d).0 as i64;
            let r = s * s + c * c;
            let one = (ONE.0 as i64) * (ONE.0 as i64);
            assert!((r - one).abs() < one / 1000, "sin²+cos² at {d} is {r}");
        }
    }

    #[test]
    fn isqrt_floors_every_value_near_a_square() {
        for r in [0u64, 1, 2, 3, 1000, 65535, 4_000_000_000] {
            let sq = r * r;
            assert_eq!(isqrt(sq), r);
            if sq > 0 {
                assert_eq!(isqrt(sq - 1), r - 1);
            }
            assert_eq!(isqrt(sq + 1), r.max(if sq == 0 { 1 } else { r }));
        }
    }

    #[test]
    fn an_fx_times_an_int_is_widened_before_it_is_narrowed() {
        // 2^20 * 2^12 overflows i32 in the middle and fits after: a check
        // that would wrap in Floodline's Mul<i32>.
        let a = Fx(1 << 20);
        assert_eq!((a * 4096) / 8192, Fx(1 << 19));
        assert_eq!(a.scale(4096, 8192), Fx(1 << 19));
    }
}
