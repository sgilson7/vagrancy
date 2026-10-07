//! Each move a tree can run has a name in the copy file. The BT Lab names
//! the running move on every tick, and a move with no string threw inside
//! its animation loop and froze each fight on the page (Sam, 2026-10-07:
//! "the views with the game will start to crash the more of them you load").

#[test]
fn each_move_a_tree_can_run_has_a_name_in_the_copy_file() {
    let c: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/copy.en.json")).unwrap()).unwrap();
    let missing: Vec<&str> = pilot::moves::MOVES.iter().copied().filter(|m| c["tree"]["act"][*m].as_str().is_none()).collect();
    assert!(missing.is_empty(), "moves with no tree.act string: {missing:?}");
}
