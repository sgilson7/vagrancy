//! `data/body.json` into `sim::body::BodyDef`.
//!
//! The file names points; `sim` wants indices. The file gives a rest pose;
//! the hinges' closest approach and the grips' places along the sword are
//! worked out here, once, with `sim`'s own integer helpers.

use serde::Deserialize;
use sim::body::{BodyDef, Cause, FatalBand, Grip, Hinge, Motor, PartDef, PointDef, Roles, SwordDef};
use sim::fx::{cos_deg, isqrt, Fx, V2, ONE};
use std::collections::BTreeMap;

pub const BODY_JSON: &str = include_str!("../../../data/body.json");

#[derive(Deserialize)]
struct File {
    fighter: BodyJson,
    post: BodyJson,
}

#[derive(Deserialize)]
struct BodyJson {
    ink: i32,
    points: BTreeMap<String, [i32; 3]>,
    parts: Vec<PartJson>,
    sticks: Vec<[String; 2]>,
    hinges: Vec<HingeJson>,
    roles: RolesJson,
    #[serde(default)]
    anchored: Vec<String>,
    balance: bool,
    sword: Option<SwordJson>,
}

#[derive(Deserialize)]
struct PartJson {
    #[allow(dead_code)]
    id: String,
    copy: String,
    near: String,
    far: String,
    radius: i32,
    mass: i32,
    drain: i32,
    #[serde(default)]
    fatal: Vec<FatalJson>,
    motor: Option<String>,
    #[serde(default)]
    hand: bool,
}

#[derive(Deserialize)]
struct FatalJson {
    from: f64,
    to: f64,
    cause: String,
}

#[derive(Deserialize)]
struct HingeJson {
    a: String,
    j: String,
    c: String,
    sign: i8,
    max_bend: i32,
}

#[derive(Deserialize)]
struct RolesJson {
    shoulder: Option<String>,
    pelvis: Option<String>,
    head: Option<String>,
    feet: Vec<String>,
}

#[derive(Deserialize)]
struct SwordJson {
    butt: [i32; 2],
    tip: [i32; 2],
    hilt: i32,
    mass: i32,
    grips: Vec<GripJson>,
}

#[derive(Deserialize)]
struct GripJson {
    hand: String,
    stiff: Option<String>,
}

/// A fraction written in the data file as a decimal, to the nearest
/// 1/4096. Data is read once, here; `sim` never sees a float.
fn frac(x: f64) -> Fx {
    Fx((x * ONE.0 as f64).round() as i32)
}

fn build(b: BodyJson) -> BodyDef {
    // Points are numbered in name order, so the numbering does not depend on
    // how the JSON object happened to be written out.
    let names: Vec<&String> = b.points.keys().collect();
    let ix = |n: &str| -> u8 { names.iter().position(|x| *x == n).unwrap_or_else(|| panic!("no point named {n}")) as u8 };
    let at = |n: &str| -> V2 {
        let p = b.points[n];
        V2::cm(p[0], p[1])
    };
    let points = names.iter().map(|n| PointDef { at: at(n), rad: Fx::int(b.points[*n][2]) }).collect();
    let parts = b
        .parts
        .iter()
        .map(|p| PartDef {
            copy: p.copy.clone(),
            near: ix(&p.near),
            far: ix(&p.far),
            radius: Fx::int(p.radius),
            mass: p.mass,
            drain: p.drain,
            fatal: p
                .fatal
                .iter()
                .map(|f| FatalBand {
                    from: frac(f.from),
                    to: frac(f.to),
                    cause: match f.cause.as_str() {
                        "neck" => Cause::Neck,
                        "heart" => Cause::Heart,
                        c => panic!("unknown fatal cause {c}"),
                    },
                })
                .collect(),
            motor: p.motor.as_deref().map(|m| match m {
                "shoulder" => Motor::Shoulder,
                "elbow" => Motor::Elbow,
                m => panic!("unknown motor {m}"),
            }),
            hand: p.hand,
        })
        .collect();
    let hinges = b
        .hinges
        .iter()
        .map(|h| {
            // Law of cosines at the hinge's largest bend: the interior angle
            // is 180° − max_bend, and d² = l1² + l2² − 2·l1·l2·cos(interior).
            let l1 = (at(&h.j) - at(&h.a)).len().0 as i64;
            let l2 = (at(&h.c) - at(&h.j)).len().0 as i64;
            let cos = cos_deg(180 - h.max_bend).0 as i64;
            let d2 = l1 * l1 + l2 * l2 - 2 * l1 * l2 * cos / ONE.0 as i64;
            Hinge { a: ix(&h.a), j: ix(&h.j), c: ix(&h.c), sign: h.sign, min_dist: Fx(isqrt(d2.max(0) as u64) as i32) }
        })
        .collect();
    let sword = b.sword.as_ref().map(|s| {
        let butt = V2::cm(s.butt[0], s.butt[1]);
        let tip = V2::cm(s.tip[0], s.tip[1]);
        let axis = tip - butt;
        SwordDef {
            butt,
            tip,
            hilt: Fx::int(s.hilt),
            mass: s.mass,
            grips: s
                .grips
                .iter()
                .map(|g| Grip {
                    hand: ix(&g.hand),
                    // Where the hand sits along the sword, as a fraction.
                    at: Fx((((at(&g.hand) - butt).dot_raw(axis) << 12) / axis.len_sq_raw()) as i32),
                    stiff: g.stiff.as_deref().map(ix),
                })
                .collect(),
            // A plain sword; data/weapons.json reshapes it (crate::weapons).
            extra: Vec::new(),
            edges: Vec::new(),
        }
    });
    BodyDef {
        points,
        parts,
        sticks: b.sticks.iter().map(|[a, c]| (ix(a), ix(c))).collect(),
        hinges,
        roles: Roles {
            shoulder: b.roles.shoulder.as_deref().map(ix),
            pelvis: b.roles.pelvis.as_deref().map(ix),
            head: b.roles.head.as_deref().map(ix),
            feet: b.roles.feet.iter().map(|f| ix(f)).collect(),
        },
        anchored: b.anchored.iter().map(|a| ix(a)).collect(),
        balance: b.balance,
        ink: b.ink,
        sword,
    }
}

/// The fighter's body and the post's, in that order: `Seat::body` 0 and 1.
pub fn bodies() -> Vec<BodyDef> {
    let f: File = serde_json::from_str(BODY_JSON).expect("data/body.json is valid");
    vec![build(f.fighter), build(f.post)]
}
