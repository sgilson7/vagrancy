//! `data/costumes.json`: what the road's opponents wear (Sam, 2026-10-08:
//! "some way to add costumes / art to the mesh for the character ... for
//! some of these enemies, like ones with plated helmets and stuff, it should
//! act like a sword in the sense that its rigid, and should defend the area
//! beneath it from getting cut").
//!
//! Two switches in the file turn the change off without touching code:
//! `art` (the pictures the page draws) and `armor` (the plates the
//! simulation holds). With both off every fight is as it was.

use serde::{Deserialize, Serialize};
use sim::body::{BodyDef, PlateDef};
use sim::fx::V2;
use std::collections::BTreeMap;

pub const COSTUMES_JSON: &str = include_str!("../../../data/costumes.json");

#[derive(Deserialize)]
pub struct File {
    pub art: bool,
    pub armor: bool,
    pub plates: BTreeMap<String, PlateJson>,
    pub slots: BTreeMap<String, SlotJson>,
    pub costumes: BTreeMap<String, Costume>,
}

#[derive(Deserialize)]
pub struct PlateJson {
    pub part: String,
    pub points: Vec<[i32; 2]>,
}

#[derive(Deserialize)]
pub struct SlotJson {
    pub part: String,
    pub r#box: [i32; 4],
}

#[derive(Clone, Deserialize)]
pub struct Costume {
    pub head: String,
    pub chest: String,
    pub waist: String,
    pub main: String,
    pub trim: String,
    #[serde(default)]
    pub armor: Vec<String>,
    #[serde(default)]
    pub glow: bool,
    /// The opponent's own signature piece (analysis/art/signatures.py), and
    /// the slot whose picture it is drawn into.
    pub sig: String,
    pub sig_on: String,
}

pub fn file() -> File {
    serde_json::from_str(COSTUMES_JSON).expect("data/costumes.json is valid")
}

/// What of data/costumes.json changes a fight: the armor switch, the plates,
/// and who wears which. The measured tables' fingerprints read this, so a
/// change to the pictures alone leaves them current.
pub fn armor_text() -> String {
    let f = file();
    let plates: Vec<String> = f.plates.iter().map(|(n, p)| format!("{n}:{}:{:?}", p.part, p.points)).collect();
    let worn: Vec<String> = f.costumes.iter().filter(|(_, c)| !c.armor.is_empty()).map(|(id, c)| format!("{id}:{}", c.armor.join("+"))).collect();
    format!("armor={};{};{}", f.armor, plates.join(","), worn.join(","))
}

/// The costume opponent `id` wears, if the file gives it one.
pub fn costume(id: &str) -> Option<Costume> {
    file().costumes.get(id).cloned()
}

/// The plates of armor opponent `id` wears on `body`, held to the ends of
/// the parts the file names. Empty when the armor switch is off.
pub fn plates(id: &str, body: &BodyDef) -> Vec<PlateDef> {
    let f = file();
    if !f.armor {
        return Vec::new();
    }
    let Some(c) = f.costumes.get(id) else { return Vec::new() };
    c.armor
        .iter()
        .map(|name| {
            let p = f.plates.get(name).unwrap_or_else(|| panic!("costume {id}: no plate named {name}"));
            let k = crate::body::part_index(&p.part).unwrap_or_else(|| panic!("plate {name}: no part named {}", p.part)) as usize;
            let part = &body.parts[k];
            PlateDef { points: p.points.iter().map(|&[x, y]| V2::cm(x, y)).collect(), near: part.near, far: part.far }
        })
        .collect()
}

/// Dress opponent `id`'s body: its costume's name, for the page to draw
/// (while the file's art switch is on) and to find its background by, and
/// its plates while the armor switch is on.
pub fn dress(id: &str, body: &mut BodyDef) {
    let f = file();
    // The name stays even with the art off: the page finds the fight's
    // background by it, and the art switch reaches the page in page_json.
    if f.costumes.contains_key(id) {
        body.costume = id.to_string();
    }
    body.armor = plates(id, body);
}

/// Whether the file dresses opponent `id` at all.
pub fn dressed(id: &str) -> bool {
    let f = file();
    f.costumes.contains_key(id)
}

/// What the page needs to draw costumes, as JSON: the art switch; for each
/// slot, the part it rides on (by index into the fighter's parts) and that
/// part's ends and the picture's box in the rest pose, in cm; which parts
/// are drawn under the clothes (the trunk and legs; arms go over them); and
/// for each costume, its slots that have a picture and whether it glows.
pub fn page_json() -> String {
    #[derive(Serialize)]
    struct Slot {
        part: u8,
        near: [i32; 2],
        far: [i32; 2],
        r#box: [i32; 4],
    }
    #[derive(Serialize)]
    struct Worn {
        slots: Vec<String>,
        glow: bool,
    }
    #[derive(Serialize)]
    struct Page {
        art: bool,
        slots: BTreeMap<String, Slot>,
        under: Vec<u8>,
        costumes: BTreeMap<String, Worn>,
    }
    let f = file();
    let body = &crate::body::bodies()[0];
    let cm = |v: V2| [v.x.0 >> sim::fx::FRAC_BITS, v.y.0 >> sim::fx::FRAC_BITS];
    let slots = f
        .slots
        .iter()
        .map(|(name, s)| {
            let k = crate::body::part_index(&s.part).unwrap_or_else(|| panic!("slot {name}: no part named {}", s.part));
            let p = &body.parts[k as usize];
            let at = |i: u8| cm(body.points[i as usize].at);
            (name.clone(), Slot { part: k, near: at(p.near), far: at(p.far), r#box: s.r#box })
        })
        .collect();
    let under = ["chest", "neck", "head", "waist", "thigh_back", "shin_back", "thigh_front", "shin_front"].iter().filter_map(|id| crate::body::part_index(id)).collect();
    let costumes = f
        .costumes
        .iter()
        .map(|(id, c)| {
            let slots = [("head", &c.head), ("chest", &c.chest), ("waist", &c.waist)].iter().filter(|(s, piece)| piece.as_str() != "none" || c.sig_on == *s).map(|(s, _)| s.to_string()).collect();
            (id.clone(), Worn { slots, glow: c.glow })
        })
        .collect();
    serde_json::to_string(&Page { art: f.art, slots, under, costumes }).expect("costumes encode")
}
