# Vagrancy: the plan

`B2 Require step-by-step justification`

This is the agent's Part A deliverable for `PLANNING-BRIEF.md`. It contains no game code, and none will be written until Sam approves it. It follows the shape in brief A.5. Sam is asked to read section 7 first (brief Part J).

**How claims are sourced.** A claim about a source names a file and line, read at the commits below. A number names the command that produced it, or is marked *(guess)*. Every departure from a recommendation says so in its first sentence.

**What was read, and how.**

| Source | Where | Commit |
|---|---|---|
| `sgilson7/floodline` | `~/Documents/Floodline` (clean) | `e18601b` |
| `sgilson7/gear-master-2d` | `~/Documents/GearMaster2D` (clean) | `23a135e` |
| `sgilson7/gear-master` | `~/Documents/GearMaster`. It has 15 untracked `analysis/nets/*.log` files and no modified tracked files (`git status --porcelain`), so the code read is the commit. | `1deaa69` |
| The CSC 484 guest lecture | `~/Documents/9_17 Lecture Applications of AI Literacy-2-2.pptx`, 55 slides | n/a |
| The website packet | `~/Documents/website-rework/runtime-copy.json` | n/a |
| The packet | `PLANNING-BRIEF.md`, `CLAUDE.md`, `TONE.md`, `HOUSE-STYLE.md`, `data/copy.en.json` | n/a |

The local copies are at the same commits that `HOUSE-STYLE.md` lists. The agent also cloned the three repositories into `vagrancy-packet/reference/` (`git clone`, same commits) before Sam pointed it at the local copies. The two sets are identical, and `reference/` will be gitignored.

Nothing was compiled and no test was run. Every number about the game itself is a guess until the recon of the milestone it belongs to.

**Toolchain present** (`rustup target list --installed`, `wasm-bindgen --version`, `ls ~/Library/Caches/ms-playwright`):
- the `wasm32-unknown-unknown` target;
- `wasm-bindgen` 0.2.127;
- Playwright browser builds for Chromium, Firefox and WebKit.

The Python `playwright` package is **not** installed (`python3 -c "import playwright"` → `ModuleNotFoundError`). `make test-ui-setup` will install it into a venv.

---

## 1. Reliance, calibrated

`A0 Calibrate your reliance`

The lecture's claim is on slide 15: "Reliability tracks how common the problem is … The rare one gets filled in from the common one." The last column is the agent's. It is revisited at the end of every milestone and the changes go in `MILESTONES.md`.

| Part of the game | How common | Filled in from | Small case that decides | How far the agent trusts its first answer, and why |
|---|---|---|---|---|
| Workspace, shim, packaging, CI | Common, house code exists | (none) | The page prints its build hash | **High.** Every piece exists in a repository the agent has read. The risk is a merge risk: Gear Master 2D's `package-web.sh` and Floodline's differ. GM2D's has the stamping (`:62-111`); Floodline's has the vendor sha256 check (`:20-23`). The new script needs both, and neither has the audio guard. |
| Lockstep for two | Uncommon; Floodline is the reference | Rollback; an authoritative host | Two worlds, 10,000 ticks, equal checksums | **Medium-high for the port, low for the new parts.** Three parts are new: a delay measured from a round trip, a delay sent in `Welcome`, and jitter in `Loopback`. Floodline has none of them (section 7, items 5 and 6). The tick rate also triples, from 20 to 60. |
| Jointed bodies in integers | Rare | `f32`, `sqrt()`, `sin()`, Rapier, Box2D | H1, H6 | **Low.** Jakobsen's paper is written in floats, so every formula is a translation and each gets a test. A second pull: position-based solvers drive a joint with a constraint, and constraint corrections are mass-weighted on both ends, which conserves momentum. The textbook can enter through the solver itself (see H5). |
| A fast blade meeting a limb | Rare | Checking positions at the end of a tick | H2, plus H2b, which the agent adds | **Low.** H2 as written is passed by two substeps, so it cannot tell a real sweep from a lucky one (section 7, item 19). |
| Cutting a body into two objects | Rare | Hiding a sprite; breaking a ragdoll joint | H3 | **Low.** "Away from the heart" is a fact about the body tree, not about distance (D11). |
| Ink | Looks common | Summing every cut ever made | H4 | **Medium.** The rule is simple, but `results.road.lose_ink` names "the cut … [that] spilled the most", so core must also keep a per-part total that does survive replacement. That is the trap set beside the trap. |
| **A swing adds speed from nowhere** | Perturbed | Newton's third law; conservation; damping "for stability" | H5 | **Low, on purpose.** The motor is the one place momentum must not be conserved. Every other internal rule must conserve it, or H5 passes for the wrong reason. The agent adds a control case, H5c. |
| **Arms only** | Perturbed | Full-body QWOP | `only_arm_bits_drive_a_joint_motor` | **Medium.** The brief disagrees with itself here: step keys move the legs (section 7, item 18). |
| **Ink is not red** | Perturbed | Red | The palette lint, and a pixel check in the gate | **High for the palette file, medium for the pixels.** Antialiasing and alpha blending make colors that are not in the file, so the gate also samples the canvas after a scripted cut. Ochre sits near the red range and needs a stated margin. |
| Opponents | Rare | "Train a network" | The ladder table; search first | **Low for quality, medium for structure.** The brief says (Part J) that whether a pilot can drive an arm key by key is unknown. |
| Strings | Common, in the wrong register | Game marketing voice | The `TONE.md` lints | **High for implementing them as written; medium for new ones.** A scan of all 206 strings against rules 3, 4, 8, 10, 12, 14 and 16 and the glossary found two hits (the script was run inline; it becomes `copy.rs`). The digit in `settings.music.formats` ("MP3") is allow-listed. The "you" in `local.keyboard_limit` breaks rule 3 (section 8, Q10). |
| Whether it is fun | Not for the agent | | Sam plays | **None.** |

---

## 2. Decisions D1 to D18

Each decision is followed by its reason and what was rejected. Where the brief marks a decision **Decided by Sam (0.7)**, it is restated and built on, not reopened.

### D1 — Crates and reuse

**Decision: as recommended, with three adjustments.** The crates are:

- `crates/sim`: `serde` and `postcard` only;
- `crates/content`: `serde_json` is allowed;
- `crates/pilot`;
- `crates/net`: `sim`, `serde`, `postcard`;
- `crates/wasm`;
- `crates/lab`: not shipped.

Start points:
- `fx.rs`, `rng.rs`, the checksum, `boundary.rs`, `determinism.rs` and `net` come from Floodline;
- packaging, `count-tests.sh`, `drive.py`, the workflow and the Makefile come from Gear Master 2D;
- the vendor sha256 check comes from Floodline's `packaging/package-web.sh:20-23`.

The adjustments:

1. **The workspace-level tests have homes in crates.** A virtual workspace has no root `tests/` (section 7, item 21). `boundary.rs` goes in `crates/sim/tests/`. `copy.rs` and `palette.rs` go in `crates/content/tests/`, because `content` owns `data/*.json` and may parse it.
2. **The file codecs live where their dependencies are allowed.** Replay v1 is `postcard` and lives in `sim`. Save v1 is JSON and lives in `content` (D15).
3. **`crates/net-web` is not ported.** Floodline's `net-web` wraps `quad_rtc.js` for miniquad (`net-web/src/lib.rs:1-19`). With D2 the page owns the transport and hands bytes to the shim, so there is nothing for it to do.

*Why:* the house split has already been tested in practice, and copying the conventions is cheaper than re-deriving them. *Rejected:* one crate for `sim` and `net`, because Floodline keeps them apart so that "a networking regression and a lockstep regression can never be mistaken for one another" (`HOUSE-STYLE.md` §3).

### D2 — Drawing

**Decision: as recommended.** The page is Canvas 2D from a vanilla ES module, in three layers: core, shim, page (lecture slide 8, quoted in section 7, item 16). The page may interpolate between two frames it was given. It never extrapolates, integrates, detects a contact or keeps its own copy of a constant.

Constants the page needs, such as the tick rate and `rounds_to_win`, come from a `numbers()` call on the shim. That call serializes them from `sim::balance`.

*Why:* the gate in three engines assumes a DOM. Floodline's `quad_rtc.js` is a miniquad plugin (`quad_rtc.js:786`), and making it a plain module removes the glue. *Rejected:* macroquad, for both reasons.

### D3 — Numbers

**Decision: as recommended, with one deviation in rounding.** Numbers are `i32` fixed point with 12 fractional bits, and one unit is a centimeter. Every product and quotient goes through `i64`, **including `Fx * i32`**. Floodline's `Mul<i32>` does not widen (`fx.rs:169-170`). Overflow checks are on in release, as in Floodline's `Cargo.toml:36-43`.

**The deviation: `Fx` multiply and divide round toward zero, and `>>` on raw values floors.** Floodline's `Fx*Fx` floors through `>>` but `Fx/Fx` truncates (`fx.rs:160`). Rounding toward zero is odd-symmetric (`trunc(-a) = -trunc(a)`), so a world mirrored left to right stays an exact mirror. That keeps the two seats exactly fair, and the replayer pilot mirrors a round (D16). Flooring would bias every left-moving product by up to one raw unit. H6's claim about `>>` is unchanged and is tested on `>>` itself.

The cost: truncation makes speed magnitudes drift slightly downward. That is a rounding effect, not a damping term. M1.0 measures it alongside the 8/12/16-bit drift.

Arithmetic for the range: `2^31 / 2^12` = 524,288 cm, about 5.2 km. The arena is about 1,200 cm *(guess)*. A squared length at 1,200 cm is `(1200·4096)² ≈ 2.4·10^13`, which fits `i64`. Its square root, `≈ 4.9·10^6`, fits `i32`. Gravity at 981 cm/s² and 60 ticks a second is `981 / 3600 = 0.2725` cm per tick², or 1,116 raw units. That is plenty of resolution.

*Rejected:* 8 bits as in Floodline (`fx.rs:25`). Floodline's bodies are points on a grid, while this game measures joint angles from short sticks. M1.0 decides.

### D4 — Physics model

**Decision: as recommended.** Particles are joined by rigid sticks and moved by position Verlet. Constraints are solved by a fixed number of relaxation passes in a fixed order. **The citation is verified:** Thomas Jakobsen, "Advanced Character Physics", Game Developers Conference 2001 (IO Interactive, used in *Hitman: Codename 47*), hosted at gamedeveloper.com/programming/advanced-character-physics (found by web search).

Consequences that are now decisions:
- **Velocity is implicit, as `x − x_prev`.** "Speed" in H1 means that difference, so a test sets `x_prev = x − 1`.
- **The speed cap clamps `|x − x_prev|`.** It needs `isqrt`, but only when the cheap squared comparison is over.
- **Joint limits are minimum-distance sticks**, as Jakobsen does for elbows, so no angle is needed.
- **The sine table is still built** (H6), for the data-side rest pose and for pilots. It is not used in the hot loop.

*Why no crate:* Floodline's reason at `floodline-design.md:143-147` applies word for word: Rapier "is floating point … breaks the moment a native headless peer joins".

### D5 — Time

**Decision: as recommended.** Sixty ticks a second, fixed. The page owns the clock, a `requestAnimationFrame` accumulator, and `sim` has none. `TICKS_PER_SECOND` lives in `sim::balance` and reaches the page through `numbers()`.

### D6 — Input

**Decision: as recommended for the byte. The test is renamed.** One byte per fighter per tick:
- bit 0, shoulder up; bit 1, shoulder down;
- bit 2, elbow in; bit 3, elbow out;
- bit 4, step left; bit 5, step right;
- bits 6 and 7 are spare, and must be zero, or the replay is refused.

`World::step([Input; 2])` is the only way the world changes. Default keys are in `data/controls.json`:
- one player: Q, W, O, P and A, D;
- one keyboard: Q, W, E, R, A, D against U, I, O, P, J, L.

The test the brief names, `only_arm_bits_move_a_joint`, contradicts Sam's 0.7 answer, because a step moves a leg. It becomes two tests:
- `only_arm_bits_drive_a_joint_motor`;
- `the_step_bits_move_the_legs_only_through_the_balance_rule`.

Section 8, Q9 asks Sam to approve this rewording of the `CLAUDE.md` line.

### D7 — Arms

**Decision: as recommended, with three things the brief left open, decided here and listed in section 8 for Sam:**

1. **Q14.** An opponent's blade can cut a hand. Only the fighter's own blade is exempt. `parts.hand` exists in the copy, and `the_hands_on_the_hilt_are_safe_from_their_own_blade` names only one's own.
2. **Q15.** A sword that no hand holds is solid but does not cut, so every cut has an owner to name in the result.
3. **Q16.** The motors live on joints. A joint whose far side has been cut away has no motor. If the lead upper arm is cut, the keys do nothing until the round ends.

The rear arm is simulated, undriven, and its hand is pinned to the hilt. The wrist is stiff: the hand-to-blade angle is held by a third stick.

### D8 — Legs

**Decided by Sam (0.7).** The legs are simulated and held upright by a balance rule the player does not control. Two keys take a slow step, and everything fast comes from the sword.

**Built on as follows. Every correction the balance rule and the step make is momentum-conserving:** equal and opposite, mass-weighted. It is the ground, through friction, that moves the fighter. This keeps H5 meaningful (D9). The balance rule stops when no foot remains.

The step length and duration are M1.0 numbers. The guess is 25 cm over 20 ticks, which crosses 1,200 cm in about 16 s *(guess)*.

### D9 — The free-energy rule

**Decided by Sam (0.7).** An arm motor pushes the arm and the sword and is not paid back by an equal push on the torso. A swing therefore adds momentum to the whole fighter.

**Built on as follows.** The motor displaces the particle beyond the joint perpendicular to the limb, at a strength from `sim::balance`. It applies **no** displacement to the near side. Everything else in the solver conserves momentum, so the motor is the only source, and H5 together with H5c can show that.

**A correction is needed.** The brief bounds speed "by a cap and by drag". `CLAUDE.md` says "Speed is bounded by the cap in `sim::balance` and by nothing else" and forbids "a damping term" (section 7, item 20).

The plan follows D9. Drag is one uniform constant in `sim::balance`, applied to every particle, and it is one of the three numbers M2.0 tunes. It is not a correction attached to the motor. Section 8, Q17 asks Sam to amend `CLAUDE.md` to say "the cap and the drag in `sim::balance`".

### D10 — What the sword is solid against

**Decision: as recommended, with the sweep stated.** The blade is solid against the ground, the walls and the other blade, all swept. The hilt does not cut.

**The sweep:** each tick, both endpoints of every blade and every capsule are interpolated linearly from `x_prev` to `x`. The interval is split into `S` substeps:

    S = ceil(max relative endpoint displacement / smallest capsule radius)

`S` is capped by a maximum that is derived from the speed cap, not typed. The first substep at which the segment-to-capsule distance is at most the radius is the contact.

This turns the brief's M3.0 guess ("four or fewer") into a formula that M3.0 checks. *Rejected:* an analytic time of impact. A rotating segment has no closed form, so the analytic version would itself need iteration.

### D11 — The cut rule

**Decided by Sam (0.7): a fighter's own blade cuts that fighter.**

**Built on as recommended, with "away from the heart" defined.** The body is a tree of parts rooted at the chest, which holds the heart. On the first tick a blade touches a part, the part is cut straight across at the spine point nearest the contact. Two particles are inserted there, and the subtree on the far side of the cut becomes a separate piece. A stick's mass is split by the cut fraction (H3).

There is one cut per blade, per part, per contact. A contact ends on the first substep with no overlap. The hands on the hilt are exempt from their own blade only (D7).

The chest's spine runs from the shoulder point down to the waist joint. The heart band covers the top of it (D12). So a chest cut above the band does not arise, and a cut below it drops the waist and the legs.

### D12 — Ink and fatal zones

**Decision: as recommended.** `data/body.json` gives each part:
- a drain rate per tick for a stump;
- optional fatal bands along its spine, as fractions;
- the copy key it reports as.

Rules:
- A cut on the head or neck ends the round.
- A chest cut inside the heart band ends the round.
- Any other cut opens a stump that drains every tick, starting with the tick the cut lands.
- A stump removed by a later cut stops draining (H4).
- Zero ink ends the round.

Core also keeps `spilled[part]`, the ink drained by each part's stumps over the round, including stumps since removed. `results.road.lose_ink` asks for this. Part I Q1 (the waist) is open, and `body.json` makes the answer a data change.

### D13 — A match

**Decision: as recommended.** A round resets both fighters. If both stop on the same tick, the round is played again (`results.draw`). The match is first to `ROUNDS_TO_WIN`, a constant in `sim::balance`, recommended at three (Part I Q8). `how.match.body` reads it through the `{rounds_to_win}` placeholder.

### D14 — Lockstep

**Decision: as recommended, which means three pieces Floodline does not have** (section 7, item 5).

The structure is Floodline's star with two seats. The host relays and has a clock, and it is not an authority (`lockstep.rs:5-8`). Nobody simulates ahead and nobody rolls back. Every lockstep message is reliable and ordered, as in Floodline (`lockstep.rs:247` and on). The unreliable channel is used only if M4.0 measures stalls.

The three new pieces:

1. **A measured delay.** In the lobby the host sends 20 pings and takes the 90th-percentile round trip. A joiner's input travels to the host and comes back in a bundle, which is one round trip, so:

       delay_ticks = clamp(ceil(rtt_p90 / tick_ms) + 1, 2, 8)

   Checked by hand at `tick_ms = 16.67`: 30 ms → 3 ticks; 80 ms → 6 ticks. That agrees with the brief's guess of 3 to 6 for 30 to 80 ms.
2. **`Welcome` carries `delay` and the `Setup`.** Floodline's carries a snapshot and no delay (`wire.rs:22`).
3. **`Loopback` gains jitter.** Floodline's has latency and loss only (`loopback.rs:23-42`).

The desync stop is redefined to be checkable. Each input carries the checksum of tick `t − delay`. The host compares, and on a mismatch it sends `Stop{tick}` and sends no further bundle. Both peers then display the same `tick`, and neither advances past the host's last bundle. In Floodline only the host detects a desync and joiners go to `Ended` (`lockstep.rs:552-584`), which is not "the same tick" on both.

The silence thresholds are a 1 s warning and a 10 s drop *(guess, M4.0)*. Floodline's are 5 s and 30 s at 20 ticks a second (`lockstep.rs:25-28`), too long for a sword fight.

### D15 — Files

**Decision: deviates from the recommendation in two ways.**

1. **Replay v1 carries more than the recommended `{format, version, sim_version, setup, inputs}`:**
   - `ticks` and `checksum`, the recorded end state, because 0.3 item 8 asks for playback "to the same final checksum", and a file without the checksum cannot be checked against it;
   - `checkpoints`, a checksum every 60 ticks, so `A3 Locate the first divergence` can name the first second that differs without re-recording.
2. **The replay is `postcard`, not JSON,** so it lives in `sim` with its allowed dependencies, and the shim decides nothing. At three minutes it is about 21.6 KB of inputs (180 s × 60 × 2 bytes).

Save v1 is `{format, version, state}` as recommended, in JSON, in `content`. It holds road progress, bindings and options. Its destructures are exhaustive, as in Gear Master 2D's `save.rs:346, 578, 580`, and it is refused in two passes, envelope then body, as in `save.rs:502-535`. Both files use the house Blob download (GM2D `web/app.js:4659-4661`) and file-input load (`app.js:5290`). The `localStorage` copy is labeled by `settings.save.autosave`. GM2D planned this label and never shipped it (section 7, item 11).

### D16 — Opponents

**Decision: as recommended.** A pilot is `(&World, seat, &mut PilotState) -> Input` and holds nothing a player cannot press. There are six kinds, mapped to the eight opponents already in the copy:

| Opponent | Kind | Pilot data that fills its placeholders |
|---|---|---|
| scarecrow | still | none |
| gatekeeper | pose-holder | none |
| thresher | loop | `pause_ticks` → `{pause_s}` |
| ferryman | state machine, longer blade (a `Setup` override) | blade length → `{reach_pct}` |
| vaulter | state machine (plant, vault, cut down) | none |
| windmill | loop with steps (one-direction swing, drift) | none |
| sampler | replayer: still in round one, then the previous round mirrored | none |
| reader | look-ahead search on a cloned world | `horizon_ticks` → `{horizon_ms}`, `reaction_ticks` → `{reaction_ms}` |

Further rules:
- **The reader's reaction delay is a ring of past `World` clones in `PilotState`.** The search runs on the clone that is `reaction_ticks` old.
- **The yardstick is a ninth entry, `yardstick`,** of the search kind at a fixed horizon. It is not on the road. "At its strongest" means its largest horizon in `crates/lab`, with no tick budget.
- **Seeded matches differ by the world `Rng`'s spawn jitter** (a few centimeters per round) **and by a pilot `Rng` held in `PilotState`.** `sim` keeps one `Rng`, in `World`.
- No learned policy in MVP. The house rule is quoted in `gear-master/design/rl-agent-plan.md:53`.

Replays record both seats' inputs, so playback never runs a pilot.

### D17 — Look

**Decision: as recommended.** The figures are flat paper capsules drawn from particle positions on a paper ground, with no sprites. Indigo is solid, and ochre is striped (`fighters.*.described`). A fighter's ink is that fighter's color. A cut face is drawn in the paper color with the fighter's ink along its edge.

`data/palette.json` is the only place a color is written. "The red range" is defined so the lint can rule: a color whose HSL hue lies in [330°, 20°] with saturation at or above 0.20.

Proposed values *(guesses, contrast measured in M6)*:
- indigo `#3E4A89`, hue about 231°;
- ochre `#B8862B`, hue about 39°, 19° clear of the band;
- paper `#EFE8D8`.

Any color near the band goes to Sam (brief 0.4).

### D18 — Sound

**Decision: as recommended.** The music slot ships in M2:
- `<input type="file">` produces an object URL, played by an `<audio loop>`;
- volume from Settings;
- "Remember this track" keeps the file in **IndexedDB**. The copy says "this browser's storage", and `localStorage` cannot hold an audio file.
- the `LICENSES.md` guard runs in packaging.

Sound effects come in M6, synthesized by the page with Web Audio from core's events. No file ever enters the build for them.

---

## 3. Repo layout and conventions

The layout is brief D.0, with these changes, each argued above or in section 7:

```
vagrancy/                     (repo root: section 8, Q11)
  crates/sim/tests/           boundary.rs, determinism.rs, h1..h6 tests, golden.rs
  crates/content/tests/       copy.rs, palette.rs, data.rs, save.rs
  crates/net/tests/           lockstep.rs
  testing/                    drive.py, referee.py, replays/
  reference/                  gitignored clones (A.1)
  web/assets/music/           gitignored (0.5)
  analysis/                   ladder.md, recon tables (written by make targets)
  .github/workflows/          deploy.yml (test, web, gate in three engines, deploy)
```

There is no `crates/net-web` (D1). The rules live in `CLAUDE.md` and are not repeated here.

Conventions taken from the house, with their sources:
- **Release profile:** `opt-level = "z"`, `lto`, one codegen unit, `strip`, `overflow-checks`, and no `panic = "abort"`. Comments are copied from Floodline's `Cargo.toml:28-44`.
- **Test profile:** `debug = "line-tables-only"`, `opt-level = 2` (GM2D `Cargo.toml:28-45`).
- **`wasm-bindgen` version** is read from `Cargo.lock` (GM2D `package-web.sh:30-36`).
- **Content stamp:** an 8-character sha256 prefix, so a returning player gets the fix. The reason is `Cache-Control: max-age=600` (GM2D `package-web.sh:62-111`).
- **Vendor bundles** are pinned by sha256 (Floodline `package-web.sh:20-23`, `web/vendor/README.md:9-10`).
- **Counting tests:** `--no-run --message-format=json`, then `--list` on each binary (GM2D `count-tests.sh:24-40`), over the whole workspace.

`package-web.sh` also fills the player-facing strings in `index.html` (`<title>`, `<noscript>`, the description) from `data/copy.en.json` at package time. Those strings must appear before any script runs, and copying them by hand would make a second copy.

---

## 4. Milestones

A milestone is not complete until all of these are true:
- `make test` passes;
- `make web` builds;
- `make test-ui` walks the gate in three engines;
- the deploy gate is live and Sam has seen it.

Every new test is broken once and watched failing, and the red line goes in the commit. Counts come from `packaging/count-tests.sh`.

### M0 — Foundation, and the voice in the repo

- **Goal:** an empty page deploys and names its build, and every lint that guards the words and the colors exists and has been seen failing.
- **Recon:**
  - that `make web` and the stamp work with `wasm-bindgen` 0.2.127 as pinned in the new lockfile (guess: they do);
  - how long the empty workspace's suite takes (guess: under 30 s, which sets the baseline before physics arrives; `HOUSE-STYLE.md` §12.2).
- **Deliverables:**
  - the workspace with six crates compiling empty, and the profiles with their comments;
  - `Makefile`: `test`, `check`, `web`, `serve`, `test-ui`, `test-ui-setup`, `ladder` (stub), `publish`, `help`;
  - `packaging/package-web.sh` (stamp, vendor pin, audio guard, copy fill) and `packaging/count-tests.sh`;
  - `.github/workflows/deploy.yml`;
  - `web/index.html`, `app.js` and `styles.css` showing `game.description`, `game.loading`, `game.build` with its hash, and `game.content_note` if Sam says yes (Part I Q3); no About link (Q5);
  - `testing/drive.py`, ported from GM2D. It uses `ORIGIN` as Part G names it (GM2D's is `GM2D_ORIGIN`, `drive.py:50`), plus a check that every visible text node equals a rendered copy string;
  - `LICENSES.md`, empty table; `MILESTONES.md`; `DECISIONS.md`; `.gitignore`.
- **Acceptance:**
  - `sim_depends_on_serde_and_postcard_and_nothing_else`;
  - `sim_has_no_float_no_hashmap_and_no_clock`. This is a source scan with comments and strings stripped, which Floodline does not have (section 7, item 1). It is a source-text lint on purpose, because the rule is about source, and it is broken once by inserting `let _x: f32 = 0.0;` into a copy of a sim file;
  - `every_string_a_player_reads_is_in_the_copy_file`, in the gate, from rendered text, so it checks behavior;
  - `no_string_breaks_the_tone_file`, which holds rules 3, 4, 8, 10, 12, 13, 14 and 16 and the glossary;
  - `nothing_from_the_show_appears_in_data_or_web`, whose deny-list Sam confirms (Q13);
  - `the_game_name_is_spelled_only_in_game_name`;
  - `no_ink_or_cut_color_is_red`;
  - `no_audio_ships_without_a_license_row` (packaging test, ahead of M2 so the guard exists before the slot);
  - the gate: three engines, no console error, no request that leaves the origin.
- **Hand-computed cases:** none.
- **Deployable?** Gate 1. A visitor sees what the game is, a loading line, and the build hash.
- **Risks:**
  - The `local.keyboard_limit` hit means `no_string_breaks_the_tone_file` is red on the authored copy. It ships with that key on a recorded allow-list naming Q10 until Sam rules. The string is not repaired.
  - The GitHub repository does not exist (Q11).

### M1 — A body in integers, and a match you can play back

- **Goal:** one fighter stands, its arm answers four keys, and the same inputs give the same world everywhere.
- **Recon (M1.0), each a `crates/lab` command whose output goes in the commit:**
  - drift of a fighter left alone for 600 ticks at 8, 12 and 16 fractional bits (guess: 12 holds it within 1 cm);
  - the cost of one tick for two fighters, natively and in wasm (guess: under 50 µs natively);
  - one step's distance and duration at three step speeds (guess: about 16 s to cross, D8);
  - the downward bias of truncating multiplies over 600 ticks (D3; guess: smaller than the drift).
- **Deliverables:**
  - `Fx` with 12 bits, everything widened, truncating multiply and divide;
  - `isqrt(u64) -> u64`, from Floodline's `fx.rs:108`, which takes `i64`;
  - the sine table at 1° resolution, a 91-entry quarter wave, generated by a committed script with its output committed. Floodline's 256-per-turn table cannot hit 30° (section 7, item 3);
  - `Rng`, from Floodline's `rng.rs`;
  - `Setup`, `World`, `World::step`;
  - particles, sticks and the fixed-order solver;
  - the ground, the balance rule and the step, the arm motors;
  - the checksum (FNV-1a over `postcard`, Floodline's `world.rs:777-784`, `:2180`) and `SIM_VERSION`;
  - replay v1;
  - the shim calls `new`, `step`, `frame`, `checksum`, `numbers`, `replay_bytes`, `load_replay`;
  - a page that draws `frame` and offers "Download replay" and "Load a replay".
- **Acceptance:**
  - H1 and H6;
  - `two_worlds_fed_the_same_inputs_agree_for_ten_thousand_ticks`;
  - `a_mirrored_world_stays_a_mirror_for_ten_thousand_ticks`, which the agent adds (D3);
  - `only_arm_bits_drive_a_joint_motor` and `the_step_bits_move_the_legs_only_through_the_balance_rule` (D6);
  - `a_replay_played_back_ends_on_the_recorded_checksum`;
  - `a_replay_from_another_sim_version_is_refused_with_a_sentence`;
  - `a_damaged_replay_is_refused_rather_than_half_loaded`;
  - `a_replay_with_a_spare_bit_set_is_refused`;
  - `a_golden_replay_ends_where_it_always_has`;
  - `the_checksum_would_actually_catch_a_divergence`, after Floodline's `determinism.rs:104`;
  - in the gate: the native checksum of a fixed script equals the wasm checksum in all three engines.
- **Hand-computed cases:** H1, H6.
- **Deployable?** Gate 2. Move the arm, download the replay, reload, load it, and watch the same motion.
- **Risks:**
  - The relaxation passes may not hold a five-stick arm stiff at 12 bits. M1.0 measures it, and the answer may move D3.
  - The balance rule is the least specified thing in the brief. If it cannot be kept momentum-conserving, that is a worklist row for Sam, because H5 then needs a different control.

### M2 — The sword is solid, and a swing is free

- **Goal:** a fighter can pogo, and can build speed by swinging, in a practice yard with music.
- **Recon (M2.0):**
  - three tunings of (motor strength, cap, drag), each measuring:
    - pogo height;
    - ticks to reach the cap by swinging;
    - distance carried by one swing in the air.

    Guess: one tuning gives a pogo of 1 to 2 body heights and reaches the cap in 2 to 4 s;
  - over 100 random-input runs, the share that end in a self-cut. This is measured once M3's cut exists; here it is measured as contacts. Guess: unknown.

  Sam picks the tuning by playing. All three ship behind a practice-yard selector until Sam picks.
- **Deliverables:**
  - the blade as a swept stick against the ground and walls (D10);
  - friction;
  - the motor rule;
  - cap and drag in `sim::balance`;
  - the practice yard with its five steps from the copy;
  - the music slot (D18).
- **Acceptance:**
  - H5 and H5c;
  - `a_planted_sword_lifts_the_fighter`, with the measured height in the assertion;
  - `speed_never_passes_the_cap`, over 100 seeded random runs;
  - `nothing_leaves_the_arena`;
  - `a_blade_never_ends_a_tick_below_the_ground`;
  - in the gate: a loaded track plays and no request leaves the origin. The gate uses a 1 s generated WAV written to a temp directory and never committed.
- **Hand-computed cases:** H5.
- **Deployable?** Gate 3. Swing, pogo, and cross the yard by swinging. Sam plays from here on.
- **Risks:**
  - The pogo depends on the wrist being stiff (D7). A soft wrist absorbs the push.
  - The tunings may disagree with the "slow step" feel. That is Sam's call.

### M3 — Cuts, ink, and two fighters at one keyboard

- **Goal:** two people at one keyboard can play a whole match.
- **Recon (M3.0):** the `S` the formula in D10 produces at the chosen cap, and the measured tick cost at that `S` (guess: 4 or fewer, under budget). Part I Q1 and Q8 must be answered before this starts.
- **Deliverables:**
  - the second fighter;
  - blade against blade;
  - the swept contact;
  - the cut rule and pieces;
  - stumps, ink and `spilled`;
  - fatal bands from `body.json`;
  - rounds and the match;
  - the event list (`Cut`, `Clash`, `Landing`, `RoundEnd{cause, part, by}`);
  - result strings;
  - palette and stripes;
  - key bindings in Settings, with the conflict string.
- **Acceptance:**
  - H2, H2b, H3, H4;
  - `a_cut_on_a_piece_that_has_dropped_spills_nothing`;
  - `the_hands_on_the_hilt_are_safe_from_their_own_blade`;
  - `an_opponents_blade_cuts_a_hand` (pending Q14);
  - `a_neck_cut_ends_the_round_on_the_tick_it_lands`;
  - `a_cut_inside_the_heart_band_ends_the_round`;
  - `two_blades_never_pass_through_each_other`, over seeded random runs;
  - `the_result_names_the_cut_that_ended_the_round`;
  - `the_part_that_spilled_most_counts_removed_stumps`;
  - in the gate:
    - a scripted match runs to a result in three engines and its replay round-trips;
    - canvas pixels after a scripted cut contain no red-range pixel.
- **Hand-computed cases:** H2, H3, H4.
- **Deployable?** Gate 4. The first build worth playing beside a friend.
- **Risks:**
  - The self-cut rate may make early play unplayable. The M2.0 number goes to Sam as a worklist row about a minimum cutting speed, and is not added unilaterally.
  - Rollover on six keys (the copy already warns).

### M4 — Lockstep, on a loopback and then between two browsers

- **Goal:** two browsers on two networks play a match that stays one match.
- **Part one, no browser:**
  - `Peer` and `Loopback` with jitter;
  - the wire format;
  - the two-seat lockstep;
  - the measured delay (D14);
  - `Stop{tick}`;
  - the drop on silence.
- **Part two, the transport:**
  - `web/rtc.js`, from `window.FLOODLINE_RTC` in `quad_rtc.js:683-710`, with the plugin registration (`:786`) and the canvas hack (`:625-670`) removed;
  - Trystero 0.25.4, vendored and pinned (Floodline `vendor/README.md:9-10`);
  - `web/echo.html`;
  - room codes and pasted codes;
  - the build hash in the room name;
  - the lobby strings.
- **Recon (M4.0), with Sam and a friend:** round trip and stall rate between two home networks (guess: 30 to 80 ms, so a delay of 3 to 6 ticks).
- **Acceptance:**
  - `an_input_takes_effect_delay_ticks_later_on_both_peers`;
  - `the_delay_is_chosen_from_the_measured_round_trip`;
  - `a_mismatch_stops_both_peers_on_the_same_tick_and_names_it`;
  - `a_silent_peer_is_waited_for_and_then_dropped`;
  - `jitter_changes_nothing_but_the_wait`;
  - echo checks for both paths;
  - a two-tab match;
  - `testing/referee.py`, after Floodline's `packaging/browser/referee.py`, watches a scripted three-minute match and writes down whether the peers stayed together;
  - **Sam plays one real match across two networks.**
- **Hand-computed cases:** the delay arithmetic in D14.
- **Deployable?** Gate 5. Send a friend a link and fight them.
- **Risks:**
  - The gate forbids off-origin requests, but relays and STUN are off-origin by nature. The main gate never opens the online path. Online checks run in a separate mode that uses pasted codes with host ICE candidates only, and that mode lists any host it contacts (Q18).
  - Symmetric NATs fail without TURN. `online.error.no_path` already says so.

### M5 — The road (MVP complete)

- **Goal:** a player can walk the road from the first opponent to the last.
- **Recon (M5.0):** headless ticks per second, and the search horizon that fits in one tick in wasm (guess: 200,000 ticks a second or more; a horizon of about 20 ticks, a third of a second).
- **Deliverables:**
  - the six pilot kinds and the yardstick;
  - `data/pilots.json` and `data/road.json`;
  - `make ladder`, which writes `analysis/ladder.md`;
  - the road screen, with introductions filled from pilot data;
  - save v1 with the labeled convenience copy.
- **Acceptance:**
  - `a_pilot_returns_an_input_and_nothing_else`;
  - `the_road_is_ordered_by_the_yardstick`, over 200 seeded matches per stop;
  - `every_opponent_can_be_beaten`;
  - `every_number_in_an_introduction_comes_from_the_pilot_data`;
  - `what_an_introduction_says_a_pilot_usually_does_is_what_it_does` (TONE rule 11);
  - save round-trip, newer-version and damaged-file tests;
  - the 0.3 checklist walked by hand, every line a yes.
- **Hand-computed cases:** none new.
- **Deployable?** Gate 6, tag `v0.1.0-mvp`. The tag is Sam's push.
- **Risks:** the pilots may be unable to fight well with a key-by-key arm (brief Part J). If the search pilot loses to the yardstick's weaker settings, the road order is reported, not forced.

### M6 — Sound, figures, and a pass over the page (after MVP)

- **Deliverables:**
  - Web Audio effects from events;
  - TikZ backdrops, if wanted;
  - a second `TONE.md` pass;
  - accessibility: focus order, the canvas name, contrast of both fighters on paper, measured;
  - reduce motion.
- **Note on reduce motion:** `settings.motion.desc` is shipped only if screen shake or slow motion exists (`_depends`). The plan proposes neither, so that string stays unshipped unless Sam wants the effects.
- **Deployable?** Yes, with no version bump.

### The hand-computed cases, with the agent's arithmetic

`C4 Verify against hand-computed cases`. Every case agrees with the brief's answer. Two cases are strengthened, and the strengthenings are the agent's own; they are marked as such.

**H1 (M1).**
- Starting speed is 1, gravity 4, speed updated first. The steps are 5, 9, 13, …, `4k + 1`.
- The sum is `Σ_{k=1..n}(4k + 1) = 2n(n + 1) + n = n(2n + 3)`.
- At n = 2: 2·7 = **14**. At n = 10: 10·23 = **230**.
- Position first: `Σ_{k=0..n−1}(4k + 1) = 2n(n − 1) + n = n(2n − 1)`, which is 6 at n = 2.
- `n(n + 1)/2` is 3 at n = 2.
- In Verlet the order is set by `x_prev = x − 1`. The first displacement is then `1 + 4 = 5`, the same as speed first.

Agrees.

**H2 (M3).**
- On tick k the blade spans x = −10 to 0. On tick k + 1 it spans 10 to 20. The limb's surface is at x = 3 to 5.
- Gap at k: 3. Gap at k + 1: 5. No contact at either end.
- The tip crosses x = 3 at 3/20 of the tick. The contact is on y = 5, so the nearest spine point is **(4, 5)**.

Agrees.

*Strengthening (H2b):* H2's blade moves along its own length, so any substep size up to 12 (blade length 10 plus limb diameter 2) catches it. H2 therefore cannot fail for a two-substep sweep.

H2b sets up a case a coarse sweep misses:
- a blade from (0, 3) to (0, 7), moving +20 a tick across a vertical limb at x = 10, radius 1;
- substeps of 5 sample x = 5, 10, 15, 20, and only x = 10 hits;
- substeps of 4 sample 4, 8, 12, 16, 20, and miss.

So the formula in D10 must give `S ≥ 10` here (displacement 20 / radius 1 = 20 → `S = 20`). **The cut must land at (10, 5).**

**H3 (M3).**
- Mass 8, cut at a quarter from the near end.
- Near piece: 8·1/4 = **2**. Far piece: 8·3/4 = **6**.

Agrees. The far piece is the one off the heart's side of the body tree.

**H4 (M3).**
- End of tick t, for 0 ≤ t ≤ 29: `1000 − 10(t + 1)`. At t = 29: **700**.
- From tick 30 the rate is 20 alone: `700 − 20(t − 29)` = 0 when t − 29 = 35, so **t = 64**.
- Adding the rates gives 30 a tick: `700 − 30(t − 29)`. That is 10 at t = 52 and below zero at t = 53, so **53**.

Agrees on both.

**H5 (M2).** Gravity is off, there is no ground, and a shoulder key is held for 60 ticks. Total momentum is not zero, and kinetic energy is above its starting value.

*Strengthenings:*
- **The kinetic-energy half cannot fail on its own.** The fighter starts at rest with KE 0, so any motion at all raises it, and a motor that does push back on the torso also creates KE. The momentum half is the one that decides. It stays the assertion, and the KE half stays because the brief wrote it.
- **H5c, the control.** In the same world, the step bits alone for 60 ticks, then no bits for 60 ticks, leave total momentum **exactly zero**. This proves the momentum in H5 came from the motor, and not from the balance rule or rounding.
- **Direction.** Over 60 ticks of a held shoulder key the arm may turn far enough that the injected pushes partly cancel. H5 asserts `|p|` above a threshold measured in M2.0 and records its direction, rather than "not zero".

**H6 (M1).**
- `isqrt(15) = 3`, `isqrt(16) = 4`, `isqrt(17) = 4`, `isqrt(2^62) = 2^31`.
- `2^31` does not fit `i32` (maximum `2^31 − 1`), so the function returns `u64`. Gear Master 2D's `isqrt(n: i32) -> i32` (`shot.rs:610`) could not run this case.
- Sine at 12 bits: sin 0° = 0, sin 90° = 4096 (exactly one), sin 30° = 2048 (exactly one half). The table is indexed in whole degrees.
- `-5 >> 1 = -3`, which floors (Floodline's `fx.rs:52-53`, tested at `:318`).

Agrees.

---

## 5. Wire, replay and save formats, by example

**Wire** (`net::wire`, `postcard`, every message reliable and ordered):

```
Hello   { proto: 1, build: "3f9a01c2" }                           joiner → host
Welcome { seat: 1, delay: 4, setup: <Setup bytes>, seed: 917 }    host → joiner
Ping    { n: 7 }  /  Pong { n: 7 }                                lobby, 20 rounds
Input   { tick: 1204, input: 0b0000_0101, checked: 1200, sum: 0x8c1e…77 }
Bundle  { tick: 1204, inputs: [0b0000_0101, 0b0001_0000] }        host → joiner
Stop    { tick: 1200 }                                            checksum mismatch
Bye     { reason: Full | Build | Left }
```

**Replay v1** (`sim::replay`, `postcard`, shown here as a struct):

```
Replay {
  format: "vagrancy.replay", version: 1, sim_version: 7,
  setup: Setup { seed: 917, rounds_to_win: 3, body: …, fighters: [...], arena: … },
  inputs: [[0x05, 0x10], [0x05, 0x10], …],      // one pair per tick
  ticks: 6012, checksum: 0x51d2_09aa_c3e1_7f40,
  checkpoints: [0x…, 0x…, …],                     // every 60 ticks
}
```

`Setup` carries the full content, not data ids, so a replay does not change when `data/` does. Only `SIM_VERSION` can make one unplayable.

**Save v1** (`content::save`, JSON):

```json
{
  "format": "vagrancy.save", "version": 1,
  "state": {
    "road": { "cleared": ["scarecrow", "gatekeeper"], "next": "thresher" },
    "bindings": { "solo": { "shoulder_up": "KeyQ", "shoulder_down": "KeyW", "elbow_in": "KeyO",
                            "elbow_out": "KeyP", "step_left": "KeyA", "step_right": "KeyD" } },
    "options": { "music_volume": 70, "effects_volume": 70, "remember_track": false }
  }
}
```

The format tags spell the game's name. That depends on Q12.

## 6. Data formats

Every value below is a placeholder *(guess)* until the recon of the milestone that owns it.

```json
// data/body.json — the tree is rooted at the chest; "near" is toward the chest
{ "ink": 6000,
  "parts": [
    { "id": "chest",    "copy": "chest", "parent": null,     "len": 50, "radius": 14, "mass": 30,
      "drain": 40, "fatal": [{ "from": 0.0, "to": 0.6, "cause": "heart" }] },
    { "id": "neck",     "copy": "neck",  "parent": "chest",  "len": 10, "radius": 5,  "mass": 2,  "fatal": [{ "from": 0.0, "to": 1.0, "cause": "neck" }] },
    { "id": "head",     "copy": "head",  "parent": "neck",   "len": 22, "radius": 10, "mass": 5,  "fatal": [{ "from": 0.0, "to": 1.0, "cause": "neck" }] },
    { "id": "waist",    "copy": "waist", "parent": "chest",  "len": 20, "radius": 13, "mass": 12, "drain": 30 },
    { "id": "upper_arm_lead", "copy": "arm", "parent": "chest", "len": 30, "radius": 5, "mass": 3, "drain": 20, "motor": "shoulder" },
    { "id": "forearm_lead",   "copy": "arm", "parent": "upper_arm_lead", "len": 28, "radius": 4, "mass": 2, "drain": 10, "motor": "elbow" },
    { "id": "hand_lead",      "copy": "hand", "parent": "forearm_lead", "len": 8, "radius": 4, "mass": 1, "drain": 5, "grip": true }
    // … rear arm (no motor), thighs and shins ("leg"), feet
  ],
  "sword": { "len": 95, "hilt": 20, "mass": 2 } }

// data/controls.json — browser KeyboardEvent.code values
{ "solo":  { "shoulder_up": "KeyQ", "shoulder_down": "KeyW", "elbow_in": "KeyO", "elbow_out": "KeyP", "step_left": "KeyA", "step_right": "KeyD" },
  "left":  { "shoulder_up": "KeyQ", "shoulder_down": "KeyW", "elbow_in": "KeyE", "elbow_out": "KeyR", "step_left": "KeyA", "step_right": "KeyD" },
  "right": { "shoulder_up": "KeyU", "shoulder_down": "KeyI", "elbow_in": "KeyO", "elbow_out": "KeyP", "step_left": "KeyJ", "step_right": "KeyL" } }

// data/palette.json — the only place a color is written
{ "paper": "#EFE8D8", "ground": "#D9CDB2", "line": "#2B2A28",
  "indigo": { "body": "#3E4A89", "ink": "#3E4A89", "pattern": "solid" },
  "ochre":  { "body": "#B8862B", "ink": "#B8862B", "pattern": "stripes" },
  "red_band": { "hue_from": 330, "hue_to": 20, "min_saturation": 0.20 } }

// data/road.json — order is a measurement (make ladder) and Sam's play
{ "stops": ["scarecrow", "gatekeeper", "thresher", "ferryman", "vaulter", "windmill", "sampler", "reader"] }

// data/pilots.json
{ "scarecrow":  { "kind": "still" },
  "gatekeeper": { "kind": "pose", "shoulder": 10, "elbow": 175 },
  "thresher":   { "kind": "loop", "pattern": "overhead", "pause_ticks": 90 },
  "ferryman":   { "kind": "machine", "script": "late_heavy", "sword_len": 125 },
  "vaulter":    { "kind": "machine", "script": "vault" },
  "windmill":   { "kind": "loop", "pattern": "spin", "drift": true },
  "sampler":    { "kind": "replayer", "first_round": "still", "mirror": true },
  "reader":     { "kind": "search", "horizon_ticks": 18, "reaction_ticks": 12, "branches": 9 },
  "yardstick":  { "kind": "search", "horizon_ticks": 12, "reaction_ticks": 15, "branches": 9 } }
```

How the placeholders are filled, all by `content`:
- `{reach_pct}` = `round(100·(125 − 95)/95)` = 32, from the ferryman's `sword_len` and the default sword's `len`;
- `{pause_s}` = 90/60 = 1.5;
- `{horizon_ms}` = 18·1000/60 = 300;
- `{reaction_ms}` = 200.

None of these is typed into a sentence.

---

## 7. Corrections to the brief, from the code

In the form of `gear-master/design/rl-agent-plan.md:29-80`: each claim is quoted, given a verdict, and the evidence is cited by line. That section is a numbered list with verdicts, not a table (item 14), so this one is too. Items 1 to 17 correct what the brief and `HOUSE-STYLE.md` say about the sources. Items 18 to 23 are places where the brief disagrees with itself.

**About the sources**

1. **"`tests/boundary.rs` enforces [no `f32`, `HashMap`, `std::time`]" (`CLAUDE.md`, Non-negotiable).** False for Floodline's version. It parses `Cargo.toml` and nothing else: `sim_depends_on_serde_and_postcard_and_nothing_else` (`boundary.rs:18`) and `sim_names_no_graphics_or_networking_crate` (`:35`). No test scans `sim`'s source. Floodline's `sim/src` is in fact clean; the only hits are in comments (`lib.rs:6`, `world.rs:831`). The source scan is new work in M0.
2. **"`fx.rs` … with every multiply and divide passing through `i64`" (`HOUSE-STYLE.md` §2).** Partly. `Fx*Fx` and `Fx/Fx` widen (`fx.rs:153-164`). `Mul<i32>` and `Div<i32>` do not (`:169-170`, `:176`). The module doc claims they do (`:16`). Also, `>>` floors (`:52-53`) while `Fx/Fx` "Truncates toward zero" (`:160`), so the file has two rounding rules. D3 picks one.
3. **"Start `fx.rs` … from Floodline's" (D1), and H6's sine values.** Floodline's angles are `Turns = u8` with a 256-entry table (`fx.rs:183, 188`). There 30° is 21.33 steps, so H6's "exactly one half at 30°" cannot be met by that table. Gear Master 2D's `shot.rs` uses a 5° cosine table (`:127-133`), which can. The new table is in whole degrees (M1).
4. **H6's `isqrt(2^62) = 2^31`.** Floodline's `isqrt(n: i64) -> i64` (`fx.rs:108`) can run it. Gear Master 2D's `isqrt(n: i32) -> i32` (`shot.rs:610`) cannot, and `shot.rs:12` ("no `sqrt`") means no float `sqrt`. The plan uses `u64`.
5. **"Floodline's star … The delay is fixed per match from a measured round trip … sent in `Welcome`" (D14).** Floodline has no measured delay. It has `DELAY: u32 = 3` (`lockstep.rs:21`), and its `Welcome` has no delay field (`wire.rs:22`). Its comment says three ticks is "three hundred milliseconds" (`lockstep.rs:19`), but at `TICKS_PER_SECOND = 20` (`sim/src/balance.rs:45`) three ticks is 150 ms. That is a stale comment in Floodline, not something the brief claimed. Also, peers advance on `Bundle`, not on relayed `Turn`s as `floodline-design.md` §8 (`:349-401`) says. Port the code, not the design.
6. **"Proven in `cargo test` with latency and jitter injected" (M4).** Floodline's `Loopback` has latency and loss with a seeded `Rng` (`loopback.rs:23-42, 60`) and **no jitter**. Jitter is new.
7. **"`a_mismatch_stops_both_peers_on_the_same_tick`" (M4).** Floodline does not do this. Only the host detects a desync (`check_agreement`, `lockstep.rs:552`) and sends `Bye "DESYNC at tick …"` (`:576-584`). Joiners go to `Ended` when the `Bye` arrives. D14 defines a version that is checkable.
8. **"`web/quad_rtc.js` … made a plain ES module" (M4).** It is a miniquad plugin (`miniquad_add_plugin`, `quad_rtc.js:786`), and `echo.html` loads `mq_js_bundle.js` and `sapp_jsutils.js` (`:72-73`). But `window.FLOODLINE_RTC` (`:683-710`) is already a plain API, so the conversion is modest. Trystero loads by dynamic `import()` (`:229`), and the room name joins build and room with a hyphen (`:240`). The vendor pin is Trystero 0.25.4 (`web/vendor/README.md:9-10`), checked by **Floodline's** `package-web.sh` (`:20-23`). Gear Master 2D's has no vendor check.
9. **"`ORIGIN=… testing/drive.py chromium firefox webkit`" (Part G).** Gear Master 2D's variable is `GM2D_ORIGIN` (`drive.py:50`), and its CI never walks the live page. `deploy.yml` runs test, package and the three-engine gate, then deploys (`:34-66`). The live walk at `8a23dfa` was "a person pointing the gate at the live page". The new `drive.py` uses `ORIGIN` as the brief says, and the live walk stays a step done after the push.
10. **"A check that compares zero with zero (rows 10, 35, 36)" (brief E.3).** Only row 36 is that (`SECOND-ORDER-M22.md:48`). Row 10 is "A check written against a compiled-in constant cannot be made to fail" (`:22`). Row 35 is a lint that "read the source and passed with the cost halved" (`:47`). Row 35 bears directly on this plan: it is why `sim_has_no_float_no_hashmap_and_no_clock` strips comments and strings and is broken on a real type, and why the copy check also runs in the gate on rendered text.
11. **"`localStorage` holds a convenience copy that is labeled as not the real save" (`HOUSE-STYLE.md` §10).** Gear Master 2D planned the label (`PLAN.md:422`) but its UI does not show it. The autosave is at `app.js:43, 1128, 4762`, and the nearest text is `index.html:134`. Here the label is `settings.save.autosave`, which the gate checks.
12. **"A game's page has a fixed entry point … (`runtime-copy.json`, `game_locks`)" (`HOUSE-STYLE.md` §4).** That file is in the website packet (`~/Documents/website-rework/runtime-copy.json:221`), not in any game repository. `data/copy.en.json`'s `game.*` strings match its template field for field.
13. **"Gear Master has a `Verb` enum, 'everything a player can do, and nothing else'" (`HOUSE-STYLE.md` §1).** Confirmed (`console/src/verb.rs:1`), with two limits. It is not serde-serializable; it is persisted as text through `Verb::line`/`parse` (`:198, :242`). And the GUI and CLI do not play through it. Only the agent does (`agent/src/hands.rs:75`). The `Input` byte here is `Copy` and `serde`, and the page, the pilots and the net all go through it.
14. **"`design/rl-agent-plan.md` §0 is the model: the brief's claims are listed, checked against the code, and corrected by line" (A.5).** Confirmed, but as a numbered list with verdicts and `file:line` evidence (`:29-80`), not a table. Its own line citations have drifted: it cites `combat.rs:3850` for code now at `:4272`. Line numbers in this section were read at the commits above and will drift the same way.
15. **"`does not run git push … on its own judgement`" (`HOUSE-STYLE.md` §7).** The sentence is in Gear Master 2D's `CLAUDE.md:486`. Gear Master's `CLAUDE.md` has no such rule, and its push lives inside `make publish` (`Makefile:82`).
16. **"This is the lecture's own example (slide 18)" (H1).** Slide 18 is the induction example "5+9+13+…+(4n+1) = n(2n+3)", with the chatbot's `k(k+1)/2` caught at n = 2 (14 against 3). The integrator is the brief's dressing of that sum. The arithmetic is the same. The slide footers lag the slide index by one, and the brief's citations use the index.
17. **The labels.** All seventeen, A0 to C6, match the deck word for word: Procedure A on slide 17, B on slide 19, C on slide 21. The failure modes are on slide 16. The nine stages are on slide 53 (and on slide 27 without a title). No correction.

**Where the brief disagrees with itself**

18. **Arms only, against the steps.** 0.3 item 1 says "no key drives any other joint", and `CLAUDE.md` says "No key drives a leg". Sam's 0.7 answer says "The legs take slow steps" on two keys. Both cannot be literally true. The plan's reading is in D6, and Q9 asks Sam to confirm it.
19. **H2 cannot tell a real sweep from two substeps** (section 4, H2b). It is kept, and H2b is added.
20. **Drag.** D9 bounds speed "by a cap and by drag". `CLAUDE.md` says "by nothing else" and bans "a damping term". The plan follows D9 (Q17).
21. **"`tests/boundary.rs`; `tests/copy.rs` …; `tests/palette.rs`" (M0)** cannot sit at the root of a Cargo virtual workspace. Their homes are in D1.
22. **Replay v1's field list** has no recorded checksum, so 0.3 item 8 could not be checked from the file alone (D15).
23. **"The gate … fails on … any request that leaves the origin" and "two browsers on two networks"** cannot both hold in one walk, because introductions use public relays and STUN (Floodline `web/config.js`: Nostr strategy, Google and Twilio STUN, no TURN). The gate needs an online mode with a stated allow-list (Q18).

---

## 8. Open questions for Sam

None blocks M0 or M1 except Q11, which blocks deploy gate 1. Each question has the agent's recommendation beside it.

**From brief Part I**

1. **Is a waist cut fatal at once, or does it drain fast?** *Drain.* Then the heart and the neck stay the only instant endings, and `how.end.body` stays true as written. (Before M3.)
2. **Ink or blood in the text?** *Ink.* The copy already uses "blood" once, in `game.content_note`.
3. **Show the content note on the entry page?** *Yes.*
4. **Are public relays acceptable for introductions?** *Yes, with the pasted-code path as the alternative.* That is Floodline's arrangement, and `online.host_room.desc` and `online.privacy` already state the limits. (Before M4.)
5. **The About link.** *Leave it out until `/projects/vagrancy/` exists.* `game.about_link` stays unshipped.
6. **May `TONE.md` be public?** *Yes.* It holds derived rules and no transcript. The lints work either way.
7. **A license for the track.** *The slot stands until a license exists.* Then it is one `LICENSES.md` row.
8. **Rounds to win, and stops on the road?** *Three rounds and eight stops.* The copy already has eight opponents. (Before M3 and M5.)

**Raised by the plan**

9. **May `CLAUDE.md` read "Only arm bits drive a joint motor; the step bits ask the balance rule for a step"?** *Yes.* Otherwise Sam's 0.7 answer breaks a non-negotiable (section 7, item 18).
10. **`local.keyboard_limit` says "both of you" in a string two players read at one keyboard.** `TONE.md` rule 3: "in a versus string, search for 'you'. It should not be there." *Sam rewrites the string, or amends rule 3 to allow one-keyboard instructions addressed to both players.* The agent does not repair it. Until Sam rules, the lint carries it on a recorded allow-list.
11. **Where the repository lives.** `sgilson7/vagrancy` does not exist on GitHub (`gh repo view sgilson7/vagrancy` → "Could not resolve"). `~/Documents/vagrancy` is not a git repository, and the packet is in its `vagrancy-packet/` subfolder. *Make `~/Documents/vagrancy` the repo root, move the packet files up into it, and leave the zip outside the repo or gitignore it. Sam then creates the GitHub repository and sets Pages to deploy from Actions.* The agent will not create the remote or push.
12. **Does "no other string spells it" cover identifiers?** The rule says `game.name` is the only string that spells the name. That could be read to cover the repository name, the URL, crate names, the replay and save format tags, and `<title>` (which is filled from `game.name`). *It covers strings a player reads. Identifiers may spell it.*
13. **The deny-list for `copy.rs`.** The agent's draft:
    - the show's title;
    - its studio and director;
    - the names of its three leads and recurring characters;
    - the soundtrack's artists, including Force of Nature;
    - "sunflower" (the show's central plot device).

    *Sam confirms or extends it.* Anything the agent has to argue is not on the list is Sam's (brief 0.4).
14. **Can an opponent's blade cut a hand on the hilt?** *Yes. Only a fighter's own blade is exempt.*
15. **Does a sword nobody holds still cut?** *No.* Solid, but every cut has an owner to name.
16. **What do the keys do after the lead arm is cut?** *They drive only the joints still attached.* A cut upper arm leaves the keys doing nothing until the round ends, and ink decides it.
17. **May `CLAUDE.md` read "bounded by the cap and the drag in `sim::balance`"?** *Yes.* D9 already says drag (section 7, item 20).
18. **The gate's online mode.** *The main gate stays strict. A separate `make test-ui-online` uses pasted codes with host candidates only and prints every off-origin host it contacts.* Relay checks are done by hand, by Sam.
19. **Minimum cutting speed.** This is not a question yet. It becomes one if M2.0 finds that most random runs end in a self-cut.

---

## Approval

Sam approved this plan on 2026-10-03 and answered four questions at the same time:

1. **Push rights.** The agent may create the public repository `sgilson7/vagrancy`, turn on Pages (deploy from Actions), and push at every deploy gate without asking each time. It never force-pushes, and it walks the live page after every deploy. This is the explicit ask that `CLAUDE.md` requires before a push.
2. **Repo root (Q11).** `~/Documents/vagrancy`, with the packet files moved up into it.
3. **Open questions.** The agent's recommendation stands as the answer to every question in section 8. Each is recorded in `DECISIONS.md`, and Sam may override any of them later. `local.keyboard_limit` stays on the lint allow-list, unchanged.
4. **Steps that need Sam.** These are: choosing the M2 tuning by playing, the M4.0 match across two networks, and seeing each gate before the next milestone starts. The agent takes the default, runs the nearest check it can (two tabs and the referee script for M4), and carries each skipped check as a notebook row with status `the human's`. Sam asked the agent to keep going until there is a working MVP on Pages.
