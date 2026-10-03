# Vagrancy — planning brief for the Claude Code agent

*The game is called Vagrancy and the repository is `vagrancy`. `data/copy.en.json` holds the name once, as `game.name`, and no other string spells it.*

This document does not build the game. It tells a Claude Code agent how to **produce the plan** that builds the game, what that plan must contain, and what "done" means at every step. Put it at the root of a new repo together with `CLAUDE.md`, `TONE.md`, `HOUSE-STYLE.md` and `data/copy.en.json`, then start the agent with:

> Read PLANNING-BRIEF.md. Execute Part A to produce PLAN.md. Stop and show me PLAN.md before executing any milestone.

## How this brief is organized

It follows the nine stages of the GenAI-assisted prototyping procedure from the CSC 484 guest lecture. Each part carries the label of the AI-literacy step it applies. The labels are the lecture's, and they are used the lecture's way: same label, same words, every time it appears.

| Stage | Who | Label | Part |
|---|---|---|---|
| brief | Sam writes it | `C1 Frame the problem and set the rules` · `B1 Establish context` | 0 |
| plan | the agent | `B2 Require step-by-step justification` | A |
| I read the plan | Sam reads it | `C3 Interrogate the claim` | B |
| recon | the agent | `B2 Require step-by-step justification` | C |
| build | the agent | `B2 Require step-by-step justification` | D |
| notebook | the agent | `B3 Build in self-questioning` | E |
| I play it | Sam plays it | `C4 Verify against hand-computed cases` | F |
| deploy | Sam pushes | `C4 Verify against hand-computed cases` | G |
| handoff | the agent | `C5 Consolidate in your own words` | H |

Three more labels do work here: `A0 Calibrate your reliance` in A.2, the autopsy steps `A1` to `A6` in E.2, and `B4 Plan the corrective dialogue` in F.3. Part J is `C6 Question the system`, turned on this brief.

Two loops run through the stages. A plan Sam does not approve goes back to the agent (**revise**). Anything play finds becomes a row that goes back into build (**triage rows**).

---

## Part 0 — The brief

`C1 Frame the problem and set the rules` · `B1 Establish context`

### 0.1 What we are making

A two-dimensional, one-against-one sword-fighting game for the browser. Each player controls only their fighter's arms, a few keys per joint, in the manner of QWOP. Everything in the fight is simulated: the arms, the sword and the bodies all move under one physics rule set, and nothing is animated by hand.

The sword is a solid object. It stops against the ground and against the other sword, so a player can plant it and push off, and can swing it to pull the fighter along. In this game a swing adds speed that comes from nowhere. That is deliberate, and it is the rule most likely to be "fixed" by mistake (A.2).

When a blade touches a fighter, it cuts straight across the part it touched and the part beyond the cut drops off. A cut across the neck or head, or across the heart, ends the round. Any other cut spills ink, and a fighter who runs out of ink loses the round. Ink is this game's blood. It is never red.

There are two ways to play: a story mode against opponents that get harder, and a versus mode against one friend.

### 0.2 Where the idea comes from, and what may not be taken

House plans cite the games they borrow from and say what is borrowed.

| Source | What is borrowed |
|---|---|
| QWOP (Bennett Foddy, 2008) | A body moved by holding keys that drive single joints. Here, arms only. |
| Toribash (Nabi Studios, 2006) | A fight decided by joint control and physics; parts that come off; a match shared as a replay file. |
| Nidhogg (Messhof, 2014) | Two players, one clean hit, an immediate reset, and the pleasure of playing beside a friend. |
| Getting Over It (Bennett Foddy, 2017) | A tool that is also the way you move. *This citation is the brief author's, not Sam's.* |
| Samurai Champloo (2004) | The vibe, and only the vibe. See below. |
| "Vagrancy", Force of Nature | The track Sam wants to hear while playing, and the word the game is named with. See 0.5. |

**Influence, not material.** The game may use these general ideas: an old road walked on foot, with present-day music culture stated flat beside it; episodic stops, each with one opponent; fighting styles as personalities, formal against improvised; and DJ vocabulary (sample, cut, crossfade) for mechanics. The game takes nothing specific from the show: no title or logo, no character names or likenesses, no episode titles, no dialogue, no plot elements, no three travelers, no music. `tests/copy.rs` carries a deny-list of the show's proper nouns and fails on any of them in `data/` or `web/`.

**The name.** Sam named the game Vagrancy. It is an ordinary word and it is also the title of the track above. The name is the whole of the reference: no string, credit or page element mentions the track, its artists or the show, and the game ships none of its audio (0.5).

### 0.3 Done

**MVP is finished when all of these are true in the deployed browser build:**

1. Four keys per fighter drive the shoulder and elbow of the sword arm, and no key drives any other joint.
2. The sword is solid against the ground and the other sword. A planted sword lifts the fighter, and swinging in the air adds momentum. Both are tests with numbers in them.
3. A blade that touches a body part cuts it at that position, and everything beyond the cut becomes a separate object that falls.
4. A round ends on a neck or head cut, a heart cut, or zero ink, and the result screen says which.
5. Nothing drawn for ink, cuts or fighters is red, and no string breaks `TONE.md`. Both are lints.
6. Story mode has a road of opponents whose order is measured against one yardstick, and progress is kept in a save file the player can download and load.
7. Versus works two ways: two players at one keyboard, and two browsers on two networks with no server of Sam's.
8. **Any match can be downloaded as a replay file and played back to the same final checksum in Chromium, Firefox and WebKit.** This is the hard gate. A build that cannot round-trip a replay is not shippable, whatever else works.
9. A player can load a music track from their own device and hear it loop during fights. No audio file is in the repository or the build without a row in `LICENSES.md`.
10. It is static files on GitHub Pages, in its own repository.

### 0.4 What the agent may not decide on its own

Each of these is Sam's. The agent raises it as a question and continues with independent work.

- Any change to the four decisions Sam has already made (0.7).
- Any audio file entering the repository or the build (0.5).
- Any change to a string in `data/copy.en.json` (0.6).
- Anything on the show's deny-list, or anything close enough that the agent has to argue it is not.
- A color for ink or a cut in the red range, for any reason.
- A dependency in `crates/sim` beyond `serde` and `postcard`, a float in `sim`, a server, a bundler, or an npm step.
- `git push`, `make publish`, or anything else that deploys.
- Whether the game feels right. The agent measures; Sam plays.
- Every item in Part I.

### 0.5 The music

Sam wants "Vagrancy" by Force of Nature to play during fights. It is a commercially released recording (the first track of *Samurai Champloo Music Record: Masta*, Victor Entertainment, 2004). The repository is public and the build is a public website, so shipping the file would be distributing it.

**The agent does not download, rip, record, transcribe or commit this recording or any part of it, and does not embed a stream of it.** A stream would also fail the gate, which refuses any request that leaves the origin.

What gets built instead is a **slot**: the player picks an audio file from their own device, and the page loops it during fights. The file is read by the browser and goes nowhere. Sam loads his own copy and hears the track he asked for; so can anyone else who owns it. If Sam later obtains a license, the file ships by adding one row to `LICENSES.md`, and nothing else changes.

A second guard keeps Sam's local copy out of the build: `web/assets/music/` is gitignored, and `package-web.sh` refuses to package any audio file that has no `LICENSES.md` row.

### 0.6 The words

Every string a player reads was written before the build, against `TONE.md`, and lives in `data/copy.en.json`. The agent implements those strings exactly. It does not rewrite, shorten or extend them. This is the website packet's rule, kept here for the same reason: the voice was authored and reviewed outside the implementation session.

Two cases need handling:

- **A string the build needs that is not in the file.** Write it against `TONE.md` with the file open, add it under the same key scheme with `"review": "new"` beside it, and list it in the milestone's final message. Sam accepts or replaces it.
- **A string that states something a decision has changed.** `_depends` in the copy file maps such strings to the decisions in A.3. If `PLAN.md` decides differently, list the string under open questions. Do not repair it.

### 0.7 What Sam has already decided

Sam was asked four questions before this brief was finished, and answered them. These are settled. The agent builds on them and does not reopen them; if recon finds that one of them cannot work as stated, that is a worklist row brought to Sam.

| Asked | Sam's answer | Where it lands |
|---|---|---|
| What is the game called? | Vagrancy. | `game.name`; the repository and the URL |
| Does a fighter's own blade cut that fighter? | Yes. | D11 |
| Do the legs step, or does all movement come from the sword? | The legs take slow steps. | D6, D8 |
| Is "free energy" the rule the brief wrote down? | Yes: arm motors push the arm and sword with no equal push back on the torso. | D9, H5 |

---

## Part A — Produce the plan (the agent's first job)

`B2 Require step-by-step justification`

The agent's first deliverable is `PLAN.md`. It writes no game code until `PLAN.md` is approved. Every decision in it carries the reason for it, and every factual claim about a source carries a file name and a line, a commit, or the command that produced it.

### A.1 Read the sources

Clone the reference repositories into `reference/` (gitignored). Borrow conventions and small pieces. Do not import a crate wholesale.

| Source | What to extract |
|---|---|
| `HOUSE-STYLE.md` | The whole file. It is the map of the rows below. If it and a repository disagree, the repository is right and the disagreement is a notebook row. |
| `sgilson7/floodline` | `crates/sim/src/fx.rs`, `rng.rs`, `World::checksum`; `crates/sim/tests/boundary.rs` and `determinism.rs`; all of `crates/net`; `web/quad_rtc.js`, `web/config.js`, `web/echo.html`, `web/vendor/README.md`; `floodline-design.md` §3.4, §8 and §9; the profile comments in `Cargo.toml`. |
| `sgilson7/gear-master-2d` | `packaging/package-web.sh`, `packaging/count-tests.sh`, `.github/workflows/`, `testing/drive.py`, the `Makefile`; the save code's exhaustive destructure; `crates/core/src/shot.rs` for integer physics; `SECOND-ORDER-M22.md` rows 1 to 3, 9, 10, 27, 35 and 36. |
| `sgilson7/gear-master` | `crates/console/src/verb.rs` for one input type shared by players and agents; `design/rl-agent-plan.md` §0 for how a brief's claims are corrected from the code. |
| `TONE.md`, `data/copy.en.json` | Every rule and every string. Note each `_depends` entry. |
| The lecture's Procedure A, B and C | The label table at the end of this brief. |

### A.2 Calibrate before deciding

`A0 Calibrate your reliance`

The lecture's claim is that reliability tracks how common a problem is, and that a rare problem gets filled in from the common one it resembles. Several parts of this game are rare, and three are common problems **perturbed on purpose**. For each row, `PLAN.md` states how much the agent expects to trust its first answer, and why. Revisit the table at the end of each milestone.

| Part of the game | How common | The textbook it will be filled in from | The small case that decides |
|---|---|---|---|
| Workspace, shim, packaging, CI | Common, and house code exists | (none expected) | The page prints its build hash |
| Lockstep for two | Uncommon; Floodline's code is the reference | Rollback netcode; an authoritative host | Two worlds, 10,000 ticks, equal checksums |
| Jointed bodies in integers | Rare | Float physics: `f32`, `sqrt()`, `sin()`, Rapier, Box2D | H1, H6 |
| A fast blade meeting a limb | Rare | Testing where things are at the end of a tick | H2 |
| Cutting a body into two objects | Rare | Hiding a sprite; breaking a ragdoll joint | H3 |
| Ink | Looks common | Summing every cut ever made | H4 |
| **A swing adds speed from nowhere** | Perturbed | Newton's third law; conservation of energy; damping "for stability" | H5 |
| **Arms only** | Perturbed | Full-body QWOP with thigh and calf keys | Test: only arm bits move a joint |
| **Ink is not red** | Perturbed | Red | The palette lint |
| Opponents | Rare | "Train a network" | The ladder table; search first |
| Strings | Common, in the wrong register | Game marketing voice | The `TONE.md` lints |
| Whether it is fun | Not something the agent can judge | | Sam plays |

### A.3 Make the design decisions the brief leaves open

`PLAN.md` must state a decision and a one-paragraph rationale for each row. The brief's recommendation is in italics. The agent may deviate from a recommendation, and must say why. Text marked **Decided by Sam** is not a recommendation: the plan restates it and builds on it.

| # | Decision | Recommendation |
|---|---|---|
| D1 | Crates and reuse | *`crates/sim` (the fight; `serde` and `postcard` only), `crates/content` (turns `data/*.json` into a `Setup`; may use `serde_json`), `crates/pilot` (opponents), `crates/net` (lockstep), `crates/wasm` (the shim), `crates/lab` (the bench; not shipped). Start `fx.rs`, `rng.rs`, the checksum and `net` from Floodline's, and packaging, the workflow and the gate from Gear Master 2D's.* |
| D2 | Drawing | *Canvas 2D from a vanilla ES module: the lecture's core, shim, page. Not macroquad, because the transport then lives in a plain module and the three-engine gate already assumes a DOM. The page may interpolate between two frames core sent. It may not extrapolate, integrate, or detect a contact.* |
| D3 | Numbers | *`i32` fixed point with 12 fractional bits, one unit a centimeter, every product through `i64`, overflow checks on in release. Integer square root and a sine table are allowed; they are integer functions with pinned test values. Recon M1.0 measures whether 12 bits is enough.* |
| D4 | Physics model | *Particles joined by rigid sticks, solved by a fixed number of relaxation passes in a fixed order (Jakobsen, "Advanced Character Physics", 2001; cited from memory, verify before relying on it). It needs no angles or inertia, a sword is one stick, and a cut is one new particle. No physics crate, for Floodline §3.4's reasons.* |
| D5 | Time | *Sixty ticks a second, fixed. The page owns the clock; `sim` has none.* |
| D6 | Input | *One byte per fighter per tick: shoulder up and down, elbow in and out, step left and right, two bits spare. `World::step([Input; 2])` is the only way the world changes. Default keys in `data/controls.json`: Q, W, O, P and A, D for one player; Q, W, E, R, A, D against U, I, O, P, J, L at one keyboard.* |
| D7 | Arms | *The lead arm is driven. The rear arm is simulated, undriven, and its hand is pinned to the hilt as a second grip, so cutting either arm leaves the other holding the sword. The wrist is stiff.* |
| D8 | Legs | **Decided by Sam (0.7).** Simulated, and held upright by a balance rule the player does not control. Two keys take a slow step. Everything fast comes from the sword. *How slow is a number for recon M1.0.* |
| D9 | The free-energy rule | **Decided by Sam (0.7).** An arm motor pushes the arm and the sword and is not paid back by an equal push on the torso, so a swing adds momentum to the whole fighter. Speed is bounded by a cap and by drag, never by conservation. *The rule is settled; its numbers are not. Recon M2.0 measures three tunings of it (motor strength, cap, drag) against H5 and the pogo test, and Sam chooses among them by playing.* |
| D10 | What the sword is solid against | *The ground, the arena walls and the other blade, all swept, so a fast blade cannot pass through any of them between two ticks. The hilt is the part of the sword that does not cut.* |
| D11 | The cut rule | **Decided by Sam (0.7): a fighter's own blade cuts that fighter.** *On the first tick a blade touches a body part, the part is cut straight across at the point on its spine nearest the contact, and everything on the side away from the heart becomes a separate object. One cut per blade, per part, per contact. The hands on the hilt are exempt, because a sword that cuts the hand holding it cannot be held.* |
| D12 | Ink and fatal zones | *`data/body.json` gives each part a drain rate for a stump on it and optional fatal zones along its spine. Any cut on the head or neck ends the round. A chest cut inside the heart band ends the round. Any other cut opens a stump that drains ink every tick. A stump removed by a later cut stops counting. Zero ink ends the round.* |
| D13 | A match | *First to three rounds. A round resets both fighters. If both fighters stop on the same tick the round is played again.* |
| D14 | Lockstep | *Floodline's star with two seats: the host relays, nobody simulates ahead, nobody rolls back. The delay is fixed per match from a measured round trip, between 2 and 8 ticks, sent in `Welcome` and shown to both players. Use the reliable ordered channel first, and move input to the unreliable channel only if M4.0 measures stalls.* |
| D15 | Files | *Replay v1 is `{format, version, sim_version, setup, inputs}`. Save v1 is `{format, version, state}` and holds road progress, bindings and options. Both use the house Blob download and file-input load. The `localStorage` copy is labeled a convenience.* |
| D16 | Opponents | *A pilot returns an `Input` and nothing else, so it holds nothing a player cannot press. Six kinds: still, pose-holder, loop, state machine, replayer (plays the player's previous round back, mirrored), and short look-ahead search on a cloned world. No learned policy in MVP: the house rule is that a search baseline must be beaten before any net.* |
| D17 | Look | *Flat paper figures on a paper ground, drawn as capsules from particle positions. No sprites. One fighter indigo and one ochre, told apart by a pattern as well as by hue. A fighter's ink is that fighter's color. `data/palette.json` is the only place a color is written.* |
| D18 | Sound | *The music slot from 0.5, in M2 so every playtest after it has music. Sound effects in M6, synthesized by the page from events core reports.* |

### A.4 Hand-computed cases

`C4 Verify against hand-computed cases`

Small cases computed by hand decide who is right. Each of these becomes a named test in the milestone shown, and `PLAN.md` restates each one with the agent's own arithmetic beside it. If the agent's arithmetic disagrees with the brief's, that is a notebook row and a question for Sam, not a silent correction in either direction.

**H1 — The order of the integrator (M1).** In a test world with no ground, no drag and the speed cap out of reach, a particle starts with downward speed 1 and gravity adds 4 a tick. Update speed first, then position. It falls 5, then 9, then 13, and after n ticks it has fallen 5 + 9 + 13 + … + (4n + 1) = n(2n + 3). At n = 2 that is **14**; at n = 10 it is **230**. Updating position first gives n(2n − 1), which is 6 at n = 2. The familiar sum n(n + 1)/2 gives 3. This is the lecture's own example (slide 18), and it is here because the wrong answers are the common ones.

**H2 — A blade that jumps a limb (M3).** A limb's spine runs from (4, 0) to (4, 10) with radius 1. A blade of length 10 lies along y = 5 with its tip at x = 0 on tick k, moving +20 a tick, in a test world whose speed cap allows it. On tick k the blade spans x = −10 to 0. On tick k + 1 it spans x = 10 to 20. It touches the limb on neither tick. **The cut must land at (4, 5), the middle of the spine.** A test of end-of-tick positions reports no cut. This is `gear-master-2d` `SECOND-ORDER-M22` row 1, "a one-tile wall is not a wall to a ball," in this game.

**H3 — Which side drops (M3).** A uniform limb of mass 8 is attached to the body at its near end and is cut a quarter of the way along from that end. **The fighter keeps 2 and the piece that drops has mass 6.** The piece that drops is always the one farther from the heart.

**H4 — A stump is replaced, not added (M3).** Ink is drained once at the end of every tick, including the tick a cut lands on. A fighter has 1,000 ink. A forearm stump drains 10 a tick and an upper-arm stump drains 20. The forearm is cut on tick 0. At the end of tick 29 the fighter has 700. The upper arm of the same arm is cut on tick 30, which removes the forearm stump. **The fighter reaches zero at the end of tick 64.** An implementation that adds the two rates says tick 53.

**H5 — Momentum from nowhere (M2).** In a test world with gravity off and no ground, a fighter starts at rest and one shoulder key is held for 60 ticks. **The fighter's total momentum, bodies and sword together, is not zero, and its kinetic energy is greater than at the start.** The textbook answer is that total momentum stays zero. Here the textbook is wrong on purpose.

**H6 — Integer helpers (M1).** The integer square root rounds down: 15 gives 3, 16 gives 4, 17 gives 4, and 2^62 gives 2^31. The sine table gives exactly 0 at 0°, exactly one at 90°, and exactly one half at 30°. A right shift of a negative number rounds toward negative infinity, as in Floodline's `fx.rs`.

### A.5 Write PLAN.md in this shape

```
1. Reliance, calibrated (the A.2 table, with the agent's column filled in)
2. Decisions D1 to D18 (decision, one paragraph of why, and what was rejected)
3. Repo layout and conventions (D.0)
4. Milestones M0 to M6 (Part D), each with:
     - Goal (one sentence)
     - Recon (what is measured first, and the guess it is checked against)
     - Deliverables (files and features, checkable)
     - Acceptance (test names as sentences, and what a person verifies)
     - Hand-computed cases that land here
     - Deployable? (what the deployed page lets a visitor do)
     - Risks specific to this milestone
5. The wire format, the replay format and the save format, each as an example
6. Data formats: body, controls, palette, road, pilots
7. Corrections to the brief, from the code (what the sources say that this brief got wrong)
8. Open questions for Sam (Part I, plus anything the agent could not decide)
```

Section 7 is not optional. This brief was written from a reading of the repositories and not from running them (Part J), so some of what it says about them is wrong. `gear-master/design/rl-agent-plan.md` §0 is the model: the brief's claims are listed, checked against the code, and corrected by line.

Then stop and present it.

---

## Part B — I read the plan

`C3 Interrogate the claim`

Sam reads `PLAN.md` and approves it or sends it back. The agent makes that reading possible:

- **A reference the agent cannot find is a question for Sam, not a guess.** If a file, function, paper or library version named in this brief or wanted by the plan cannot be located, it goes in section 8.
- Every claim about a source names where it was read. Every number names the command that produced it, or is marked as a guess.
- Every decision that departs from a recommendation says so in its first sentence.
- When Sam sends the plan back, the agent revises the plan. It does not start building the parts that were not questioned.

After approval, `PLAN.md` wins where it and this brief disagree, and each divergence is a row in the notebook.

---

## Part C — Recon

`B2 Require step-by-step justification`

Every milestone begins by measuring. Recon is done against the running build, before anything is authored on top of a guess, and what it found goes in the commit message. A number that disagrees with the plan must name which of the two is wrong.

The brief's guesses are below so that they can be checked. They have never been measured.

| Recon | What is measured | The brief's guess |
|---|---|---|
| M1.0 | How far a fighter left alone for 600 ticks drifts, at 8, 12 and 16 fractional bits | 12 bits holds it within a centimeter |
| M1.0 | The cost of one tick, natively and in wasm, for two fighters | Under 50 microseconds natively |
| M1.0 | How far one step moves a fighter, and how long it takes, at three step speeds | A step that crosses the arena in about ten seconds reads as slow |
| M2.0 | For three tunings of the motor rule: the height of a pogo, the ticks to reach the speed cap by swinging, and the distance one swing carries a fighter in the air | One tuning gives a pogo of one to two body heights and reaches the cap in two to four seconds |
| M2.0 | How often 100 runs of random input end with a fighter cutting itself | Unknown. Self-cuts are intended; if nearly every run ends in one, whether to add a minimum cutting speed is a worklist row for Sam |
| M3.0 | How many substeps a swept blade needs at the speed cap | Four or fewer |
| M4.0 | Round trip and stall rate between two real home networks. **This needs Sam and a friend.** | 30 to 80 ms, which makes the delay 3 to 6 ticks |
| M5.0 | Headless ticks per second, and what search depth fits inside one tick | 200,000 ticks a second or more; a search of a third of a second |

---

## Part D — Build

`B2 Require step-by-step justification` · `C2 Elicit one step with its rule named`

One milestone at a time, in this order. Each commit names the decision or the plan section it implements. A milestone is not complete until `make test` passes, `make web` builds, `make test-ui` walks the gate in three engines, and the deploy gate is live and Sam has seen it.

### D.0 Repo layout and conventions

```
vagrancy/
  PLANNING-BRIEF.md   this file
  PLAN.md             the agent's plan (Part A output)
  CLAUDE.md           the rules, kept short
  DECISIONS.md        why things are the way they are, one paragraph each
  TONE.md             the voice
  HOUSE-STYLE.md      what the reference repositories do
  MILESTONES.md       the status table, appended after every milestone
  SECOND-ORDER-M<n>.md  the notebook for milestone n
  LICENSES.md         one row per third-party file in the build
  Cargo.toml          workspace
  crates/sim/         the fight. serde + postcard. no floats. no clock.
  crates/content/     data/*.json -> Setup
  crates/pilot/       opponents: (&World, seat, &mut state) -> Input
  crates/net/         Peer, Loopback, wire, lockstep
  crates/wasm/        the shim. decides nothing.
  crates/lab/         the bench: ladder, replay runner, recon. not shipped.
  web/                index.html, app.js, rtc.js, styles.css, vendor/
  data/               copy.en.json, body.json, controls.json, palette.json,
                      road.json, pilots.json
  testing/            drive.py and the browser checks; replays/ for reports
  packaging/          package-web.sh, count-tests.sh
  Makefile            make test | check | web | serve | test-ui | ladder | publish | help
```

The rules that hold across every milestone are in `CLAUDE.md` and are not repeated here. A rule with two homes is a rule with two answers.

### M0 — Foundation, and the voice in the repo

**Goal:** an empty page deploys and names its build, and every lint that guards the words and the colors exists and has been seen failing.

- **Deliverables:** the workspace with all crates compiling empty; the house profiles in `Cargo.toml`, comments included; `Makefile`; `package-web.sh` with the lockfile-pinned `wasm-bindgen` and a content stamp on everything the browser caches; the Pages workflow running test, build, gate, deploy; a page that shows the entry-point strings from `data/copy.en.json` and the build hash; `tests/boundary.rs`; `tests/copy.rs` (the `TONE.md` lints and the show deny-list); `tests/palette.rs`; `packaging/count-tests.sh`; `LICENSES.md`; `MILESTONES.md`.
- **Acceptance:** `sim_depends_on_serde_and_postcard_and_nothing_else`; `sim_has_no_float_no_hashmap_and_no_clock`; `every_string_a_player_reads_is_in_the_copy_file`; `no_string_breaks_the_tone_file`; `no_ink_or_cut_color_is_red`; the gate passes in three engines with no console error and no request that leaves the origin. Each lint was broken once and watched failing.
- **Deploy gate 1:** a visitor sees what the game is, a loading line, and the build hash.

### M1 — A body in integers, and a match you can play back

**Goal:** one fighter stands, its arm answers four keys, and the same inputs give the same world everywhere.

- **Recon:** M1.0.
- **Deliverables:** `Fx`, the integer square root, the sine table, `Rng`; `Setup` and `World`; `World::step` as the only door; particles, sticks and the fixed-order solver; the ground; the balance rule; arm motors; the checksum; `SIM_VERSION`; replay v1 with download and load; the shim's `step`, `frame`, `checksum`, `replay_bytes`, `load_replay`; a page that draws what `frame` returns.
- **Acceptance:** H1 and H6; `two_worlds_fed_the_same_inputs_agree_for_ten_thousand_ticks`; `only_arm_bits_move_a_joint`; `a_replay_played_back_ends_on_the_recorded_checksum`; `a_replay_from_another_sim_version_is_refused_with_a_sentence`; `a_damaged_replay_is_refused_rather_than_half_loaded`; `a_golden_replay_ends_where_it_always_has` (if it moves on purpose, bump `SIM_VERSION`, re-record, and say so in the commit). In the gate: the native checksum of a fixed input script equals the wasm checksum in all three engines.
- **Deploy gate 2:** move the arm, download the replay, reload, load it, and watch the same motion. Boring on purpose.

### M2 — The sword is solid, and a swing is free

**Goal:** a fighter can pogo, and can build speed by swinging, in a practice yard with music.

- **Recon:** M2.0. Sam chooses the tuning after playing the candidates.
- **Deliverables:** the blade as a swept stick against the ground and walls; friction; the motor rule; the speed cap and drag in `sim::balance`; the practice yard and its five steps from the copy file; the music slot (file input, loop during play, volume, the optional remembered copy, the `LICENSES.md` guard in packaging).
- **Acceptance:** H5; `a_planted_sword_lifts_the_fighter`, with the measured height in it; `speed_never_passes_the_cap` over 100 seeded runs of random input; `nothing_leaves_the_arena`; `no_audio_ships_without_a_license_row`. In the gate: a loaded track plays, and no request leaves the origin while it does.
- **Deploy gate 3:** a visitor can swing, pogo, and cross the yard by swinging.

### M3 — Cuts, ink, and two fighters at one keyboard

**Goal:** two people at one keyboard can play a whole match.

- **Recon:** M3.0.
- **Deliverables:** the second fighter; blade against blade; the swept contact; the cut rule; splitting a part and releasing the far side; stumps and ink; fatal zones from `data/body.json`; rounds and a match; an event list out of `step` (cut, clash, landing, round end) for the page to draw and later to sound; the result strings; the palette and the patterns; key bindings in Settings.
- **Acceptance:** H2, H3 and H4; `a_cut_on_a_piece_that_has_dropped_spills_nothing`; `the_hands_on_the_hilt_are_safe_from_their_own_blade`; `a_neck_cut_ends_the_round_on_the_tick_it_lands`; `two_blades_never_pass_through_each_other` over seeded runs of random input; `the_result_names_the_cut_that_ended_the_round`. In the gate: a scripted match runs to a result in three engines, and its replay round-trips.
- **Deploy gate 4:** the first build worth playing beside a friend.

### M4 — Lockstep, on a loopback and then between two browsers

**Goal:** two browsers on two networks play a match that stays one match.

- **Part one, no browser.** `Peer`, `Loopback`, the wire format, the two-seat lockstep, the delay, the stop on a checksum mismatch, the drop on silence. Proven in `cargo test` with latency and jitter injected, before any transport exists.
- **Part two, the transport.** `web/rtc.js`, taken from Floodline's `quad_rtc.js` and made a plain ES module; the Trystero bundle vendored and pinned by name and sha256; `web/echo.html`; the room code path and the pasted-code path; the build hash in the room name; the lobby and its strings.
- **Recon:** M4.0, with Sam and a friend.
- **Acceptance:** `an_input_takes_effect_delay_ticks_later_on_both_peers`; `a_mismatch_stops_both_peers_on_the_same_tick_and_names_it`; `a_silent_peer_is_waited_for_and_then_dropped`; echo checks for both paths; a two-tab match; a referee script that watches both peers of a scripted three-minute match and writes down whether they stayed together. **And Sam plays one real match across two networks.**
- **Deploy gate 5:** send a friend a link and fight them.

### M5 — The road (MVP complete)

**Goal:** a player can walk the road from the first opponent to the last.

- **Recon:** M5.0.
- **Deliverables:** the six pilot kinds; `data/pilots.json` and `data/road.json`; `make ladder`, which plays the yardstick pilot against every opponent and writes `analysis/ladder.md`; the road screen and the opponent introductions from the copy file, with their numbers filled from pilot data; save v1 with download, load and the labeled convenience copy.
- **Acceptance:** `a_pilot_returns_an_input_and_nothing_else`; `the_road_is_ordered_by_the_yardstick` (the yardstick's win rate does not rise from one stop to the next, over 200 seeded matches each); `every_opponent_can_be_beaten` (by the yardstick at its strongest, or by a recorded human replay in `testing/replays/`); `every_number_in_an_introduction_comes_from_the_pilot_data`; the save round-trip tests. The MVP checklist in 0.3 is walked by hand and every line is a yes.
- **Deploy gate 6:** tag `v0.1.0-mvp`.

Road order is a measurement and the copy does not depend on it. If the ladder disagrees with the order in `data/road.json`, reorder the data. A rating predicts nothing about whether a fight is winnable, so the order Sam's play reports outranks the ladder's.

### M6 — Sound, figures, and a pass over the page (after MVP)

- Sound effects synthesized from the event list. No audio files.
- Backdrops, if wanted, as TikZ sources compiled to SVG with the house prompt. Geometric placeholders until then.
- A second pass over every string against `TONE.md`, and an accessibility pass: focus order, the canvas name, the reduce-motion setting, contrast of both fighters on the ground color.
- Deployable, but not a version bump.

### What this brief deliberately leaves

Costed here so the next plan does not measure them again.

- **Rollback.** The world already encodes to bytes for the checksum, so snapshot and restore are cheap and the door is open. It is not in MVP, because the house lockstep is proven and rollback is not.
- **A learned opponent.** After the search pilot is measured, and trained in `crates/lab`, never in CI.
- **More than one arena, or a scrolling stage.**
- **Gamepads.**
- **Spectators and more than two players.**

### The status table

After every milestone's commit, append this table to `MILESTONES.md` and print it as the last thing in the milestone's final message. Sam reads this table to know where the work is.

```
| # | milestone | deliverables | tests | commit | state |
|---|---|---|---|---|---|
| M0 | Foundation, and the voice in the repo | … | 41 (+41) | a1b2c3d | done |
| M1 | A body in integers | … | | | in progress |
| … | | | | | |
| notebook | rows open / done / the human's | 0 / 0 / 0 | | | |
| new strings | awaiting Sam | 0 | | | |
```

`tests` is the number from `packaging/count-tests.sh` and its change from the row above. It is never read off `cargo test`'s output, which interleaves and cannot be summed.

---

## Part E — The notebook

`B3 Build in self-questioning` · `C3 Interrogate the claim`

### E.1 Keep it while the work happens

`SECOND-ORDER-M<n>.md`, one per milestone, written the moment something is noticed and never saved up for the end. One row per assumption.

```
| # | row | kind | status |
|---|---|---|---|
| 1 | **Bold first sentence that states what was seen.** Then the measurement, the file, and what it changes. | divergence | done |
```

`kind` is one of three:

- **divergence** — the build and the plan disagree. Say which is wrong.
- **finding** — something true about the game that nobody had written down. It is closed by writing it into `DECISIONS.md` or by a test.
- **worklist** — a change that reaches further than this milestone, or a decision that is Sam's. It is brought to Sam and not taken.

`status` is `open`, `done`, or `the human's`. A row is a sentence about what was seen, not a plan to fix it.

### E.2 A divergence gets an autopsy

When the build disagrees with the plan, the row is written by walking the lecture's Procedure A. The labels go in the row.

| Label | In this repo |
|---|---|
| `A1 Restate the claim as given` | Quote the plan's sentence or the test's assertion exactly, numbers included. |
| `A2 Extract the rule the chatbot actually used` | Find the rule the code implements. Quote the line. |
| `A3 Locate the first divergence` | The first line, or the first tick, where the two part ways. For a desync or a replay that drifts, compare per-tick checksums and name the first tick that differs. |
| `A4 Test a small case` | Run the nearest hand-computed case from A.4. If the numbers agree, check one step of the derivation anyway. |
| `A5 Classify the failure mode` | Axiom reversion, invalid syntactic shortcut, or spatial / counting miscount. |
| `A6 Explain why the failure was likely` | Connect it to A.2: which common problem was this one filled in from? |

The three failure modes, as they appear in a game:

- **Axiom reversion.** The build swaps in the textbook rule the game was perturbed from. *Indicator:* the code matches a physics tutorial and not the plan. A motor that pushes back on the torso, a damping term, an `f32`, a thigh key, a red particle, a prediction step in the netcode.
- **Invalid syntactic shortcut.** The form is right and the claim is false. *Indicator:* the check cannot fail. An assertion inside an `if` that never runs, a test that compares zero with zero, a checksum over a hand-picked list of fields, a `..` in a destructure, a lint that reads the source text instead of the behavior.
- **Spatial / counting miscount.** Positions or totals are wrong. *Indicator:* the parts do not sum to the whole. A blade that passes a limb between ticks, the wrong side of a cut dropping, two stumps counted on one arm, an off-by-one in the delay, a test total read from interleaved output.

### E.3 Traps the house has already paid for

Each of these cost a day in another repository. Each has a version here.

| The house's version | This game's version | Mode | The guard |
|---|---|---|---|
| A ball passes through rock (`SECOND-ORDER-M22` row 1) | A blade passes through a limb, a blade or the ground between ticks | Spatial / counting miscount | Swept contact; H2 |
| A check that compares zero with zero (rows 10, 35, 36) | A cut test whose blade never arrives | Invalid syntactic shortcut | Break every new test and watch it fail |
| A second copy of a constant (`CLAUDE.md`, Rules) | The page's own tick rate, rounds to win, or key names | Spatial / counting miscount | The page reads the payload |
| 1,137 tests that were 1,037 (lecture slide 36) | Any count in a document | Spatial / counting miscount | `count-tests.sh`; `make ladder` |
| Overflow means two things in two builds (Floodline `Cargo.toml`) | A fixed-point product at the speed cap | Invalid syntactic shortcut | Overflow checks on in release; `speed_never_passes_the_cap` |
| A field the save forgot (`save.rs`) | A field the replay or the checksum forgot | Invalid syntactic shortcut | Hash the whole encoding; destructure exhaustively |
| `git checkout` on a file with uncommitted work (row 9) | The same | | Copy the file before breaking it |
| A deployed fix a returning player cannot receive (`package-web.sh`) | The same | | Stamp everything the browser caches |

---

## Part F — I play it

`C4 Verify against hand-computed cases`

### F.1 What Sam does

At each deploy gate from the third on, Sam plays the deployed build against what the plan claimed. Play is the hand-computed case for everything the agent cannot check: whether a swing feels like a swing, whether the pogo is worth using, whether an opponent teaches what its introduction says it teaches. The lecture's version is a fortnight of real play. Everything wrong becomes a row.

A report is written from the screen, not from the source, in `PLAYTEST-M<n>.md`. Wherever possible it comes with a **replay file** saved from the result screen into `testing/replays/`. The simulation is deterministic, so a replay is the whole of a bug report.

### F.2 What the agent does with a report

For each reported moment the agent first reproduces it by running the replay headlessly in `crates/lab`, and writes down the tick at which it happens. Then it writes the triage row:

```
| # | severity | what | cost | disposition |
```

- **severity** is about the player: *blocks*, *wrong but survivable*, or *cosmetic*.
- **cost** is one of: content, a number, a function, or a design decision that is the human's.
- **disposition** is fixed, carried, or declined, with the reason.

Blockers are fixed before the next milestone starts. The rest are carried openly.

### F.3 What Sam will say when something is wrong

`B4 Plan the corrective dialogue`

The corrections are written in advance, so that an error meets a prepared sentence and not an improvised one. When the agent reads one of these, it answers the question asked before it changes any code.

| When Sam sees | Sam says |
|---|---|
| A float, a clock or a new dependency in `sim` | "`crates/sim` has `<what>` at `<file:line>`. Remove it, make `boundary.rs` catch it, and write the row: which textbook did this come from?" |
| A test that has never been seen failing | "Show me this test failing. Break the rule it guards, run it, paste the red line, and restore the file from the copy you took." |
| A number with no command behind it | "Which command produced this number? If none did, it is a guess. Mark it as one or measure it." |
| A string that differs from the copy file | "This string is not the one in `data/copy.en.json`. Restore it, and put your wording in the notebook as a worklist row." |
| A swing that conserves momentum | "The motor now pushes back on the torso. D9 says it does not. Restate the rule as given, show me the line where the build parts from it, and run H5." |
| A reference that cannot be found | "You cited `<name>` and I cannot find it. A reference you cannot find is a question for me, not a guess. Where did it come from?" |
| A fix for a bug the report did not have a replay for | "What tick does this happen on, in which replay? If there is no replay, make one first." |

---

## Part G — Deploy

`C4 Verify against hand-computed cases`

- **The agent never pushes on its own judgement.** It does not run `git push` or `make publish`, even when the work is green and Sam is clearly going to want it. The exception is an explicit ask, and it is never inferred.
- Sam pushes. The workflow tests, builds, walks the gate in three engines, and deploys.
- **A deploy is not finished until the gate has walked the live page.** `ORIGIN=https://sgilson7.github.io/vagrancy/ testing/drive.py chromium firefox webkit`, every check, exit 0, with the build hash on the page matching the one that was pushed. A deployed fix is not a delivered fix.
- A locally built page and the deployed one cannot join each other online, because the room name carries the build hash. That is the guard working.

---

## Part H — Handoff

`C5 Consolidate in your own words`

`HANDOFF.md` is written at every deploy gate, and whenever the session's context reaches about 700,000 tokens, whichever comes first. Past that point the lecture's observation is that the model's work starts to degrade, and the handoff is what lets a cleared session continue.

It is written for a reader who has none of this session's context, in the agent's own words and not by pasting the plan:

1. What this is, in five sentences.
2. The rules that are load-bearing, and what breaks silently when each is broken.
3. The shape of the code.
4. The commands.
5. What will bite within the hour.
6. The mistakes that cost a day, each filed under the system it happened to.
7. The single next action.

History goes in `DECISIONS.md` and the notebooks. `CLAUDE.md` stays short enough to read in a minute (`HOUSE-STYLE.md` §12).

---

## Part I — Open questions for Sam

Put these in `PLAN.md` section 8 with the agent's recommendation beside each. None of them blocks M0 or M1. The four questions that did block a milestone are answered in 0.7.

1. **Is a cut across the waist fatal at once, or does it drain ink very fast?** The brief recommends draining, so the heart and the neck stay the only instant endings. (Wanted before M3.)
2. **Is the fluid called ink or blood in the text?** The copy says ink, and says once that it stands in for blood.
3. **Should the entry page carry the content note in `game.content_note`?** The brief recommends yes: the site it sits beside is a research and teaching site.
4. **Are public relays acceptable for introductions?** They are what Floodline uses. A direct connection shows each player the other's IP address, and the copy says so. (Wanted before M4.)
5. **The About link.** House games link to `/projects/<name>/` on the main site. That page does not exist yet. The brief recommends leaving the link out until it does, since a link to a missing page is a defect.
6. **May `TONE.md` be public?** It was derived from the editorial and voice guide, which is private. `TONE.md` contains the derived rules and none of the guide's transcript evidence. If it should stay private too, gitignore it and keep the lints.
7. **A license for the track.** If Sam obtains one, the file ships with a `LICENSES.md` row. Until then the slot is the answer.
8. **How many rounds win a match, and how many opponents are on the road?** The brief recommends three and eight. (Wanted before M3 and M5.)

---

## Part J — Question the system

`C6 Question the system`

The lecture ends by asking who built the system, on whose work, and what you would want to know before leaning on it. Those questions apply to this brief.

- **Who wrote it.** Claude, in a chat session, from Sam's description of the game, the five repositories at the commits listed in `HOUSE-STYLE.md`, the lecture deck, the voice guide and the website packet.
- **What it did not do.** It compiled nothing, ran no test, played no game, and heard no music. It has not built a fixed-point jointed body, and neither has anything in the house.
- **Where it is strongest.** The process, the document shapes and the conventions, which were read directly from the repositories.
- **Where it is guessing.** Every number in Part C. The physics model in D4, including its citation. Whether six keys per player survive a real keyboard. Whether the pilots in D16 can be made to fight well with an arm they must drive key by key. Whether a fight in which a fighter's own blade cuts them is playable before anyone has practiced.
- **What to check before leaning on it.** Recompute H1 to H4 by hand. Play the M2.0 tunings before accepting the numbers behind D9. Read section 7 of `PLAN.md`, where the agent corrects this brief from the code, before reading anything else in it.
- **Did the confident tone mislead?** The recommendations are written as plain statements because the house style asks for that. A plain statement here is a proposal, and A.3 says the agent may deviate.

---

## The labels

Same label, same words, every time it appears.

| Label | What the step is for |
|---|---|
| `A0` | Calibrate your reliance |
| `A1` | Restate the claim as given |
| `A2` | Extract the rule the chatbot actually used |
| `A3` | Locate the first divergence |
| `A4` | Test a small case |
| `A5` | Classify the failure mode |
| `A6` | Explain why the failure was likely |
| `B1` | Establish context |
| `B2` | Require step-by-step justification |
| `B3` | Build in self-questioning |
| `B4` | Plan the corrective dialogue |
| `C1` | Frame the problem and set the rules |
| `C2` | Elicit one step with its rule named |
| `C3` | Interrogate the claim |
| `C4` | Verify against hand-computed cases |
| `C5` | Consolidate in your own words |
| `C6` | Question the system |
