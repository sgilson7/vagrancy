//! Matches outside the road (data/duels.json; Sam, 2026-10-08: Paul against
//! Feyd-Rautha, story mode's prize for its first chapter, the player as
//! Paul).

#[test]
fn the_arrakeen_match_opens_with_the_first_chapter_and_is_shielded_and_one_round() {
    let d = content::duels::duel("arrakeen").expect("data/duels.json has the arrakeen match");
    let mut s = content::save::fresh();
    assert!(!content::duels::open(&d, &s), "the match is open before the first chapter");
    s.story = 1;
    assert!(content::duels::open(&d, &s), "the first chapter did not open the match");
    let setup = content::duels::setup(1, sim::balance::DEFAULT_TUNING, &d);
    assert_eq!(setup.rounds_to_win, 1);
    for seat in 0..2 {
        let b = &setup.bodies[setup.seats[seat].unwrap().body as usize];
        assert!(b.shield.is_some(), "seat {seat} has no shield");
        assert!(b.sword.is_some(), "seat {seat} has no knife");
    }
    // Paul's crysknife is curved: its blade has points off the line from
    // butt to tip; the Emperor's knife is straight.
    let knife = |seat: usize| setup.bodies[setup.seats[seat].unwrap().body as usize].sword.clone().unwrap();
    assert!(!knife(0).extra.is_empty(), "Paul's knife is straight");
    assert!(knife(1).extra.is_empty(), "Feyd's knife is curved");
    // And the opponent's pilot is one the road knows.
    assert!(content::road::road().iter().any(|st| st.id == d.pilot), "no pilot {}", d.pilot);
}
