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
