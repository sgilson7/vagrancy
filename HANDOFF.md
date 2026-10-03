# Handoff

Written for a reader with none of this session's context. It is rewritten at
every deploy gate.

## 1. What this is

Vagrancy is a two-player physics sword-fighting game for the browser. Each player drives only the shoulder and elbow of a sword arm, plus two slow-step keys. Everything in a fight is integer physics in `crates/sim`; a wasm shim carries the numbers to a canvas page that draws them. It ships as static files on GitHub Pages at https://sgilson7.github.io/vagrancy/, built by `.github/workflows/deploy.yml`. `PLANNING-BRIEF.md` says what is being made, and `PLAN.md` (approved 2026-10-03) says how; the plan wins where they disagree.

## 2. Load-bearing rules, and what breaks silently

- **No float, no `HashMap`, no clock in `sim`.** Two browsers disagree and the match desyncs a minute later. Guard: `crates/sim/tests/boundary.rs`.
- **Every string a player reads is in `data/copy.en.json`, exactly.** A paraphrase passes review and breaks the voice. Guard: `crates/content/tests/copy.rs`, and the gate's every-visible-line check.
- **Colors live only in `data/palette.json`, and none is red.** Guard: `crates/content/tests/palette.rs`.
- **A swing adds momentum from nowhere.** "Fixing" it kills the game's movement. Guard: H5, from M2.
- **The agent pushes only on Sam's explicit ask.** Sam gave that ask for this run (`PLAN.md`, Approval).

## 3. The shape of the code

- `crates/sim`: the fight. `crates/content`: data and copy.
- `crates/pilot`: opponents. `crates/net`: lockstep.
- `crates/wasm`: the shim. `crates/lab`: the bench.
- `web/`: the page. `packaging/`: the build. `testing/drive.py`: the gate.

## 4. The commands

`make test`, `make web`, `make test-ui`, `make count`. For the live gate: `ORIGIN=https://sgilson7.github.io/vagrancy .venv-test/bin/python testing/drive.py chromium firefox webkit`.

## 5. What will bite within the hour

- `index.html` must hold only `{{key}}` tokens.
- The wasm-bindgen CLI must be exactly the version pinned in `Cargo.lock` (0.2.127).
- A laptop build and the CI build produce different content hashes; compare commits.

## 6. Mistakes that cost time

- **Packaging:** a `{{key}}` inside an HTML comment was filled as a copy key.

## 7. The single next action

Build M1: `fx.rs` (done), `rng.rs`, the body, the solver, replay v1.
