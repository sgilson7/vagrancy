//! `sim` depends on `serde` and `postcard`, and on nothing else, ever; and its
//! source holds no float, no hash map and no clock.
//!
//! The manifest half is Floodline's `crates/sim/tests/boundary.rs`, ported.
//! The source half is new: Floodline's test reads only `Cargo.toml`
//! (PLAN.md §7 item 1), so a stray `f32` would have passed it.
//!
//! The source scan is a lint that reads text, which the brief lists as an
//! indicator of an invalid syntactic shortcut (E.2). It is used here on
//! purpose, because the rule *is* about source text; to keep it honest it
//! strips comments and string literals before it looks, so a doc comment that
//! says "no f32" does not trip it and a real `f32` cannot hide in one.

use std::collections::BTreeSet;
use std::path::Path;

const ALLOWED: [&str; 2] = ["serde", "postcard"];
/// Tests read the data files through `content`, which is never shipped with
/// `sim`. Nothing else may appear, even here.
const DEV_ALLOWED: [&str; 1] = ["content"];

#[test]
fn sim_depends_on_serde_and_postcard_and_nothing_else() {
    let manifest = include_str!("../Cargo.toml");
    let found = runtime_dependency_names(manifest);
    let allowed: BTreeSet<&str> = ALLOWED.into_iter().collect();
    assert_eq!(
        found, allowed,
        "\n`sim`'s dependencies are {found:?}, and the only ones allowed are {allowed:?}.\n\
         A new one is Sam's decision (PLANNING-BRIEF 0.4). Do not delete this test.\n"
    );
}

#[test]
fn sims_tests_may_read_content_and_nothing_else() {
    let manifest = include_str!("../Cargo.toml");
    let all = dependency_names(manifest);
    let runtime = runtime_dependency_names(manifest);
    let dev: BTreeSet<&str> = all.difference(&runtime).copied().collect();
    let allowed: BTreeSet<&str> = DEV_ALLOWED.into_iter().collect();
    assert!(dev.is_subset(&allowed), "`sim`'s dev-dependencies are {dev:?}; only {allowed:?} may be");
}

/// Names from `[dependencies]` and target-gated `.dependencies]` tables
/// only: what ships with `sim`.
fn runtime_dependency_names(manifest: &str) -> BTreeSet<&str> {
    let mut names = BTreeSet::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line == "[dependencies]" || (line.starts_with("[target.") && line.ends_with(".dependencies]"));
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            let name = key.trim().split('.').next().unwrap_or("").trim_matches('"');
            if !name.is_empty() {
                names.insert(name);
            }
        }
    }
    names
}

#[test]
fn sim_names_no_graphics_physics_or_networking_crate() {
    let manifest = include_str!("../Cargo.toml");
    for banned in [
        "macroquad", "miniquad", "rand", "hashbrown", "rapier2d", "box2d", "nalgebra",
        "glam", "wasm-bindgen", "web-sys", "js-sys", "serde_json",
    ] {
        assert!(
            !dependency_names(manifest).contains(banned),
            "`sim` has picked up {banned}, which is exactly what this crate exists not to do"
        );
    }
}

/// The tokens that would mean the rule has gone, and why each is banned.
const BANNED_TOKENS: [(&str, &str); 7] = [
    ("f32", "a float can round differently on two machines"),
    ("f64", "a float can round differently on two machines"),
    ("HashMap", "iteration order is a decision; use Vec or BTreeMap"),
    ("HashSet", "iteration order is a decision; use Vec or BTreeSet"),
    ("Instant", "sim has no clock; the page owns time"),
    ("SystemTime", "sim has no clock; the page owns time"),
    ("thread_rng", "the one Rng lives in World"),
];

#[test]
fn sim_has_no_float_no_hashmap_and_no_clock() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(!files.is_empty(), "found no source under {}", src.display());
    let mut problems = Vec::new();
    for f in &files {
        let code = strip_comments_and_strings(&std::fs::read_to_string(f).unwrap());
        problems.extend(violations(&code).into_iter().map(|v| format!("{}: {v}", f.display())));
    }
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n"));
}

/// Every violation in already-stripped code.
fn violations(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let words: Vec<&str> = code
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .collect();
    for (tok, why) in BANNED_TOKENS {
        if words.contains(&tok) {
            out.push(format!("`{tok}`: {why}"));
        }
    }
    if code.replace(' ', "").contains("std::time") {
        out.push("`std::time`: sim has no clock; the page owns time".into());
    }
    // A float literal: digits, a dot, a digit — but not a range (`0..9`) and
    // not a tuple field (`a.0.1`), which follows an identifier or a dot.
    let b = code.as_bytes();
    for i in 1..b.len().saturating_sub(1) {
        if b[i] == b'.' && b[i - 1].is_ascii_digit() && b[i + 1].is_ascii_digit() {
            let mut s = i - 1;
            while s > 0 && (b[s - 1].is_ascii_digit() || b[s - 1] == b'_') {
                s -= 1;
            }
            let before = if s == 0 { b' ' } else { b[s - 1] };
            if !(before.is_ascii_alphanumeric() || before == b'.' || before == b'_') {
                let end = (i + 6).min(code.len());
                out.push(format!("a float literal near `{}`", &code[s..end]));
            }
        }
    }
    out
}

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// Comments become spaces and string or char literals become `""`, so what is
/// left is code. Raw strings are handled; nested block comments are counted.
fn strip_comments_and_strings(src: &str) -> String {
    let b: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let next = b.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 1;
            i += 2;
            while i < b.len() && depth > 0 {
                if b[i] == '/' && b.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if b[i] == '*' && b.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            out.push(' ');
        } else if c == 'r' && (next == Some('"') || next == Some('#'))
            && (i == 0 || !(b[i - 1].is_alphanumeric() || b[i - 1] == '_'))
        {
            let mut j = i + 1;
            let mut hashes = 0;
            while j < b.len() && b[j] == '#' {
                hashes += 1;
                j += 1;
            }
            if b.get(j) == Some(&'"') {
                j += 1;
                loop {
                    if j >= b.len() {
                        break;
                    }
                    if b[j] == '"' && (0..hashes).all(|k| b.get(j + 1 + k) == Some(&'#')) {
                        j += 1 + hashes;
                        break;
                    }
                    j += 1;
                }
                out.push_str("\"\"");
                i = j;
            } else {
                out.push(c);
                i += 1;
            }
        } else if c == '"' {
            i += 1;
            while i < b.len() && b[i] != '"' {
                if b[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            out.push_str("\"\"");
        } else if c == '\'' && b.get(i + 2) == Some(&'\'') {
            out.push_str("' '");
            i += 3;
        } else if c == '\'' && next == Some('\\') {
            let mut j = i + 2;
            while j < b.len() && b[j] != '\'' {
                j += 1;
            }
            out.push_str("' '");
            i = j + 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

#[test]
fn the_source_scan_sees_what_it_claims_to() {
    // The scanner stands between `sim` and a silently passing boundary test,
    // so it gets its own check, in the manner of Floodline's
    // `the_manifest_parser_reads_what_it_claims_to`.
    let caught = |s: &str| !violations(&strip_comments_and_strings(s)).is_empty();
    assert!(caught("let x: f32 = 0;"), "a float type");
    assert!(caught("let x = 1.5;"), "a float literal");
    assert!(caught("use std::collections::HashMap;"), "a hash map");
    assert!(caught("let t = std :: time :: Instant::now();"), "a clock");
    assert!(!caught("// no f32 here\nlet x = 1;"), "a comment is not code");
    assert!(!caught("/* f64 */ let x = 2;"), "nor is a block comment");
    assert!(!caught(r#"let s = "f32 HashMap 1.5";"#), "nor is a string");
    assert!(!caught(r##"let s = r#"Instant"#;"##), "nor is a raw string");
    assert!(!caught("for i in 0..10 { let y = t.0.1; }"), "a range and a tuple field");
    assert!(!caught("let c = 'f';"), "a char literal");
}

/// Every dependency name in a manifest, from all three dependency tables.
/// Floodline's small parser: `sim` may not have dependencies, and neither may
/// the test that checks it.
fn dependency_names(manifest: &str) -> BTreeSet<&str> {
    let mut names = BTreeSet::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line.ends_with("dependencies]");
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            let name = key.trim().split('.').next().unwrap_or("").trim_matches('"');
            if !name.is_empty() {
                names.insert(name);
            }
        }
    }
    names
}

#[test]
fn the_manifest_parser_reads_what_it_claims_to() {
    let sample = r#"
[package]
name = "sim"

[dependencies]
serde.workspace = true
postcard = { version = "1" }
"quoted-key" = "1"
# commented = "1"

[dev-dependencies]
proptest = "1"

[target.'cfg(unix)'.dependencies]
libc = "0.2"

[features]
default = []
"#;
    let found = dependency_names(sample);
    assert!(found.contains("serde"), "a dotted workspace key is still a dependency");
    assert!(found.contains("postcard"));
    assert!(found.contains("quoted-key"), "quoted keys count");
    assert!(found.contains("proptest"), "dev-dependencies count too");
    assert!(found.contains("libc"), "target-gated dependencies count too");
    assert!(!found.contains("commented"), "comments do not");
    assert!(!found.contains("default"), "[features] is not a dependency table");
    assert!(!found.contains("name"), "neither is [package]");
}
