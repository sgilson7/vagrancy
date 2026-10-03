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
