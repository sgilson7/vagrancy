//! Every string a player reads is checked against `TONE.md`, the show's
//! deny-list and the name rule (PLANNING-BRIEF 0.2, 0.6; TONE.md).
//!
//! Each rule below names the TONE.md rule it implements. A rule that needs a
//! reviewer (sentence case, a sentence with no verb) is not pretended here.
//! What a player sees on the page is checked again by `testing/drive.py`,
//! from rendered text, so this file is not the only guard.

use content::copy::{copy, placeholders, player_strings};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).canonicalize().unwrap()
}

/// Lowercase words, keeping hyphens and apostrophes inside a word.
fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '\''))
        .map(|w| w.trim_matches(|c| c == '-' || c == '\'').to_lowercase())
        .map(|w| w.strip_suffix("'s").map(str::to_string).unwrap_or(w))
        .filter(|w| !w.is_empty())
        .collect()
}

/// A phrase as a whole-word match, case-insensitive.
fn has_phrase(s: &str, phrase: &str) -> bool {
    let w = words(s).join(" ");
    let p = words(phrase).join(" ");
    format!(" {w} ").contains(&format!(" {p} "))
}

// Rule 4: concrete verbs, no praise words.
const PRAISE: &[&str] = &[
    "epic", "brutal", "ultimate", "deadly", "awesome", "insane", "intense", "legendary",
    "unleash", "unleashes", "unleashed", "destroy", "destroys", "destroyed",
];
// Rule 8: no invented universals, unless `_universals` names the test.
const UNIVERSALS: &[&str] = &[
    "always", "never", "every", "all", "only", "impossible", "unbeatable", "guarantee",
    "guarantees", "instantly", "nothing is stored",
];
// Rule 10: no digit and no number word from three up. "One" and "two" stay.
const NUMBER_WORDS: &[&str] = &[
    "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve",
    "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen",
    "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety", "hundred",
    "thousand", "million", "dozen",
];
const DIGIT_ALLOWED: &[&str] = &["mp3"];
// Rule 13: a cut is described by its rule, not its sensation.
const GORE: &[&str] = &[
    "gore", "gory", "guts", "flesh", "bleed", "bleeds", "bleeding", "pain", "painful", "agony",
    "wound", "wounds", "wounded", "sever", "severs", "severed", "decapitate", "decapitated",
    "dismember", "dismembered", "mutilate", "mutilated", "slaughter", "butcher", "carnage",
    "kill", "kills", "killed", "killing", "blood", "bloody", "corpse", "dead", "die", "dies",
];
// `_glossary.ink.note`: "The word blood appears nowhere." Sam removed the
// one line that used it, game.content_note, on 2026-10-04.
const GORE_ALLOWED: &[(&str, &str)] = &[];
// Rule 14: no red word for ink or a cut.
const RED_WORDS: &[&str] = &[
    "red", "reds", "reddish", "crimson", "scarlet", "vermilion", "maroon", "blood-red", "ruby",
    "carmine", "cerise",
];
// Rule 16: no speech filler; American spelling, spot-checked.
const FILLER: &[&str] = &["basically", "essentially", "just", "simply"];
const BRITISH: &[&str] = &[
    "colour", "colours", "favour", "behaviour", "centre", "metre", "organise", "recognise",
    "travelling", "travelled", "armour", "grey", "defence",
];
// Rule 3: in a versus string, "you" should not be there.
const VERSUS_PREFIXES: &[&str] = &["results.versus.", "local."];
const YOU: &[&str] = &["you", "your", "yours", "yourself"];
/// Carried, not repaired: PLAN.md §8 Q10. Sam rewrites the string or the rule.
const YOU_ALLOWED: &[&str] = &["local.keyboard_limit"];

/// The show's proper nouns (PLAN.md §8 Q13, confirmed with the plan). A match
/// anywhere in `data/` or `web/` fails, in any case.
const DENY: &[&str] = &[
    "champloo", "mugen", "jin", "fuu", "manglobe", "watanabe", "nujabes", "force of nature",
    "fat jon", "tsutchie", "shing02", "minmi", "battlecry", "shiki no uta", "masta",
    "sunflower", "sunflowers",
];

/// Every TONE.md failure in one string, as sentences naming the rule.
fn tone_failures(key: &str, s: &str, universals: &Value) -> Vec<String> {
    let w = words(s);
    let mut f = Vec::new();
    for p in PRAISE {
        if w.iter().any(|x| x == p) {
            f.push(format!("rule 4: praise word `{p}`"));
        }
    }
    for u in UNIVERSALS {
        if has_phrase(s, u) && universals.get(key).is_none() {
            f.push(format!("rule 8: `{u}` with no test named in _universals"));
        }
    }
    for n in NUMBER_WORDS {
        if w.iter().any(|x| x == n) {
            f.push(format!("rule 10: number word `{n}`; fill it from core"));
        }
    }
    let stripped: String = {
        // Placeholders are filled from core, so digits inside `{}` are not typed.
        let mut out = String::new();
        let mut depth = 0;
        for c in s.chars() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ if depth == 0 => out.push(c),
                _ => {}
            }
        }
        out
    };
    for tok in words(&stripped) {
        if tok.chars().any(|c| c.is_ascii_digit()) && !DIGIT_ALLOWED.contains(&tok.as_str()) {
            f.push(format!("rule 10: digit in `{tok}`; fill it from core"));
        }
    }
    if s.contains('!') {
        f.push("rule 12: an exclamation mark".into());
    }
    for g in GORE {
        if w.iter().any(|x| x == g) && !GORE_ALLOWED.contains(&(key, g)) {
            f.push(format!("rule 13: `{g}` describes sensation, not rule"));
        }
    }
    for r in RED_WORDS {
        if w.iter().any(|x| x == r) {
            f.push(format!("rule 14: red word `{r}`"));
        }
    }
    for x in FILLER {
        if w.iter().any(|y| y == x) {
            f.push(format!("rule 16: filler `{x}`"));
        }
    }
    for b in BRITISH {
        if w.iter().any(|y| y == b) {
            f.push(format!("rule 16: British spelling `{b}`"));
        }
    }
    if (s.contains('…') || s.contains("...")) && key != "game.loading" {
        f.push("rule 16: an ellipsis outside a loading line".into());
    }
    if VERSUS_PREFIXES.iter().any(|p| key.starts_with(p)) && !YOU_ALLOWED.contains(&key) {
        for y in YOU {
            if w.iter().any(|x| x == y) {
                f.push(format!("rule 3: `{y}` in a versus string; use the fighters' names"));
            }
        }
    }
    f
}

fn glossary_failures(s: &str, glossary: &Value) -> Vec<String> {
    let mut f = Vec::new();
    if let Some(m) = glossary.as_object() {
        for (term, entry) in m {
            for syn in entry["not"].as_array().into_iter().flatten().filter_map(|v| v.as_str()) {
                if has_phrase(s, syn) {
                    f.push(format!("rule 5: `{syn}` renames the glossary term `{term}`"));
                }
            }
        }
    }
    f
}

#[test]
fn no_string_breaks_the_tone_file() {
    let c = copy();
    let strings = player_strings(&c);
    assert!(strings.len() > 150, "only {} strings found; is the file being walked?", strings.len());
    let mut problems = Vec::new();
    for (k, s) in &strings {
        for why in tone_failures(k, s, &c["_universals"]).into_iter().chain(glossary_failures(s, &c["_glossary"])) {
            problems.push(format!("{k}: {why}\n    \"{s}\""));
        }
    }
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n"));
}

#[test]
fn the_tone_lint_catches_each_rule() {
    let none = Value::Null;
    let caught = |k: &str, s: &str| !tone_failures(k, s, &none).is_empty();
    assert!(caught("x", "An epic swing."), "rule 4");
    assert!(caught("x", "The blade always cuts."), "rule 8");
    assert!(caught("x", "Win three rounds."), "rule 10, a word");
    assert!(caught("x", "Win 3 rounds."), "rule 10, a digit");
    assert!(!caught("x", "Win {rounds_to_win} rounds."), "a placeholder is not a typed number");
    assert!(!caught("x", "MP3 files play."), "the allow-list");
    assert!(caught("x", "You won the round!"), "rule 12");
    assert!(caught("x", "The cut was painful."), "rule 13");
    assert!(caught("x", "Crimson ink spills."), "rule 14");
    assert!(caught("x", "Just swing."), "rule 16, filler");
    assert!(caught("x", "Loading..."), "rule 16, ellipsis");
    assert!(caught("results.versus.x", "You won."), "rule 3");
    assert!(!caught("results.road.x", "You won."), "rule 3 is for versus strings");
    assert!(!caught("x", "Several keys are held."), "a word that starts like `sever` is not one");
    let g = copy();
    assert!(!glossary_failures("Your health is low.", &g["_glossary"]).is_empty(), "rule 5");
}

#[test]
fn every_placeholder_is_one_the_copy_file_explains() {
    let c = copy();
    let explained: Vec<String> = c["_placeholders"]
        .as_object()
        .unwrap()
        .keys()
        .flat_map(|k| k.split(',').map(|p| p.trim().to_string()).collect::<Vec<_>>())
        .collect();
    let mut problems = Vec::new();
    for (k, s) in player_strings(&c) {
        for p in placeholders(&s) {
            if !explained.iter().any(|e| e == p) {
                problems.push(format!("{k}: {{{p}}} is not in _placeholders"));
            }
        }
    }
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n"));
}

fn text_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut v: Vec<_> = rd.map(|e| e.unwrap().path()).collect();
    v.sort();
    for p in v {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if p.is_dir() {
            // Vendored third-party code is listed in LICENSES.md, not authored here.
            if name != "vendor" && name != "music" {
                text_files(&p, out);
            }
        } else if ["html", "js", "css", "json", "md", "txt"]
            .iter()
            .any(|e| name.ends_with(&format!(".{e}")))
        {
            out.push(p);
        }
    }
}

#[test]
fn nothing_from_the_show_appears_in_data_or_web() {
    let root = root();
    let mut files = Vec::new();
    text_files(&root.join("data"), &mut files);
    text_files(&root.join("web"), &mut files);
    assert!(files.len() >= 2, "found {} files to scan", files.len());
    let mut problems = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        for d in DENY {
            if has_phrase(&text, d) {
                problems.push(format!("{}: `{d}`", f.strip_prefix(&root).unwrap().display()));
            }
        }
    }
    assert!(problems.is_empty(), "\nthe show's proper nouns appear in:\n{}\n", problems.join("\n"));
}

#[test]
fn the_game_name_is_spelled_only_in_game_name() {
    // PLANNING-BRIEF, first line. Identifiers (repo, URL, format tags) may
    // spell it in lower case (PLAN.md §8 Q12); a player-read string may not.
    let c = copy();
    let name = c["game"]["name"].as_str().unwrap();
    let holders: Vec<String> =
        player_strings(&c).into_iter().filter(|(_, s)| s.contains(name)).map(|(k, _)| k).collect();
    assert_eq!(holders, vec!["game.name".to_string()], "the name is spelled in {holders:?}");
    let root = root();
    let mut files = Vec::new();
    text_files(&root.join("web"), &mut files);
    text_files(&root.join("data"), &mut files);
    for f in files {
        if f.ends_with("copy.en.json") {
            continue;
        }
        let text = std::fs::read_to_string(&f).unwrap();
        assert!(!text.contains(name), "{} spells the game's name; use {{game}}", f.display());
    }
}

#[test]
fn index_html_carries_no_text_of_its_own() {
    // Every word in the entry page is a `{{key}}` token that packaging fills
    // from the copy file, so the page cannot hold a second copy of a string.
    let html = std::fs::read_to_string(root().join("web/index.html")).unwrap();
    let c = copy();
    let keys: Vec<String> = player_strings(&c).into_iter().map(|(k, _)| k).collect();
    let mut body = html.clone();
    for (open, close) in [("<script", "</script>"), ("<style", "</style>"), ("<!--", "-->")] {
        body = remove_spans(&body, open, close);
    }
    let mut text = String::new();
    let mut attrs = Vec::new();
    let mut in_tag = false;
    let mut tag = String::new();
    for ch in body.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                for a in ["aria-label=\"", "title=\"", "alt=\"", "placeholder=\""] {
                    if let Some(i) = tag.find(a) {
                        let v = &tag[i + a.len()..];
                        attrs.push(v[..v.find('"').unwrap_or(v.len())].to_string());
                    }
                }
                text.push(' ');
            }
            _ if in_tag => tag.push(ch),
            _ => text.push(ch),
        }
    }
    let mut problems = Vec::new();
    let tokens = text.split_whitespace().chain(attrs.iter().map(|s| s.as_str()));
    for t in tokens {
        if t == "&nbsp;" {
            continue;
        }
        match t.strip_prefix("{{").and_then(|t| t.strip_suffix("}}")) {
            Some(k) if keys.iter().any(|x| x == k) => {}
            Some(k) => problems.push(format!("{{{{{k}}}}} is not a key in the copy file")),
            None => problems.push(format!("`{t}` is text the page wrote itself")),
        }
    }
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n"));
}

/// `s` with every `open … close` span removed, ends included.
fn remove_spans(s: &str, open: &str, close: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find(open) {
        out.push_str(&rest[..i]);
        rest = match rest[i..].find(close) {
            Some(j) => &rest[i + j + close.len()..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}
