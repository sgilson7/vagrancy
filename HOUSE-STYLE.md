# House style of game development: the sgilson7 repositories

An analysis of how the games and tools linked from <https://sgilson7.github.io> are built, written so that a new game can be built the same way. `PLANNING-BRIEF.md` cites this file by section instead of repeating it.

## What was read, and what was not

| Repository | Commit read | Dated | What it is |
|---|---|---|---|
| `sgilson7/gear-master` | `1deaa69` | 2026-10-01 | Gear auto-battler; engine, agents, the Q-learning write-ups |
| `sgilson7/gear-master-2d` | `23a135e` | 2026-10-01 | Open-world RPG on the same engine; the fullest document set |
| `sgilson7/floodline` | `e18601b` | 2026-10-01 | Multiplayer city builder; lockstep over WebRTC, no server |
| `sgilson7/pdf-redactor` | `6a7a065` | 2026-10-01 | Browser tool; the first core, shim, page build |
| `sgilson7/perturbation-workbench` | `3cc83a6` | 2026-10-01 | Browser tool; the same pattern |
| `sgilson7/sgilson7.github.io` | `864777d` | 2026-10-02 | The built site (generated output only) |

Also read: the CSC 484 guest lecture deck (55 slides), `Website-Voice-Guide.md`, and the website rework packet.

**Not done.** Nothing was compiled, no test suite was run, and no game was played. Every count below is quoted from a repository's own document or commit message and says which one. Line counts are `wc -l` over `.rs` files. Treat any number here as a quotation, the way `gear-master/CLAUDE.md` treats its own: if the tip has moved, the command is the measurement.

## The style in one paragraph

A game is a deterministic simulation in a Rust crate with no graphics, no clock and almost no dependencies. A thin layer carries its numbers to a page that draws them and decides nothing. The result is static files on GitHub Pages, with no server and no account. Content is data and state is a file the player can download. A claim about the game is expected to be a test, a lint, or a number produced by a command, and the documents record what was measured, what the plan got wrong, and what each mistake cost. An agent plans, builds, tests and writes up. Sam reads, plays, decides and deploys.

## 1. Architecture

| Repository | Crates | Logic crate depends on | Drawn by | Size of the logic crate |
|---|---|---|---|---|
| gear-master | `engine`, `console`, `cli`, `gui`, `agent`, `oracle`, `trades`, `lab` | nothing | macroquad | about 79k lines, tests included |
| gear-master-2d | `core`, `wasm`, `lab` | `serde`, `serde_json` | vanilla JS on canvas | 60.7k in `src`, 38.1k in 95 test files |
| floodline | `sim`, `net`, `net-web`, `gui` | `serde`, `postcard` | macroquad | 11.9k in `src`, 8.7k in tests |
| pdf-redactor | `core`, `wasm` | `serde`, `unicode-normalization` | vanilla JS | 3.0k |
| perturbation-workbench | `core`, `cli`, `wasm` | `serde` | vanilla JS | 8.1k |

There are two drawing lineages. Gear Master and Floodline use macroquad. The two tools and Gear Master 2D use the pattern the lecture names as the house architecture (slide 8): a Rust core, a `wasm-bindgen` shim, and a hand-written ES module page with no bundler, no node and no npm. `gear-master-2d/packaging/package-web.sh` says the same in its header: the tools "agree with each other; gear-master's own web build predates both and ships macroquad."

Four rules hold in every repository, whichever lineage it is in:

1. **The logic crate imports nothing that draws.** Gear Master 2D's rule 1 is that `crates/core` never imports `wasm-bindgen`, `web-sys` or anything DOM-shaped. Floodline enforces its version with a test, `tests/boundary.rs`, which reads `sim`'s own `Cargo.toml` and names the crates whose arrival would mean the boundary is gone (`rapier2d`, `rand`, `hashbrown`, `wasm-bindgen` among them).
2. **The shim decides nothing.** "A rule decided there is a rule the test suite cannot reach in seconds, and then there are two rulebooks" (`gear-master-2d/CLAUDE.md`, Rules).
3. **The page draws numbers the core sent and never recomputes one.** `gear-master-2d/CLAUDE.md` records three violations, each invisible until found.
4. **The world changes through one door.** Gear Master has a `Verb` enum, "everything a player can do, and nothing else"; Floodline has `World::apply(player, Command)`. Agents and bots hold the same type a player does, so they cannot do what a player cannot.

Supporting habits recur in the manifests and scripts: `opt-level = "z"`, LTO and one codegen unit for release; no `panic = "abort"`, because a browser then reports only "unreachable"; test binaries with line tables only and `opt-level = 2`; the `wasm-bindgen` CLI version read from `Cargo.lock`; and a content hash stamped into every asset URL because GitHub Pages caches for ten minutes. Each of these carries a comment giving the incident that produced it.

## 2. Determinism

Determinism is the property the rest depends on.

- **No floats where two machines must agree.** Floodline's `fx.rs` is an `i32` with 8 fractional bits, with every multiply and divide passing through `i64`. Gear Master 2D's `shot.rs` runs a billiard shot in sixteenths of a tile with angles from a table, "no `sqrt`, no `f32` and no trigonometry at runtime." Both give the same reason: float rounding is the one thing that makes two peers running the same code disagree.
- **No physics crate.** `floodline-design.md` §3.4 decides on "a purpose-built fixed-point physics layer inside `sim`, not Rapier" and says why.
- **One random stream.** A single xorshift64* `Rng` lives in the world. Floodline's version takes its uniform draw by modulo instead of rejection so that the number of draws never depends on the values drawn.
- **No `HashMap`, no clocks.** Iteration order is a decision, so collections are `Vec` or `BTreeMap`.
- **Overflow means the same thing in every build.** Floodline turns `overflow-checks` on in release, because a debug native peer would panic where a release wasm peer wrapped, "which is not a crash, it is a desync."
- **The checksum is the whole world.** FNV-1a over the `postcard` encoding of `World`, so a new field is covered the day it is added (lecture slide 44).
- **Combat has no random numbers** in Gear Master: a fight is a pure function of what was packed.

## 3. Multiplayer

Floodline is the only networked project and its design is specific:

- A **star**: every joiner connects only to the host; the host relays. The host is "a relay with a clock, not an authority."
- **Lockstep with a fixed delay.** Twenty ticks a second, commands take effect three ticks later, every peer advances only on the host's bundle. "Nobody simulates ahead and nobody rolls back."
- **No server of Sam's.** Trystero over public Nostr relays introduces two browsers, and a pasted code replaces the relays when they fail. The Trystero bundles are vendored and pinned by sha256.
- **The room name carries the build hash**, so two builds cannot half-join.
- **Lockstep was proven on `net::Loopback` first**, with no browser, "so a networking regression and a lockstep regression can never be mistaken for one another." A separate `echo.html` exercises the transport with no game in it.
- **A mismatch stops the game** on every peer at the same tick and names the peer and the tick.

The public description of this is careful about its claim. The site's project page says connection setup "can involve external infrastructure, so 'no dedicated simulation server' does not mean 'no network services.'"

## 4. Content, strings and tone

- **Content lives in `data/*.json`.** "If you are editing a `.rs` file to change what a player reads, you are in the wrong file."
- **A tone file governs every string.** `gear-master-2d/TONE.md` has fifteen rules, and each ends in a *Check* that can be answered yes or no about one sentence: "if a reviewer cannot point at a clause and rule on it, it is not a rule."
- **A mechanical description is derived, not typed.** TONE rule 13a exists because the skill tree "shipped eight nodes whose blurbs described armour and mana they did not grant."
- **A refusal names the thing in the way**, in one sentence.
- **The website packet goes further.** There, copy is authored outside the implementation session and supplied as exact JSON; the implementing agent may not write, paraphrase or extend it, and reports a gap instead of filling it.
- **A game's page has a fixed entry point**: one sentence saying what the game is, an About link, loading, error and fallback lines, and an accessible name on the canvas (`runtime-copy.json`, `game_locks`).

## 5. Testing

The lecture's slide 29 lists four gates, and the repositories match it.

1. **Unit tests on the core, in seconds.** Gear Master 2D's suite is quoted as 1,144 tests in 96 binaries (commit `d390ee5`); Floodline's README says its whole suite takes about twelve seconds.
2. **Lints on the content, not just the code.** Tests read the data files: every gate lands beside its door, every town sells something, no sentence has a gap in the middle.
3. **Save round-trip on every commit.** "A red round-trip blocks everything." Adding a field to `Game` is a compile error until the save carries it, because the save code destructures exhaustively and `..` is forbidden.
4. **A browser gate in three engines, then the same gate against the live page.** `testing/drive.py` walks the page with Playwright in Chromium, Firefox and WebKit, prints one `ok:` line per check (114 at commit `8a23dfa`), and fails on any console error or any request that leaves the origin, "so 'nothing is uploaded' is tested rather than asserted."

Habits that go with the gates:

- **Test names are sentences**: `a_damaged_file_is_refused_rather_than_half_loaded`, `the_random_stream_resumes_where_it_was_saved`.
- **A check nobody has seen fail is not a check.** Each new test is broken on purpose and watched going red. The notebooks record several that could not fail ("compares zero with zero").
- **One source for every count.** `packaging/count-tests.sh` exists because `cargo test` interleaves its output and three parsings gave three totals.
- **Agents play the game without reading the source.** `testing/agent_driver.py` and Floodline's `driver.py`, `two_agents.py` and `referee.py` give an agent hands and eyes, and an `AGENT-BRIEF` holds everything the playing agent is told.
- **A second kind of walker** (`make play`) plays as a player would and writes a transcript, because it finds what a route written to exercise checks does not.

## 6. Documents and process

The lecture's slide 28 lists the artifacts. Gear Master 2D has all of them at its root.

| Artifact | Written by | Holds |
|---|---|---|
| `PLANNING-BRIEF.md` | Sam | What is being made, what done means, what the agent may not decide |
| `CLAUDE.md` | both | The load-bearing rules, and the mistakes that cost a day |
| `TONE.md` | Sam | Checkable rules for every string a player reads |
| `PLAN.md`, `PLAN-M<n>.md` | the agent | Each decision with a paragraph of why |
| `PROMPT-M<n>.md` | Sam | Reading order, the ask verbatim, recon first |
| `SECOND-ORDER-M<n>.md` | the agent | Every assumption, checked while the work happens |
| `PLAYTEST-M<n>.md`, `TRIAGE-M<n>.md` | Sam, or a playing agent | What play reported, sorted into rows |
| `HANDOFF.md` | the agent | What the next reader needs before touching anything |
| `MILESTONES.md` | the agent | A status table, appended after every milestone |

How they are used:

- **No code before the plan is read.** The first prompt is fixed: "Execute Part A to produce PLAN.md. Stop and show me PLAN.md before executing any milestone."
- **A brief leaves decisions open on purpose** and gives a recommendation for each; the agent may deviate and must say why. After approval, the plan wins over the brief and the divergence is recorded.
- **Recon comes before content.** A block's first milestone is often only measurement. `PLAN-M22.md` opens with a table of what the plan said, what was measured, and what that changed, under the sentence "a plan that was wrong and was corrected is worth more than one that was never checked."
- **The notebook is written during the work**, one row per second-order effect, a bold first sentence, a state. Rows that are the human's decision are carried as questions, not taken.
- **Triage rows have a severity about the player** (blocks; wrong but survivable; cosmetic), a cost (content, a number, a function, or a design decision that is the human's) and a disposition.
- **Plans cite the games they borrow from**, by name and by what is borrowed.
- **A handoff is written when the context is long** (about 700k tokens, slide 37) so a cleared model can continue.

## 7. Roles and deployment

The agent plans, builds, tests and writes up. Sam reads, plays, decides and deploys (slide 37). The agent "does not run `git push` or `make publish` on its own judgement"; the exception is an explicit ask, and it has been used rarely. A deploy is not finished until the gate has walked the live page: "a deployed fix is not a delivered fix."

## 8. Register of the code and the commits

- Comments explain why. Many carry the history of the constant they sit beside: what the first value was, what was measured, and what it broke.
- Commit subjects are imperative or declarative sentences. Bodies say why, name what was rejected, quote what was asked for, give the test count from the script, and say when a check was watched failing.
- Section headings in `CLAUDE.md` are findings stated as sentences: "A save never places you where you cannot stand."
- The register is plain and specific. Floodline's rules ask for "no hype."

## 9. Look and sound

- **Flat shapes.** Floodline is "dots and stick figures, flat colour, no sprites," so that the effort goes into simulation. Gear Master 2D draws geometric placeholders and adds figures later as TikZ sources compiled to SVG, from a fixed prompt.
- **A palette is a measurement.** `floodline/crates/gui/src/palette.rs` records two colors that were changed after players could not see them, with the pixel difference that was measured. Gear Master 2D's board was rebuilt for colorblind play.
- **No project has audio.** A search of all five repositories for audio APIs and audio files finds only macroquad's bundled loader. Music and sound are new work with no house pattern to copy.

## 10. Design tendencies

- Players are given the numbers, and the numbers come from the constants that decide them.
- Difficulty is measured against a fixed yardstick and written as a test. One repeated finding: "a rating predicts nothing about whether a fight is winnable."
- An opponent uses the same pieces a player has ("monsters wear the catalogue").
- For learned opponents, "a search baseline must be beaten before a neural net is justified" (`gear-master/design/rl-agent-plan.md`), training is never run in CI, and the bench crate is not shipped.
- State is a save file with `format` and `version` fields. A file the build cannot read is refused with a sentence. `localStorage` holds a convenience copy that is labeled as not the real save.

## 11. What transfers to the sword game

| From | Take | Use as |
|---|---|---|
| floodline `sim` | `fx.rs`, `rng.rs`, the `postcard` checksum, `boundary.rs`, `determinism.rs`, overflow checks in release | The start of the new `sim` |
| floodline `net` | `Peer`, `Loopback`, the wire format, the star lockstep, the desync stop | Two-seat lockstep |
| floodline `web/quad_rtc.js`, `web/vendor/` | Trystero room codes, the pasted-code path, pinned bundles, `echo.html` | The transport, moved into a plain ES module |
| gear-master-2d | `package-web.sh`, `deploy.yml`, `drive.py`, `count-tests.sh`, Blob download and file-input load, the document set | Packaging, the gate, replay and save files, the process |
| gear-master `console`, `agent` | One input type for players and pilots; search before any net | Story-mode opponents |
| the website packet | Authored copy as an exact JSON authority; the game entry point | `data/copy.en.json` |

**What is new, with no house precedent:**

1. **Real-time input.** Every existing project is turn-based or paced by commands. Floodline runs 20 ticks a second with a three-tick delay. A sword fight at 60 ticks a second is the first project where input latency is the game.
2. **Continuous physics.** The house has an integer billiard ball and integer flood water. It has no jointed bodies, no contact between moving solids, and nothing that splits.
3. **Audio.**
4. **Stylized violence.** Gear Master 2D's tone file keeps violence "Saturday-morning grade" and describes no damage to a body. A game about cuts needs its own rule.

## 12. Three observations worth acting on

1. **`gear-master-2d/CLAUDE.md` is 492 KB.** A file that is loaded into every session at that size uses a large share of the context the lecture says begins to degrade near 700k tokens. Floodline splits the same material differently: a 2.6 KB `CLAUDE.md` of rules and a 216 KB `DECISIONS.md` of history. The new repository should use Floodline's split.
2. **Suite time is a recurring cost.** `SECOND-ORDER-M22` row 17 measures fifty minutes to relink 94 test binaries on a busy machine. A physics suite that simulates many matches will meet this early; the optimized test profile and the habit of one full run per milestone should be in place from the first commit.
3. **Tunnelling has already happened once.** `SECOND-ORDER-M22` row 1 found that a billiard ball passed through rock because only the tile it landed on was tested. A fast blade is the same bug in a game where it decides who wins.
