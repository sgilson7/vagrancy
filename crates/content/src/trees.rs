//! Every opponent's behavior tree with its labels filled from the copy, as
//! one JSON text: what `lab trees` writes for analysis/trees/render.py, and
//! what the test that the drawn figures are current hashes.

use serde_json::{json, Value};

fn text(copy: &Value, l: &pilot::view::Label) -> String {
    let mut s = l.key.split('.').fold(copy, |o, k| &o[k]).as_str().unwrap_or_default().to_string();
    for (k, v) in &l.vars {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

fn node(copy: &Value, n: &pilot::view::Node) -> Value {
    json!({
        "id": n.id,
        "kind": n.kind,
        "icon": n.icon,
        "interrupt": n.interrupt,
        "text": text(copy, &n.label),
        "children": n.children.iter().map(|c| node(copy, c)).collect::<Vec<_>>(),
    })
}

/// Every pilot's tree, by id, labels filled, pretty-printed with sorted keys.
pub fn json() -> String {
    let copy = crate::copy::copy();
    let all: serde_json::Map<String, Value> =
        crate::road::pilots().into_iter().map(|(id, spec)| (id, node(&copy, &pilot::view::describe(&spec)))).collect();
    serde_json::to_string_pretty(&Value::Object(all)).unwrap() + "\n"
}

/// 64-bit FNV-1a, the same the world's checksum uses, written out so
/// render.py can compute it byte for byte.
pub fn fingerprint(bytes: &[u8]) -> String {
    format!("{:016x}", sim::world::fnv1a(bytes))
}
