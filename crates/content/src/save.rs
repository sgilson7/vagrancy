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
/// 2: the road keeps each stop's best result, not only that it was won,
/// because a fight can ask for a flawless or a quick win (Sam's tree).
/// 3: the tutorial's finished missions.
/// 4: the weapon the player carries.
/// 5: each fight's best result names the weapons it was won with (weapon
/// challenges). A version 4 file reads as it is, with none named.
/// 6: each best result says whether a won match there had a headshot and a
/// round won untouched (the flanked fights' requirements). A version 5 file
/// reads as it is, with neither.
/// 7: how many chapters of story mode are finished. A version 6 file reads
/// as it is, with none.
/// 8: each best result says whether a won match there had a round ended by
/// a thrown blade, and every won round ended so (the boomerang's fights). A
/// version 7 file reads as it is, with neither.
/// 9: whether the player fights with four arms. A version 8 file reads as it
/// is, with two.
/// 10: each best result says whether a won match there had a round won with
/// no blade in hand. A version 9 file reads as it is, with none.
/// 11: the most chapters cleared in one full run of story mode. A version
/// 10 file reads as it is, with none.
/// 12: the outfit the player wears, one won from an opponent beaten in
/// arcade mode (Sam, 2026-10-08). A version 11 file reads as it is, plain.
pub const VERSION: u32 = 12;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveState {
    pub road: Road,
    pub bindings: Bindings,
    pub options: Options,
    /// Finished tutorial tasks, by `content::tutorial::part_key`.
    pub tutorial: Vec<String>,
    /// The weapon carried on the road, in the yard and online, by id.
    pub weapon: String,
    /// Story mode chapters finished; the next one is open.
    #[serde(default)]
    pub story: u32,
    /// The player fights with four arms (won at the final fight). Written
    /// only when on.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub four_arms: bool,
    /// The most chapters cleared in one full run of story mode. Written
    /// only when there are some.
    #[serde(default)]
    pub story_full: u32,
    /// The outfit the player wears, by the id of the opponent it was won
    /// from (content::costumes::outfits). Written only when one is worn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outfit: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Road {
    /// The best won match at each stop won at least once.
    pub best: BTreeMap<String, crate::road::Best>,
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
        tutorial: Vec::new(),
        weapon: crate::weapons::DEFAULT.to_string(),
        story: 0,
        four_arms: false,
        story_full: 0,
        outfit: None,
    }
}

/// A save with each fight on the road won 3 rounds to none, with each feat
/// a requirement can ask for, and story mode cleared in a full run, except
/// the fights named in `unwon`: for Sam
/// to try the end of arcade mode without playing to it (`lab test-saves`,
/// testing/saves/).
pub fn finished(unwon: &[&str]) -> SaveState {
    let mut s = fresh();
    let with: Vec<String> = crate::weapons::weapons().into_iter().filter(|w| !w.enemy_only).map(|w| w.id).collect();
    for st in crate::road::road().into_iter().filter(|st| !unwon.contains(&st.id.as_str())) {
        let best = crate::road::Best { losses: 0, ticks: 3600, with: with.clone(), headshot: true, untouched: true, thrown: true, all_thrown: true, bladeless: true };
        s.road.best.insert(st.id, best);
    }
    // And story mode walked, in a full run: the extra chapter open.
    let chapters = crate::story::story().chapters.len() as u32;
    s.story = chapters;
    s.story_full = chapters;
    s
}

/// The file's text.
pub fn encode(s: &SaveState) -> String {
    let SaveState { road, bindings, options, tutorial, weapon, story, four_arms, story_full, outfit } = s;
    let Road { best } = road;
    let Bindings { solo, left, right } = bindings;
    let Options { music_volume, remember_track } = options;
    let mut body = json!({
        "format": FORMAT,
        "version": VERSION,
        "state": {
            "road": { "best": best },
            "bindings": { "solo": solo, "left": left, "right": right },
            "options": { "music_volume": music_volume, "remember_track": remember_track },
            "tutorial": tutorial,
            "weapon": weapon,
            "story": story,
        }
    });
    // Written only when on, like the field's default.
    if *four_arms {
        body["state"]["four_arms"] = json!(true);
    }
    if *story_full > 0 {
        body["state"]["story_full"] = json!(story_full);
    }
    if let Some(o) = outfit {
        body["state"]["outfit"] = json!(o);
    }
    serde_json::to_string_pretty(&body).expect("a save always encodes")
}

#[derive(Deserialize)]
struct Envelope {
    format: Value,
    version: Value,
}

/// A version 3 file: no weapon.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileV3 {
    #[allow(dead_code)]
    format: String,
    #[allow(dead_code)]
    version: u32,
    state: StateV3,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StateV3 {
    road: Road,
    bindings: Bindings,
    options: Options,
    tutorial: Vec<String>,
}

/// A version 2 file: no tutorial.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileV2 {
    #[allow(dead_code)]
    format: String,
    #[allow(dead_code)]
    version: u32,
    state: StateV2,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StateV2 {
    road: Road,
    bindings: Bindings,
    options: Options,
}

/// A version 1 file: the road was a list of stops won.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileV1 {
    #[allow(dead_code)]
    format: String,
    #[allow(dead_code)]
    version: u32,
    state: StateV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StateV1 {
    road: RoadV1,
    bindings: Bindings,
    options: Options,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoadV1 {
    cleared: Vec<String>,
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
    let mut s = if version < 2 {
        // Its wins carry over; how they were won was not kept.
        let v1: FileV1 = serde_json::from_str(text).map_err(|_| SaveError::Damaged)?;
        let StateV1 { road, bindings, options } = v1.state;
        let best = road.cleared.into_iter().map(|id| (id, crate::road::Best::UNKNOWN)).collect();
        SaveState { road: Road { best }, bindings, options, tutorial: Vec::new(), weapon: crate::weapons::DEFAULT.into(), story: 0, four_arms: false, story_full: 0, outfit: None }
    } else if version < 3 {
        // No tutorial yet.
        let v2: FileV2 = serde_json::from_str(text).map_err(|_| SaveError::Damaged)?;
        let StateV2 { road, bindings, options } = v2.state;
        SaveState { road, bindings, options, tutorial: Vec::new(), weapon: crate::weapons::DEFAULT.into(), story: 0, four_arms: false, story_full: 0, outfit: None }
    } else if version < 4 {
        // No weapon chosen yet: the sword.
        let v3: FileV3 = serde_json::from_str(text).map_err(|_| SaveError::Damaged)?;
        let StateV3 { road, bindings, options, tutorial } = v3.state;
        SaveState { road, bindings, options, tutorial, weapon: crate::weapons::DEFAULT.into(), story: 0, four_arms: false, story_full: 0, outfit: None }
    } else {
        let file: File = serde_json::from_str(text).map_err(|_| SaveError::Damaged)?;
        file.state
    };
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
    // A weapon the player has not unlocked, or an enemy's, or one this
    // build does not know, becomes the sword rather than refusing the file.
    // The cursed blade was the longsword until SIM_VERSION 17, when the
    // longsword became the villagers' long blade.
    if s.weapon == "longsword" {
        s.weapon = "cursed_blade".into();
    }
    s.weapon = crate::weapons::usable(&s.weapon, &s.road.best);
    // Four arms not yet won are two.
    s.four_arms &= crate::road::four_arms_open(&s.road.best);
    // An outfit not won is none.
    if s.outfit.as_ref().is_some_and(|o| !crate::costumes::outfits(&s.road.best).contains(o)) {
        s.outfit = None;
    }
    validate(&s).then_some(s).ok_or(SaveError::Damaged)
}

/// Known stops, the six actions in every group, no key used twice in a
/// group, a volume in range.
fn validate(s: &SaveState) -> bool {
    if s.story as usize > crate::story::story().chapters.len() || s.story_full as usize > crate::story::story().chapters.len() {
        return false;
    }
    let stops = crate::road::stops();
    let actions: Vec<&str> = sim::Input::ACTIONS.iter().map(|(a, _)| *a).collect();
    let group_ok = |g: &BTreeMap<String, String>| {
        let keys: Vec<&String> = g.keys().collect();
        let mut codes: Vec<&String> = g.values().collect();
        codes.sort();
        codes.dedup();
        keys.len() == actions.len() && actions.iter().all(|a| g.contains_key(*a)) && codes.len() == g.len() && g.values().all(|c| !c.is_empty())
    };
    let parts: Vec<String> = crate::tutorial::missions().iter().flat_map(|m| (0..m.tasks.len()).map(|i| crate::tutorial::part_key(m, i)).collect::<Vec<_>>()).collect();
    let weapons: Vec<String> = crate::weapons::weapons().into_iter().map(|w| w.id).collect();
    s.road.best.keys().all(|c| stops.contains(c))
        && s.road.best.values().all(|b| b.with.iter().all(|w| weapons.contains(w)))
        && s.tutorial.iter().all(|t| parts.contains(t))
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
        s.road.best.insert("scarecrow".into(), crate::road::Best::won(0, 900, "sword"));
        s.road.best.insert("gatekeeper".into(), crate::road::Best::won(2, 4000, "sword"));
        s.bindings.solo.insert("shoulder_up".into(), "KeyZ".into());
        s.options.music_volume = 35;
        assert_eq!(decode(&encode(&s)), Ok(s));
    }

    #[test]
    fn a_save_from_a_newer_version_is_refused_by_name() {
        let text = encode(&fresh()).replace(&format!("\"version\": {VERSION}"), &format!("\"version\": {}", VERSION + 1));
        assert_eq!(decode(&text), Err(SaveError::Newer { theirs: VERSION + 1, ours: VERSION }));
    }

    #[test]
    fn a_damaged_save_is_refused_rather_than_half_loaded() {
        let good = encode(&fresh());
        for bad in [
            good[..good.len() / 2].to_string(),
            good.replace("\"music_volume\": 70", "\"music_volume\": 700"),
            good.replace("\"best\": {}", "\"best\": {\"nobody\": {\"losses\": 0, \"ticks\": 1}}"),
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
    fn a_version_1_save_keeps_its_wins_as_wins_of_unknown_margin() {
        let v1 = encode(&fresh()).replace(&format!("\"version\": {VERSION}"), "\"version\": 1").replace("\"best\": {}", "\"cleared\": [\"scarecrow\", \"thresher\"]").replace(",\n    \"tutorial\": []", "").replace(",\n    \"weapon\": \"sword\"", "").replace(",\n    \"story\": 0", "");
        let s = decode(&v1).expect("a version 1 save loads");
        let unknown = crate::road::Best::UNKNOWN;
        assert_eq!(s.road.best, BTreeMap::from([("scarecrow".to_string(), unknown.clone()), ("thresher".to_string(), unknown)]));
        // A win of unknown margin opens what a win opens, and nothing that
        // asks for a flawless or a quick one.
        use crate::road::Req;
        assert!(Req::Beat("scarecrow".into()).met(&s.road.best));
        assert!(!Req::Flawless("scarecrow".into()).met(&s.road.best));
        assert!(!Req::Quick { stop: "scarecrow".into(), seconds: 600 }.met(&s.road.best));
        assert_eq!(decode(&encode(&s)), Ok(s), "and it is written back as the current version");
    }

    #[test]
    fn a_version_2_save_loads_with_no_tutorial_done_and_a_finished_mission_round_trips() {
        let v2 = encode(&fresh()).replace(&format!("\"version\": {VERSION}"), "\"version\": 2").replace(",\n    \"tutorial\": []", "").replace(",\n    \"weapon\": \"sword\"", "").replace(",\n    \"story\": 0", "");
        assert!(!v2.contains("tutorial"), "{v2}");
        let s = decode(&v2).expect("a version 2 save loads");
        assert!(s.tutorial.is_empty());
        let mut done = fresh();
        done.tutorial = vec!["m_shoulder".into(), "c_sweep/1".into()];
        assert_eq!(decode(&encode(&done)), Ok(done.clone()));
        let mut bad = done;
        bad.tutorial.push("no_such_mission".into());
        assert_eq!(decode(&encode(&bad)), Err(SaveError::Damaged));
    }

    #[test]
    fn a_save_carries_an_unlocked_weapon_and_any_other_becomes_the_sword() {
        // The scimitar opens with a win at the pilgrim, who carries it
        // (data/weapons.json).
        let mut s = fresh();
        s.weapon = "scimitar".into();
        assert_eq!(decode(&encode(&s)).unwrap().weapon, "sword", "a locked weapon was carried");
        s.road.best.insert("pilgrim".into(), crate::road::Best::won(1, 2000, "sword"));
        assert_eq!(decode(&encode(&s)).unwrap().weapon, "scimitar");
        // The cursed blade waits for the final fight.
        for id in crate::road::stops().into_iter().filter(|id| Some(id) != crate::road::last().map(|l| l.id).as_ref()) {
            s.road.best.insert(id, crate::road::Best::won(0, 1, "sword"));
        }
        s.weapon = "cursed_blade".into();
        assert_eq!(decode(&encode(&s)).unwrap().weapon, "sword", "the cursed blade was carried before the local deity was beaten");
        s.road.best.insert("local_deity".into(), crate::road::Best::won(0, 1, "sword"));
        assert_eq!(decode(&encode(&s)).unwrap().weapon, "cursed_blade");
        // A save from before SIM_VERSION 17 named it the longsword, which
        // is now the villagers' long blade: it carries on as the cursed blade.
        s.weapon = "longsword".into();
        assert_eq!(decode(&encode(&s)).unwrap().weapon, "cursed_blade");
        s.weapon = "no such weapon".into();
        assert_eq!(decode(&encode(&s)).unwrap().weapon, "sword");
        // A version 3 save carries the sword.
        let v3 = encode(&fresh()).replace(&format!("\"version\": {VERSION}"), "\"version\": 3").replace(",\n    \"weapon\": \"sword\"", "").replace(",\n    \"story\": 0", "");
        assert!(!v3.contains("weapon"), "{v3}");
        assert_eq!(decode(&v3).unwrap().weapon, "sword");
    }

    #[test]
    fn a_version_4_save_keeps_its_wins_with_no_weapon_named_and_a_win_names_its_weapon() {
        let mut s = fresh();
        s.road.best.insert("scarecrow".into(), crate::road::Best::won(0, 900, "sword"));
        let v4 = encode(&s).replace(&format!("\"version\": {VERSION}"), "\"version\": 4").replace(",\n          \"with\": [\n            \"sword\"\n          ]", "");
        assert!(!v4.contains("with"), "{v4}");
        let back = decode(&v4).expect("a version 4 save loads");
        assert!(back.road.best["scarecrow"].with.is_empty());
        use crate::road::Req;
        let challenge = Req::With { stop: "scarecrow".into(), weapon: "sword".into() };
        assert!(!challenge.met(&back.road.best), "a win of unknown weapon met a weapon challenge");
        assert!(challenge.met(&s.road.best));
        // A weapon this build does not know is damage.
        let mut bad = s.clone();
        bad.road.best.insert("thresher".into(), crate::road::Best::won(0, 900, "spoon"));
        assert_eq!(decode(&encode(&bad)), Err(SaveError::Damaged));
    }

    #[test]
    fn a_version_5_save_keeps_its_wins_with_no_feats_and_a_win_keeps_its_feats() {
        let mut s = fresh();
        let mut f = crate::road::Feats::default();
        f.headshot = true;
        f.untouched = true;
        s.road.best.insert("thresher".into(), crate::road::Best::won(0, 900, "sword").with_feats(f));
        let v5 = encode(&s).replace(&format!("\"version\": {VERSION}"), "\"version\": 5").replace("\"headshot\": true,\n          ", "").replace(",\n          \"untouched\": true", "");
        assert!(!v5.contains("headshot") && !v5.contains("untouched"), "{v5}");
        let back = decode(&v5).expect("a version 5 save loads");
        use crate::road::Req;
        assert!(Req::Beat("thresher".into()).met(&back.road.best));
        assert!(!Req::Headshot("thresher".into()).met(&back.road.best), "a version 5 win met a headshot requirement");
        assert!(!Req::Untouched("thresher".into()).met(&back.road.best));
        let now = decode(&encode(&s)).unwrap();
        assert!(Req::Headshot("thresher".into()).met(&now.road.best) && Req::Untouched("thresher".into()).met(&now.road.best));
    }

    #[test]
    fn a_file_that_is_not_a_save_is_refused_as_such() {
        assert_eq!(decode("not json"), Err(SaveError::Format));
        assert_eq!(decode("{\"format\": \"something.else\", \"version\": 1}"), Err(SaveError::Format));
    }
}
