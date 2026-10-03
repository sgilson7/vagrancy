//! Nothing drawn for ink, a cut or a fighter is red, and every color is
//! written in `data/palette.json` and nowhere else (D17, CLAUDE.md).
//!
//! The red band is a rule, so it lives here beside its check and not in the
//! palette file, where editing the band would be a way to pass. It is
//! recorded in DECISIONS.md: hue in [330°, 360°) or [0°, 20°] at a saturation
//! of 0.20 or more. A color near the band is Sam's call (PLANNING-BRIEF 0.4).

use serde_json::Value;
use std::path::Path;

const RED_FROM: u32 = 330;
const RED_TO: u32 = 20;
const MIN_SATURATION_PCT: u32 = 20;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// `#RRGGBB` to (hue in degrees, HSL saturation in percent), in integers.
fn hue_sat(hex: &str) -> Option<(u32, u32)> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let p = |i: usize| i64::from_str_radix(&h[i..i + 2], 16).ok();
    let (r, g, b) = (p(0)?, p(2)?, p(4)?);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let d = mx - mn;
    if d == 0 {
        return Some((0, 0));
    }
    // Hue in tenths of a degree, then rounded to degrees.
    let hue10 = if mx == r {
        (600 * (g - b) / d).rem_euclid(3600)
    } else if mx == g {
        600 * (b - r) / d + 1200
    } else {
        600 * (r - g) / d + 2400
    };
    // HSL saturation: d / (1 - |2L - 1|), with L = (mx + mn) / 2 on a 0..255 scale.
    let sum = mx + mn;
    let denom = if sum <= 255 { sum } else { 510 - sum };
    let sat = (100 * d / denom.max(1)) as u32;
    Some((((hue10 + 5) / 10) as u32 % 360, sat))
}

fn in_red_band(hex: &str) -> bool {
    match hue_sat(hex) {
        Some((h, s)) => s >= MIN_SATURATION_PCT && (h >= RED_FROM || h <= RED_TO),
        None => false,
    }
}

fn colors(v: &Value, path: String, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) => {
            for (k, c) in m {
                if !k.starts_with('_') {
                    colors(c, if path.is_empty() { k.clone() } else { format!("{path}.{k}") }, out);
                }
            }
        }
        Value::String(s) if s.starts_with('#') => out.push((path, s.clone())),
        _ => {}
    }
}

fn palette() -> Vec<(String, String)> {
    let text = std::fs::read_to_string(root().join("data/palette.json")).unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    let mut out = Vec::new();
    colors(&v, String::new(), &mut out);
    out
}

#[test]
fn no_ink_or_cut_color_is_red() {
    let all = palette();
    assert!(all.len() >= 10, "the palette has only {} colors; is it being read?", all.len());
    let red: Vec<_> = all.iter().filter(|(_, c)| in_red_band(c)).collect();
    assert!(red.is_empty(), "\nthese palette colors are in the red band: {red:?}\n");
    for (k, c) in &all {
        assert!(hue_sat(c).is_some(), "{k} = {c} is not #RRGGBB");
    }
}

#[test]
fn the_red_band_catches_red_and_spares_ochre() {
    for red in ["#FF0000", "#8B0000", "#DC143C", "#C0392B", "#E0115F", "#FF4500"] {
        assert!(in_red_band(red), "{red} should be in the band, hue/sat {:?}", hue_sat(red));
    }
    for fine in ["#3E4A89", "#B8862B", "#EFE8D8", "#808080", "#2B2A28"] {
        assert!(!in_red_band(fine), "{fine} should be clear, hue/sat {:?}", hue_sat(fine));
    }
}

#[test]
fn every_color_is_written_in_the_palette_file_and_nowhere_else() {
    let mut problems = Vec::new();
    for entry in std::fs::read_dir(root().join("web")).unwrap() {
        let p = entry.unwrap().path();
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !["js", "css", "html"].contains(&ext) {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        for (n, line) in text.lines().enumerate() {
            let has_hex = line.char_indices().any(|(i, c)| {
                c == '#'
                    && line[i + 1..].chars().take(6).filter(|c| c.is_ascii_hexdigit()).count() == 6
                    && line[i + 1..].len() >= 6
                    && !line[..i].ends_with('\'') && !line[..i].ends_with('"') // '#id' selectors
            });
            let has_fn = ["rgb(", "rgba(", "hsl(", "hsla("].iter().any(|f| line.contains(f));
            if has_hex || has_fn {
                problems.push(format!("{}:{}: {}", p.display(), n + 1, line.trim()));
            }
        }
    }
    assert!(problems.is_empty(), "\ncolors written outside data/palette.json:\n{}\n", problems.join("\n"));
}
