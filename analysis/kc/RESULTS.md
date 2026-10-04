# Vagrancy: knowledge-component graph, what each fight teaches, and the tutorial built from it

This applies the method in Gilson, *Building and Analysing a Knowledge-Component Graph for a Course Unit* (Game2Learn, 19 September 2026), to a game instead of a course. `build_graph.py` and `run_algorithms.py` ran from the kit unmodified. The definition is `data/kc_graph.json`, with the words in `data/copy.en.json` under `kc`, `kc_edge` and `kc_compare`, so the game and the analysis read one source. `vagrancy_graph.py` reshapes it into the template's tuples. Outputs: `out/` (the game's items) and `out-with-tutorial/` (the game's items plus the tutorial's missions). Run date: 4 October 2026.

Reproduce, from the kit's directory:

```
PYTHONPATH=<repo>/analysis/kc python3 build_graph.py vagrancy_graph <repo>/analysis/kc/out
PYTHONPATH=<repo>/analysis/kc python3 run_algorithms.py vagrancy_graph <repo>/analysis/kc/out
VAGRANCY_WITH_TUTORIAL=1 ... (the same two, into out-with-tutorial)
```

## 1. What was analysed

| Source | Treatment |
|---|---|
| The practice yard's 15 steps (`practice.step.*`) | One item each (`drill/<step>`), tagged from the step's text (prompt) and from what doing it takes (solution). |
| The road's 33 fights | One item each (`fight/<id>`), tagged from the opponent's `does` (prompt) and `try` (solution) together, and from what its pilot in `data/pilots.json` makes a player do. |
| How to play (`how.*`) and the opponents' introductions | Read as the game's statements of edges: they are the `stated_in` column, not items. |

**Size.** 36 components in 9 groups (31 domain, 3 conditions and integrations, 2 strategies); 43 edges (12 prerequisite, 22 relational, 5 integrative, 4 strategy); 48 items. 40 of 43 edges (93%) have an item behind them. That is far above the proposal's 55% calibration, and it says something about games rather than about tagging: a fight is a composite task, and composites make students cross edges. The grain is near the proposal's range (44 components for CSC 226).

## 2. The components, and how a fight makes each concrete

Each component is written in condition-response form (`kc.<id>.when`, `kc.<id>.then`) and typed on KLI's four dimensions plus the strategy flag (`out/nodes.csv`). The motor components are non-verbal, and so differ from anything in either course graph. In the game they are learned by doing and never said, so their `verbal` flag is false. The reading-an-opponent components are verbal with a rationale. The game states each as a sentence, in an opponent's `try`.

The proposal's *concretization* is the transcript's idea, formalised as instances: an abstract component is learned through items that require it. In the game the item is a fight, and the pilot is what makes the component concrete. The table lists, for each fight, what it requires and the concrete situation that requires it.

| Fight | Components | How the fight concretizes them |
|---|---|---|
| scarecrow | K01 K02 K04 K13 K14 I1 | A target that never moves: the cut, aim at the neck, and closing distance, with nothing else in the way. |
| thresher | K04 K13 K19 S1 I1 I2 | The same overhead and the same pause, again and again: the opening after a committed move, made visible by repetition (count the pause). |
| courier | K04 K29 K08 K09 K19 I2 | She keeps away and rolls through your guard: distance, and a roll seen from the other side. |
| drover | K05 K23 K12 K13 I2 | A low sweep and no guard: high against low, answered with a jump or a high cut. |
| sampler | K19 K26 S1 I2 | Your last round, mirrored: your own committed moves become the opponent's. |
| salt_trader | K08 K11 K25 K19 I2 | She dodges through your point and cuts: draw the dodge, then strike in the wait after it. |
| lamplighter | K23 K25 K19 K12 I2 | A high guard and a thrust: draw the thrust, then go under the high blade. |
| ferryman | K02 K18 K19 I2 | A blade a third longer and heavier: reach, and letting the slower blade go first. |
| cooper | K20 K19 K07 K09 I2 | Jump strikes, and getting up after a bad landing: a path already chosen, seen from the ground. |
| windmill | K12 K22 K16 I2 | A blade that never stops turning, building speed the way you can: stop it with a block. |
| ropewalker | K12 K30 K20 K06 I2 | She falls on you from the air: a guard above. |
| smith | K20 K17 K06 K19 K07 I2 | Plants, vaults and air bounces: every way of leaving the ground, used against you. |
| bellringer | K22 K12 K08 K19 I2 | A turning blade that dodges away from your point. |
| gatekeeper | K12 K21 K02 I2 | A sword held out and never swung: getting around a guard. |
| reader | K27 K28 K19 I2 | A search that looks 300 ms ahead and reacts 200 ms late: the late change, made concrete by a number the introduction states. |
| dyer | K23 K05 K24 K08 I2 | Low sweeps, and dodges at the body: high against low again, with a dodge behind it. |
| potter | K19 K15 K32 K14 I2 | Thrusts, with twice the ink: the condition makes the ink plan fail and the fatal cut necessary. |
| carpenter | K17 K20 I2 | A vault now and then. |
| mason | K15 K29 I2 | Backs off when his ink runs low: ink as a clock you can read on the opponent. |
| weaver | K25 K08 K22 I2 | Dodges in through your attack: drawing a dodge that comes toward you. |
| falconer | K30 K20 K06 I2 | Air bounces and cuts from above, between searches. |
| boatwright | K19 K29 I2 | Thrust and step back: the opening is a moving one. |
| brewer | K23 K05 I2 | Punishes a jump with an overhead: high against low, from the other side. |
| herbalist | K24 K14 K15 I2 | Dodges at her head: aim where she does not defend. |
| cartographer | K27 K28 I2 | The reader's search, looking further ahead. |
| magistrate | K24 K29 K08 I2 | Dodges at head and body: the dodge carries her away, so distance comes back into it. |
| abbot | K23 K24 I2 | The brewer's overhead and the herbalist's dodge, together. |
| warden | K20 K17 K24 I2 | Dodges at her head and vaults on her sword. |
| hermit | K29 K31 K06 K24 I2 | Waits for you, under half gravity: every timing you learned in the air changes. |
| tanner | K28 K12 I2 | The fastest reaction on the road: a steady guard beats a feint. |
| watchman | K23 K24 K28 I2 | Punishes a jump, dodges at the head, reacts fast. |
| archivist | K24 K14 K28 I2 | A search that dodges most attacks at its head. |
| vaulter | K20 K17 K30 I2 | Plants and vaults, always from above: a path already chosen, as the whole plan. |

## 3. How the components are related

The 43 edges are in `out/edges.csv`, each labelled with a sentence a player should be able to say (`kc_edge.*`), and with the KLI learning process and where the game states it. The structure, briefly:

- **The arm is the root of almost everything.** K01 (the shoulder) dominates six components on both graphs: the carry-on, blocking, swinging for speed, and through blocking the guard-related three.
- **Most reading-an-opponent components are relational, not prerequisite.** They are the opening after a committed move (K19) "with one condition changed". A drawn move makes the opening (K25). A jump or a plant is a committed move that lasts longer (K20). Your own last round is the committed move you face (K26). Reaction time is how long an opening lasts (K28). This is the proposal's claim that relational edges matter most, made concrete: K19 is the hub of the reading half of the game.
- **The conditions are relational edges from the base mechanics.** Light gravity is the same jump with the fall slowed (K06→K31). Deep ink is the ink plan made twice as slow (K15→K32), which makes the cut that ends the round (K14→K32) the answer.

## 4. Findings the method produced

**4.1 Edges the game states and never makes a player cross.** Three of 43:
- *A plant is a swing whose push goes into the ground* (K16→K17). The yard teaches swinging and planting three steps apart, and no fight asks for both. The physics says the relation is literal: a full swing strikes the ground, so a swing for speed is a series of small plants. Measured in the yard: a plant from standing lifts the fighter about 283 cm, and a plant after two seconds of swinging about 502 cm.
- *Comparing two opponents* (S2→K24, S2→K32). The road never asks a player to compare. Its requirements are about winning, not about two fights side by side.

**4.2 Components with one practiced route.** Two acquire a dominator once only item-backed edges count, and neither had one on the stated graph:
- *Planting the sword* (K17) is reachable in practice only through the elbow (K02), because its relational link from swinging is the unpracticed edge above.
- *An opponent with more ink* (K32) is reachable only through *what a cut is* (K13). The potter's double ink is fought, but the comparison that makes the condition visible, the same thrust with and without it, is not.

**4.3 Components that cannot be rebuilt.** *Getting up* (K07) has a recoverability of zero on both graphs: no relational edge reaches it. It is pressed rather than understood, and if it is forgotten nothing in the game leads back to it. The least recoverable after it are *a guard above*, *getting around a guard*, *the sword carries on*, *reach* and *distance*, each with one edge in. Rank stability across damping ran from 0.926 to 0.996.

**4.4 The direction a player travels.** Seeded at what a player keeps from the yard and the first fights (shoulder, stepping, the cut, the block), the hardest components to reach are *getting up*, *light gravity*, *dodging in the air*, *rolling*, and *a late change*. The air components (K06, K10, K31) are reached late, while the road puts jumps against opponents from row 2.

**4.5 Groups against clusters.** The groups the agent wrote (Arm, Move, Defend, Cut, Physics, Read, Condition) score 0.052 modularity against 0.559 for the consensus partition (ARI 0.097, NMI 0.427). As in CSC 226, the authored partition is close to no partition. The data-driven clusters are what the tutorial's chapters follow:
- **In the air:** jumping, the bounce, the air dodge, high against low, light gravity.
- **The arm and its physics:** shoulder, elbow, carry-on, cut, swing, plant, reach, a path already chosen.
- **Reading an opponent:** block, the opening, guard, turning blade, drawing, mirror, guard above, and the match.
- **Dodging and aiming:** dodge, roll, wait, fatal parts, ink, aim, deep ink, comparison.
- **Thinking ahead:** a late change and reaction.
- **Getting up** stands alone (4.3).

**4.6 Pairs to teach together.** 80 cross-group pairs are stable at 90% or better. As the CSC 584 write-up warns, that count should not be reported as it stands. The usable list is the 13 comparison pairs in `out/comparison_pairs.csv`. Eleven are MISSING in the game, meaning no requirement or row puts the two fights side by side. Examples: the drover and the dyer (the same low sweep, with and without a dodge behind it), the reader and the cartographer (the same look-ahead, longer), the lamplighter and the salt trader (drawing a thrust and drawing a dodge).

## 5. The tutorial built from it

`data/tutorial.json`, played from **Learn in missions** on the main menu, is a map of 42 missions in the order the graph teaches:

- **Each mission** teaches the components in `teaches` and shows the edges in `builds_on`. Their sentences are the transcript's "show the edge back to a known component". Each mission's tasks then make it concrete: a drill in the yard, with a goal core checks from the world, or a fight on the road to a win.
- **Order.** `every_component_is_taught_and_after_each_one_it_builds_on` checks that every component is taught, and that each mission comes after a mission teaching each component it builds on (prerequisite, relational and integrative edges). Strategy edges are left out of the ordering because the strategy is learned in the comparisons that follow.
- **Concretization.** `each_task_makes_concrete_what_its_mission_teaches` checks that a mission's drill or fight is an item in the graph that requires what the mission teaches.
- **Comparisons.** Ten missions are comparisons (4.6), after Gick and Holyoak by way of the proposal: two fights that differ in one thing, played one after the other, with a sentence saying what they share (`kc_compare.*`).
- **Closing the gaps.** One mission, *Swing, then plant*, is an item the game did not have: a plant that lifts the fighter 400 cm. A plain plant gets about 283, so meeting it takes a swing first. With the tutorial counted as items (`out-with-tutorial/`), all 43 edges are practiced, no component is left with a single practiced route, and nothing loses recoverability. `the_tutorial_practices_every_edge_the_game_leaves_unpracticed` keeps it that way.
- **Goals are measured.** Every drill's goal has a plan of keys that meets it, and doing nothing meets none of them (`every_goal_in_the_yard_can_be_met_with_the_keys`). The plant refuses the jump key. The first goal was "the tip above the head", and standing still met it, because the arm falls and swings like a pendulum. It is now one whole turn of the shoulder, which a pendulum never adds up to.

## 6. Findings that come from simpler checks

Kept separate, per proposal §7.2.
- The ink step in the yard cannot be done there: the post has no ink (`data/body.json`), and self-cuts were removed. The tutorial's ink mission is played on the scarecrow.
- *How to play*'s cut section said your own blade cuts you until 4 October (SECOND-ORDER-M5 row 31). A read of the text found it, not the graph.
- The road's requirements are about winning margins and times, never about technique. That is a design fact, readable from `data/road.json`.

## 7. What these results do not establish

- **One coder.** The agent did the tagging alone, and the proposal names that as the largest weakness. A second coder should tag at least 25 items and the relational edges, and report Cohen's κ. One added edge can remove a dominator, so 4.2 is the most exposed.
- **Tags from text and pilots, not from play.** A player can beat the brewer without ever jumping. The tag says the fight is about high against low, not that every win uses it. Replays from Sam's play would test the tagging the way chat logs would test a course's.
- **Groups were written before the analysis.** The low modularity in 4.5 partly measures the agent's first guess.

## 8. Hard calls, for the second coder

- *Getting up* (K07): a component, or a fact? It is typed CC non-verbal. It may belong inside *stepping* as a condition, which would also remove finding 4.3.
- *Swinging for speed* (K16) and *planting* (K17) may be one component with two conditions, given the measurement in 4.1. They are kept apart because the yard teaches them apart, and the gap between them is the finding.
- The integrative *winning a match* (I2) is tagged on every fight from the thresher on. That gives it the most instances of anything and pulls the reading cluster together in 4.5.
- *High against low* (K23) joins three situations: a low sweep, a high guard, and an overhead that waits for a jump. The fights that use it (drover, lamplighter, dyer, brewer, abbot, watchman) differ enough in difficulty that the proposal's grain rule would split it.
