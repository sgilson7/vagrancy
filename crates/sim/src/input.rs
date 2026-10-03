//! One byte per fighter per tick (D6). The only thing a player, a pilot or a
//! peer can give the world.

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Input(pub u8);

impl Input {
    pub const NONE: Input = Input(0);
    pub const SHOULDER_UP: u8 = 1 << 0;
    pub const SHOULDER_DOWN: u8 = 1 << 1;
    pub const ELBOW_IN: u8 = 1 << 2;
    pub const ELBOW_OUT: u8 = 1 << 3;
    pub const STEP_LEFT: u8 = 1 << 4;
    pub const STEP_RIGHT: u8 = 1 << 5;
    /// "Play the next round". Pressed by the page's button, or by a pilot.
    /// PLAN.md D6 left bits 6 and 7 spare; M1 found that a round cannot be
    /// restarted through the one door without an input for it
    /// (SECOND-ORDER-M1).
    pub const READY: u8 = 1 << 6;
    /// Bit 7 is spare and must be zero; a replay that sets it is refused.
    pub const SPARE: u8 = 1 << 7;

    /// The actions a key can be bound to, in the order Settings lists them.
    /// The page reads these names and bits from here rather than keeping a
    /// copy (CLAUDE.md: the page keeps no constant of its own).
    pub const ACTIONS: [(&'static str, u8); 6] = [
        ("shoulder_up", Self::SHOULDER_UP),
        ("shoulder_down", Self::SHOULDER_DOWN),
        ("elbow_in", Self::ELBOW_IN),
        ("elbow_out", Self::ELBOW_OUT),
        ("step_left", Self::STEP_LEFT),
        ("step_right", Self::STEP_RIGHT),
    ];

    pub const fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }

    /// +1, -1 or 0 for a pair of opposed bits; both held cancel.
    pub const fn axis(self, plus: u8, minus: u8) -> i32 {
        (self.has(plus) as i32) - (self.has(minus) as i32)
    }

    /// The same intent for a fighter facing the other way. Arm bits are
    /// already relative to facing; only the steps are absolute.
    pub const fn mirror(self) -> Input {
        let steps = Self::STEP_LEFT | Self::STEP_RIGHT;
        let mut b = self.0 & !steps;
        if self.has(Self::STEP_LEFT) {
            b |= Self::STEP_RIGHT;
        }
        if self.has(Self::STEP_RIGHT) {
            b |= Self::STEP_LEFT;
        }
        Input(b)
    }
}
