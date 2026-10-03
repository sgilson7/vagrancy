# Milestones

The status table, appended after every milestone (PLANNING-BRIEF, "The status table"). `tests` is from `packaging/count-tests.sh`.

## After M0 (2026-10-03)

| # | milestone | deliverables | tests | commit | state |
|---|---|---|---|---|---|
| M0 | Foundation, and the voice in the repo | workspace, profiles, Makefile, packaging, Pages workflow, entry page, lints | 15 (+15) | 58d51d8 | done; gate 1 live at build 7048c165 |
| M1 | A body in integers | | | | in progress |
| notebook | rows open / done / the human's | 0 / 6 / 2 | | | |
| new strings | awaiting Sam | 0 | | | |

Reliance (PLAN.md §1), revisited: the workspace and packaging row stays **high**. Two surprises, both caught by the build in seconds: a `{{key}}` in a comment, and `rev-parse` with no commits.

## After M2 (2026-10-03)

| # | milestone | deliverables | tests | commit | state |
|---|---|---|---|---|---|
| M0 | Foundation, and the voice in the repo | workspace, packaging, Pages, lints | 15 (+15) | 58d51d8 | done; gate 1 live |
| M1 | A body in integers, and a match you can play back | Fx, isqrt, sine, Rng, solver, balance, motors, replay v1, shim, page | 39 (+24) | 4826e96 | done; gate 2 live at e6b71e83 |
| M2 | The sword is solid, and a swing is free | servo motor and blocked drive, cap hold, H5/H5c, pogo, practice yard, music slot | see commit | this commit | done once gate 3 is live |
| M3 | Cuts, ink, and two fighters at one keyboard | | | | next |
| notebook | rows open / done / the human's | 0 / 24 / 7 | | | |
| new strings | awaiting Sam | 0 | | | |

Reliance, revisited: "jointed bodies in integers" stays **low**. Four first answers were wrong in M1 and M2: the knees, the feet, the motor's strength, and D9's planted push. Each was caught by a `lab` command before it reached the page.

## After M3 (2026-10-03)

| # | milestone | deliverables | tests | commit | state |
|---|---|---|---|---|---|
| M0 | Foundation, and the voice in the repo | workspace, packaging, Pages, lints | 15 (+15) | 58d51d8 | done; gate 1 live |
| M1 | A body in integers, and a match you can play back | Fx, solver, balance, motors, replay v1, page | 39 (+24) | 4826e96 | done; gate 2 live |
| M2 | The sword is solid, and a swing is free | servo motor, blocked drive, H5/H5c, pogo, yard, music | 45 (+6) | c496085 | done; gate 3 live at b96aceb1 |
| M3 | Cuts, ink, and two fighters at one keyboard | swept contact (H2, H2b), cuts and pieces (H3), stumps and ink (H4), fatal zones, rounds, match, draw, clash, events, results, HUD, versus, key bindings | 60 (+15) | this commit | done once gate 4 is live |
| M4 | Lockstep | | | | next |
| notebook | rows open / done / the human's | 2 / 32 / 11 | | | |
| new strings | awaiting Sam | 1 | | | |

Reliance, revisited: "a fast blade meeting a limb" and "cutting a body" stay **low**. The sweep and the cut passed their hand cases on the first run. The world-level consequences (self-cuts everywhere, a hinge in a piece, blades that glued) were each found only by measuring, and none was in the brief's list of traps.

## After M4 (2026-10-03)

| # | milestone | deliverables | tests | commit | state |
|---|---|---|---|---|---|
| M0–M2 | foundation; a body; a free swing | (see above) | 45 | c496085 | done; gates 1–3 live |
| M3 | Cuts, ink, and two fighters at one keyboard | cuts, ink, rounds, match, clash, results, versus, key bindings | 60 (+15) | 7d25e6d | done; gate 4 live at 4f3289f4 |
| M4 | Lockstep, on a loopback and then between two browsers | net::Session (measured delay, Welcome with Setup, both-sides desync stop, silence drop), Loopback with jitter, rtc.js (room codes and pasted codes), vendored Trystero, online lobby and match, echo.html, online.py, the referee | 68 (+8) | this commit | done once gate 5 is live; the two-network match is Sam's |
| M5 | The road | | | | next |
| notebook | rows open / done / the human's | 2 / 39 / 14 | | | |
| new strings | awaiting Sam | 1 | | | |

Reliance, revisited: "lockstep for two" was **medium-high for the port, low for the new parts**. The new parts passed their tests on the first run; the surprises were in the browser (Firefox's mDNS on loopback) and in my own arithmetic (row 4).

## After M5 (2026-10-03): MVP

| # | milestone | deliverables | tests | commit | state |
|---|---|---|---|---|---|
| M0–M3 | foundation; a body; a free swing; cuts and a match | (see above) | 60 | 7d25e6d | done; gates 1–4 live |
| M4 | Lockstep, on a loopback and then between two browsers | Session, Loopback, rtc.js, online lobby, referee | 68 (+8) | 12baf50 | done; gate 5 live at 1c26c050 |
| M5 | The road (MVP complete) | six pilot kinds and the yardstick, pilots.json, road.json in ladder order, make ladder, the road screen, introductions filled from pilot data, save v1 with the labeled convenience copy | 77 (+9) | this commit | done once gate 6 is live |
| notebook | rows open / done / the human's | 3 / 47 / 19 | | | |
| new strings | awaiting Sam | 1 | | | |

### The MVP checklist (PLANNING-BRIEF 0.3), walked

1. **Four keys drive the shoulder and elbow of the sword arm, and no key drives any other joint.** Yes. `only_arm_bits_drive_a_joint_motor`. The step keys move a fighter only through the ground (`the_step_bits_move_a_fighter_only_through_the_ground`; Q9).
2. **The sword is solid against the ground and the other sword; a planted sword lifts the fighter; a swing in the air adds momentum; both are tests with numbers.** Yes. `a_planted_sword_lifts_the_fighter` (591 cm measured); H5 (140 mass·cm/tick measured); `two_blades_never_pass_through_each_other`; `a_blade_never_ends_a_tick_below_the_ground`.
3. **A blade cuts a part where it touched, and what lies beyond becomes a separate falling object.** Yes. H2, H2b, H3.
4. **A round ends on a neck or head cut, a heart cut, or zero ink, and the result says which.** Yes. The round-end tests and `the_result_names_the_cut_that_ended_the_round`; the gate shows it.
5. **Nothing drawn for ink, cuts or fighters is red, and no string breaks `TONE.md`; both are lints.** Yes. `palette.rs`, `copy.rs`, and the gate's canvas-pixel check.
6. **A road of opponents ordered against one yardstick, and a save file to download and load.** Yes. `the_road_is_ordered_by_the_yardstick` against `analysis/ladder.md` (200 matches a stop); the save checks in the gate.
7. **Versus at one keyboard, and between two browsers on two networks with no server of Sam's.** One keyboard: yes. Two browsers: yes, two tabs in three engines and a three-minute referee match. **Two networks: not walked by the agent.** That needs Sam and a friend (SECOND-ORDER-M4 row 7).
8. **Any match downloads as a replay and plays back to the same final checksum in Chromium, Firefox and WebKit.** Yes. The gate, every deploy, both locally and live.
9. **A music track from the player's own device loops during fights; no audio ships without a `LICENSES.md` row.** Yes. The gate's music check; `licenses.rs` and the packaging guard.
10. **Static files on GitHub Pages, in its own repository.** Yes. `sgilson7/vagrancy`, deployed by Actions.
