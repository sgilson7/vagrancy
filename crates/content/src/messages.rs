//! Which copy string says what, for outcomes core reports.
//!
//! Choosing the sentence is a rule, so it lives here where a test reaches
//! it, not in the page. The page only fills the placeholders it is given.

use serde_json::{json, Value};
use sim::replay::ReplayError;

/// `{ "key": "...", "vars": { ... } }`, for the page to look up and fill.
pub fn replay_error(e: ReplayError) -> Value {
    match e {
        ReplayError::Format => json!({ "key": "replay.error.format", "vars": {} }),
        ReplayError::Sim { theirs, ours } => json!({ "key": "replay.error.sim", "vars": { "theirs": theirs, "ours": ours } }),
        ReplayError::Damaged => json!({ "key": "replay.error.damaged", "vars": {} }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fill a copy string the way the page does, for a test to read.
    fn fill(v: &Value) -> String {
        let copy = crate::copy::copy();
        let mut s = v["key"].as_str().unwrap().split('.').fold(&copy, |o, k| &o[k]).as_str().unwrap().to_string();
        s = s.replace("{game}", copy["game"]["name"].as_str().unwrap());
        for (k, val) in v["vars"].as_object().unwrap() {
            s = s.replace(&format!("{{{k}}}"), &val.to_string());
        }
        assert!(!s.contains('{'), "an unfilled placeholder in {s}");
        s
    }

    #[test]
    fn a_replay_from_another_sim_version_is_refused_with_a_sentence() {
        let s = fill(&replay_error(ReplayError::Sim { theirs: 3, ours: 1 }));
        assert_eq!(
            s,
            "This replay was recorded with simulation version 3, and this build runs version 1. \
             The same inputs would not produce the same match, so the replay was not loaded."
        );
        assert!(fill(&replay_error(ReplayError::Damaged)).starts_with("This replay file is incomplete"));
        assert!(fill(&replay_error(ReplayError::Format)).contains("is not a Vagrancy replay"));
    }
}
