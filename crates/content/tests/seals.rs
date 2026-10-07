//! Sam, 2026-10-07: each fight on the chart stands in a scene of its own
//! (analysis/art/seals.py), "like the drover is next to some cows".

use content::road::road;

#[test]
fn each_fight_on_the_road_has_its_scene_drawn() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/art/seal");
    let missing: Vec<String> = road().into_iter().map(|s| s.id).filter(|id| !std::path::Path::new(&format!("{dir}/{id}.png")).exists()).collect();
    assert!(missing.is_empty(), "no scene for {missing:?}; run python3 analysis/art/seals.py");
}
