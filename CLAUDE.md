# CLAUDE.md — rules for working in this repo

Kept short on purpose. This file holds the rules and the commands. Why a thing is the way it is goes in `DECISIONS.md`; what was noticed while working goes in the milestone's notebook; what the next reader needs goes in `HANDOFF.md`. If something here is out of date, that is a bug in this file.

Read `PLANNING-BRIEF.md` first. Once `PLAN.md` exists and Sam has approved it, `PLAN.md` wins where the two disagree, and the divergence is a notebook row.

## Non-negotiable

Break one of these and the failure is silent and expensive.

- **`crates/sim` depends on `serde` and `postcard` only.** No `f32` or `f64`, no `HashMap` or `HashSet`, no `std::time`, no `rand`, no physics crate. One `Rng`, and it lives in `World`. `crates/sim/tests/boundary.rs` enforces this, reading both the manifest and the source; if it fails, fix that before anything else.
- **The world changes only through `World::step([Input; 2])`.** The shim, the net crate, the pilots and the page never mutate `World`.
- **`crates/wasm` decides nothing.** It moves bytes across the boundary. An `if` in the shim is a rule that belongs in `sim`.
- **The page draws numbers core sent it.** It may interpolate between two frames it was given. It never integrates, predicts, detects a contact, or keeps its own copy of a constant.
- **A swing adds momentum from nowhere, and that is the design.** Do not add a reaction force, a damping term or an energy correction to make the physics "right". Speed is bounded by the cap and the drag in `sim::balance` and by nothing else (PLAN.md §8 Q17). See `PLAN.md` D9 and test H5.
- **Only arm bits drive a joint motor.** The step bits ask the balance rule for a step; no key drives a leg, the torso or the head directly (PLAN.md §8 Q9).
- **Nothing drawn for ink, a cut or a fighter is red.** Every color is in `data/palette.json` and `crates/content/tests/palette.rs` checks it.
- **Every string a player reads is in `data/copy.en.json` and is used exactly as written.** A string the build needs and the file lacks is written with `TONE.md` open, marked `"review": "new"`, and listed for Sam. A string that looks wrong is reported, not repaired.
- **No audio file enters the repo or the build without a row in `LICENSES.md`.** Do not download, record or transcribe any commercial recording. See `PLANNING-BRIEF.md` 0.5.
- **Nothing from the show the game's mood is borrowed from**: no names, likenesses, titles, dialogue, plot or music. `crates/content/tests/copy.rs` holds the deny-list.
- **Changing what the simulation does means bumping `SIM_VERSION`** and re-recording the golden replays, in the same commit, and saying so in the message.
- **Adding a field to `World`, the replay or the save is a compile error until the encoding carries it.** Do not fix a destructure by adding `..`.
- **Never run `git push` or `make publish` on your own judgement.** Only an explicit ask from Sam changes this, and it is never inferred.
- **Nothing of ours runs anywhere but GitHub Pages.** No server, no signalling service, no database, no bundler, no npm step. If something seems to need one, stop and say so.
- **Do not start a milestone before the previous deploy gate is live and Sam has seen it.**

## Working style

- **Recon before building.** Measure against the running build, and put what you found in the commit message. A number that disagrees with the plan names which of the two is wrong.
- **Grep before you invent.** The reference repositories in `reference/` already solve most of what is not physics.
- **Every new test is broken once and watched failing before it is kept.** Restore from a copy you took yourself, never with `git checkout` on a file that has uncommitted work in it.
- **A check about a number reads the number from where it is decided.** A second copy of a constant goes stale.
- **A count comes from a command.** Test totals come from `packaging/count-tests.sh`; ladder results from `make ladder`.
- **Keep the notebook while the work happens**, in `SECOND-ORDER-M<n>.md`. One row the moment you notice something. See `PLANNING-BRIEF.md` Part E.
- **A reference you cannot find is a question for Sam, not a guess.**
- Test names are sentences. Comments say why. Commit subjects are sentences; bodies say why, name what was rejected, and quote the test count from the script. Plain and specific, no hype.
- One full suite per milestone, in the background; `cargo test --test <name>` while iterating.
- At every deploy gate, and when the context reaches about 700,000 tokens, write `HANDOFF.md`.

## Commands

- `make test` — the whole suite, native, no window and no network. Lockstep runs on `net::Loopback`.
- `make check` — a fast type-check.
- `make web` / `make serve` — the browser build into `dist/web/`, and a local static server.
- `make test-ui` — walk the gate in Chromium, Firefox and WebKit. Fails on a console error or a request that leaves the origin.
- `make ladder` — play the yardstick pilot against every opponent and write `analysis/ladder.md`.
- `packaging/count-tests.sh` — how many tests there are, counted so the answer is the same twice.
- `make publish` — Sam only.
