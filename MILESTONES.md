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
