//! Save v1: road progress, key bindings and options, as a JSON file the
//! player keeps (D15).
//!
//! Refused in two passes, after gear-master-2d's `crates/core/src/save.rs`:
//! the envelope (`format`, `version`) first, so a file from a newer build is
//! named as newer and not as damage; then the body. Every encoding below
//! destructures its struct with no `..`, so adding a field is a compile error
//! until the file carries it (CLAUDE.md).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// A lowercase identifier, not a player-read string (PLAN.md §8 Q12).
pub const FORMAT: &str = "vagrancy.save";
pub const VERSION: u32 = 1;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveState {
    pub road: Road,
    pub bindings: Bindings,
    pub options: Options,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Road {
    /// Stops won at least once, by opponent id.
    pub cleared: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bindings {
    pub solo: BTreeMap<String, String>,
    pub left: BTreeMap<String, String>,
    pub right: BTreeMap<String, String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    /// 0..=100.
    pub music_volume: u32,
    pub remember_track: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SaveError {
    /// `settings.save.error.format`
    Format,
    /// `settings.save.error.newer`
    Newer { theirs: u32, ours: u32 },
    /// `settings.save.error.damaged`
    Damaged,
}

impl SaveError {
    /// The copy key and values of the sentence that refuses the file.
    pub fn message(self) -> Value {
        match self {
            SaveError::Format => json!({ "key": "settings.save.error.format", "vars": {} }),
            SaveError::Newer { theirs, ours } => json!({ "key": "settings.save.error.newer", "vars": { "theirs": theirs, "ours": ours } }),
            SaveError::Damaged => json!({ "key": "settings.save.error.damaged", "vars": {} }),
        }
    }
}

/// A new save: nothing cleared, the default keys, default options.
pub fn fresh() -> SaveState {
    let controls: Value = serde_json::from_str(include_str!("../../../data/controls.json")).expect("data/controls.json");
    let group = |name: &str| -> BTreeMap<String, String> {
        controls[name].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect()
    };
    SaveState {
        road: Road::default(),
        bindings: Bindings { solo: group("solo"), left: group("left"), right: group("right") },
        options: Options { music_volume: 70, remember_track: false },
    }
}

/// The file's text.
pub fn encode(s: &SaveState) -> String {
    let SaveState { road, bindings, options } = s;
    let Road { cleared } = road;
    let Bindings { solo, left, right } = bindings;
    let Options { music_volume, remember_track } = options;
    let body = json!({
        "format": FORMAT,
        "version": VERSION,
        "state": {
            "road": { "cleared": cleared },
            "bindings": { "solo": solo, "left": left, "right": right },
            "options": { "music_volume": music_volume, "remember_track": remember_track },
        }
    });
    serde_json::to_string_pretty(&body).expect("a save always encodes")
}

#[derive(Deserialize)]
struct Envelope {
    format: Value,
    version: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[allow(dead_code)]
    format: String,
    #[allow(dead_code)]
    version: u32,
    state: SaveState,
}

/// Read a save file, refusing rather than half-loading.
pub fn decode(text: &str) -> Result<SaveState, SaveError> {
    let env: Envelope = serde_json::from_str(text).map_err(|_| SaveError::Format)?;
    if env.format.as_str() != Some(FORMAT) {
        return Err(SaveError::Format);
    }
    let version = env.version.as_u64().ok_or(SaveError::Damaged)? as u32;
    if version > VERSION {
        return Err(SaveError::Newer { theirs: version, ours: VERSION });
    }
    let file: File = serde_json::from_str(text).map_err(|_| SaveError::Damaged)?;
    let mut s = file.state;
    // A save still holding the earlier one-player layout exactly (as every
    // save made before Sam split the keys between the hands does, unless its
    // player changed them) moves to the new layout; a player's own choices
    // stay.
    let controls: Value = serde_json::from_str(include_str!("../../../data/controls.json")).expect("data/controls.json");
    if let Some(before) = controls["_solo_before"].as_object() {
        let matches = before.iter().all(|(k, v)| s.bindings.solo.get(k).map(String::as_str) == v.as_str());
        if matches {
            s.bindings.solo = fresh().bindings.solo;
        }
    }
    // A save written before the jump and the dodge existed has no keys for
    // them: it takes the defaults rather than being called damaged.
    let fresh = fresh();
    for (g, d) in [(&mut s.bindings.solo, &fresh.bindings.solo), (&mut s.bindings.left, &fresh.bindings.left), (&mut s.bindings.right, &fresh.bindings.right)] {
        for (action, code) in d {
            if !g.contains_key(action) && !g.values().any(|c| c == code) {
                g.insert(action.clone(), code.clone());
            }
        }
    }
    validate(&s).then_some(s).ok_or(SaveError::Damaged)
}

/// Known stops, the six actions in every group, no key used twice in a
/// group, a volume in range.
fn validate(s: &SaveState) -> bool {
    let stops = crate::road::stops();
    let actions: Vec<&str> = sim::Input::ACTIONS.iter().map(|(a, _)| *a).collect();
    let group_ok = |g: &BTreeMap<String, String>| {
        let keys: Vec<&String> = g.keys().collect();
        let mut codes: Vec<&String> = g.values().collect();
        codes.sort();
        codes.dedup();
        keys.len() == actions.len() && actions.iter().all(|a| g.contains_key(*a)) && codes.len() == g.len() && g.values().all(|c| !c.is_empty())
    };
    s.road.cleared.iter().all(|c| stops.contains(c))
        && group_ok(&s.bindings.solo)
        && group_ok(&s.bindings.left)
        && group_ok(&s.bindings.right)
        && s.options.music_volume <= 100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_round_trips() {
        let mut s = fresh();
        s.road.cleared = vec!["scarecrow".into(), "gatekeeper".into()];
        s.bindings.solo.insert("shoulder_up".into(), "KeyZ".into());
        s.options.music_volume = 35;
        assert_eq!(decode(&encode(&s)), Ok(s));
    }

    #[test]
    fn a_save_from_a_newer_version_is_refused_by_name() {
        let text = encode(&fresh()).replace("\"version\": 1", "\"version\": 2");
        assert_eq!(decode(&text), Err(SaveError::Newer { theirs: 2, ours: 1 }));
    }

    #[test]
    fn a_damaged_save_is_refused_rather_than_half_loaded() {
        let good = encode(&fresh());
        for bad in [
            good[..good.len() / 2].to_string(),
            good.replace("\"music_volume\": 70", "\"music_volume\": 700"),
            good.replace("\"cleared\": []", "\"cleared\": [\"nobody\"]"),
            good.replace("\"KeyW\"", "\"KeyQ\""),
            good.replace("\"remember_track\": false", "\"remember_track\": false, \"extra\": 1"),
        ] {
            assert!(matches!(decode(&bad), Err(SaveError::Damaged) | Err(SaveError::Format)), "accepted: {bad}");
        }
        assert_eq!(decode(&good.replace("\"music_volume\": 70", "\"music_volume\": 700")), Err(SaveError::Damaged));
    }

    #[test]
    fn a_save_with_the_old_one_player_keys_moves_to_the_new_layout() {
        let controls: Value = serde_json::from_str(include_str!("../../../data/controls.json")).unwrap();
        let mut old = fresh();
        old.bindings.solo = controls["_solo_before"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect();
        let back = decode(&encode(&old)).unwrap();
        assert_eq!(back.bindings.solo, fresh().bindings.solo);
        assert_eq!(back.bindings.solo["shoulder_up"], "KeyI");
        // A player's own layout is left alone.
        let mut mine = fresh();
        mine.bindings.solo.insert("shoulder_up".into(), "KeyZ".into());
        assert_eq!(decode(&encode(&mine)).unwrap().bindings.solo["shoulder_up"], "KeyZ");
    }

    #[test]
    fn a_save_from_before_the_jump_takes_the_default_keys_for_it() {
        let mut old = fresh();
        for g in [&mut old.bindings.solo, &mut old.bindings.left, &mut old.bindings.right] {
            g.remove("jump");
            g.remove("dodge");
        }
        let back = decode(&encode(&old)).expect("an older save still loads");
        assert_eq!(back.bindings, fresh().bindings);
    }

    #[test]
    fn a_file_that_is_not_a_save_is_refused_as_such() {
        assert_eq!(decode("not json"), Err(SaveError::Format));
        assert_eq!(decode("{\"format\": \"something.else\", \"version\": 1}"), Err(SaveError::Format));
    }
}
