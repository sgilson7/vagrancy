//! The copy file: every string a player reads (PLANNING-BRIEF 0.6).
//!
//! The strings are implemented exactly as written. This module only walks the
//! file; it never changes a string.

use serde_json::Value;

/// The copy file, compiled in so the shim and the tests read the same bytes.
pub const COPY_JSON: &str = include_str!("../../../data/copy.en.json");

pub fn copy() -> Value {
    serde_json::from_str(COPY_JSON).expect("data/copy.en.json is valid JSON")
}

/// Every player-facing string as `(dotted.key, text)`, in file order.
///
/// Keys that begin with an underscore are instructions and are never shown,
/// so they are skipped at every depth.
pub fn player_strings(v: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    walk(v, String::new(), &mut out);
    out
}

fn walk(v: &Value, path: String, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) => {
            for (k, child) in m {
                if k.starts_with('_') {
                    continue;
                }
                let p = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                walk(child, p, out);
            }
        }
        Value::String(s) => out.push((path, s.clone())),
        _ => {}
    }
}

/// The placeholder names inside one string: `{a}` and `{key.b}`.
pub fn placeholders(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(i) = rest.find('{') {
        match rest[i..].find('}') {
            Some(j) => {
                out.push(&rest[i + 1..i + j]);
                rest = &rest[i + j + 1..];
            }
            None => break,
        }
    }
    out
}
