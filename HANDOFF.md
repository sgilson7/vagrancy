# Handoff

Written for a reader with none of this session's context, and rewritten at every deploy gate. This one is for the flanked-fights deploy (2026-10-05): ten fights with an opponent on each side of the player, ledges, and weapon and ground choices online.

## 1. What this is

Vagrancy is a two-player physics sword-fighting game for the browser, at https://sgilson7.github.io/vagrancy/. Each player drives the shoulder and elbow of a sword arm with four keys; two more keys run on rigid legs. Everything in a fight is integer physics in `crates/sim`: particles and sticks relaxed in a fixed order, a blade swept against every part, a cut that drops whatever lies beyond it. A wasm shim carries frames to a canvas page that draws them and decides nothing. There is a road of 59 opponents driven by pilots, laid out as a tree of fights: each opens when its requirements are met (`data/road.json`, DECISIONS.md "the road is a tree"). Ten of the fights are flanked: the player stands in the middle with an opponent on each side (seat 0 against seats 1 and 2), and some are fought on ledges from `data/maps.json`. They open on two newer requirements, a headshot and a round won untouched. Players carry one of six weapons won on the tree, Weapon Master style; the longsword is the enemies' (`data/weapons.json`, `analysis/weapons.md`). The road can be drawn as a tree, a chart or chapters. A tutorial, "Learn in missions", is a map of 42 missions ordered by a knowledge-component analysis of the game (`analysis/kc/RESULTS.md`, `data/kc_graph.json`, `data/tutorial.json`). versus at one keyboard, online play between two browsers with no server of ours, replays, and a save file. `PLANNING-BRIEF.md` is the brief; `PLAN.md` (approved 2026-10-03) is the plan, and it wins where the two disagree.

## 2. Load-bearing rules, and what breaks silently when each is broken

- **Integers only in `sim`, one `Rng`, no clock.** Two browsers stop agreeing and an online match stops on a desync. Guards: `crates/sim/tests/boundary.rs`; the gate's native-against-wasm checksum.
- **Every correction inside a fighter is an exact pair shift.** Momentum appears from nowhere where it should not, and H5 stops meaning anything. Guard: H5c, momentum exactly zero without a motor.
- **The arm motor pushes the arm and sword with no push back on the torso (D9).** "Fixing" it kills movement. Guard: H5.
- **`SIM_VERSION` goes up with any change to what the simulation does,** with the golden replay and `testing/replays/match.replay` re-recorded (`cargo run -p lab -- golden`, `-- fixture-match`). Otherwise old replays play a different match.
- **Strings come only from `data/copy.en.json`.** Guards: `crates/content/tests/copy.rs`; every gate screen checks that each visible line is a copy string.
- **Colors come only from `data/palette.json`, never in the red band.** Guards: `palette.rs`; the gate's canvas-pixel check.
- **A pilot returns an `Input`.** If it reaches into the world, it can do what a player cannot.

## 3. The shape of the code

| crate | holds |
|---|---|
| `sim` | `fx` (12-bit fixed point), `world` (solver, balance, motors, traction), `contact` (the sweep), `fight` (cuts, clash, ink, rounds), `replay`, `frame` |
| `content` | `data/*.json` into setups; `copy`, `messages` (which sentence for which outcome), `road` (the tree, requirements, conditions, companions, `Feats`, `record`, `lineup`), `maps` (ledges), `tutorial` (missions, goals, the `Tracker` that reads them from the world), `weapons` (reshaping the sword, unlocks), `save` (v6) |
| `pilot` | six kinds of opponent, `foe` (the nearest opponent still in the round), `duel` for two or three seats |
| `net` | `Session` (two-seat lockstep; the world has three seats and online uses two), `wire` (PROTO 5), `Loopback` |
| `wasm` | the shim: `Game`, `Road`, `Online` |
| `lab` | recon commands, `ladder`, `rate <matches> <stop>...` (for tuning; rebuild first, the pilots are compiled in), `golden`, `fixture-match`, `duel`, `trace` (not shipped) |

The page is `web/`: `app.js` (screens and the clock), `draw.js`, `keys.js`, `files.js`, `music.js`, `rtc.js`, `config.js`, `echo.html`. The gate is `testing/drive.py`; the online walk is `testing/online.py`.

## 4. The commands

- `make test`, `make web`, `make test-ui`, `make test-ui-online`, `make referee`, `make ladder`, `make count`.
- The live gate: `ORIGIN=https://sgilson7.github.io/vagrancy .venv-test/bin/python testing/drive.py chromium firefox webkit`.
- To feel it: `make serve`, then `?tuning=0|1|2`.

## 5. What will bite within the hour

- **Three seats.** `World::step([Input; 2])` is a duel; `step_all([Input; 3])` drives a flanked fight. A round ends when every fighter on a side is out; a fighter who is out goes limp and neither cuts nor is cut (SECOND-ORDER-M5 rows 48–50). A duel on open ground plays exactly as in SIM_VERSION 7.
- **The stand key in the air works only over a ledge.** Allowing it everywhere changed the tree pilots' duels (row 48).
- **A flanked match costs about three times a duel.** The ladder now takes over an hour; `lab rate` and `lab film` (records a match between two opponents, or the yardstick at a flanked stop, for showing) are the quick tools.

- `make weapons` takes about ten minutes; `the_sword_is_the_strongest_weapon` reads `analysis/weapons.md` and its fingerprint (weapons, pilots, the panel, `SIM_VERSION`).
- `make ladder` takes about half an hour for 33 fights. `the_tree_gets_no_easier_going_down` and the tier test read `analysis/ladder.md`, and fail if its fingerprint is stale: pilots, the set of stops with their conditions, and `SIM_VERSION`, but not the requirements.
- The knowledge-component analysis runs with Sam's kit, unmodified, from his GameAI-Fall26 directory (`analysis/kc/RESULTS.md` has the commands). It needs `networkx`, and `scipy` and `scikit-learn` for the stability figures; those live outside this repo.
- `hidden` must win over any `display` rule (`[hidden] { display: none !important; }`); twice a display rule beat it unseen.
- WebKit online play works on macOS, but not on CI's Linux runner, so CI's online walk runs Chromium and Firefox only.
- Replays play back in real time, so a long fixture makes a slow gate.
- zsh does not word-split `$var`; use `${=var}` in shell helpers.

## 6. Mistakes that cost time, by system

- **Physics:**
  - knees that folded, then feet that splayed (friction belongs in the projection);
  - a motor too weak to pogo, then a planted push that went into the ground (the blocked-joint drive);
  - a hinge that asked a piece for its fighter's facing.
- **Cuts:**
  - every fighter cutting itself (own blade now point-first only);
  - blades crossed in an X (a clash is now a block);
  - blades that then glued together.
- **Numbers:** an `as i32` cast that wrapped silently (`fx::narrow` now panics).
- **Packaging:** a stylesheet truncated by `open(p, "w")` before it was read.
- **Drawing:** ink marks drawn under the bodies, where no one saw them and no test could fail on them.
- **Pilots:**
  - a pose control that swung;
  - a yardstick killed by a scarecrow's still sword (the cut floor is now 6 cm/tick).

## 7. The single next action

Sam plays the deployed build. In particular:
- the three tunings (`?tuning=0|1|2`);
- self-cuts (`SECOND-ORDER-M3` row 2);
- the road's order against the ladder (`SECOND-ORDER-M5` rows 8 and 9);
- one online match across two networks (`SECOND-ORDER-M4` row 7).

Then file `PLAYTEST-M5.md`, with replays in `testing/replays/`.
