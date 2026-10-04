# Decisions

Why things are the way they are, one paragraph each. The full argument for
D1 to D18 is in `PLAN.md` §2; this file holds the decision, what it rests on,
and anything decided since. Newest entries go at the bottom of their section.

## Decided by Sam

- **The name** is Vagrancy, held once in `data/copy.en.json` as `game.name` (brief 0.7).
- **A fighter's own blade cuts that fighter** (brief 0.7; D11).
- **The legs take slow steps** on two keys; everything fast comes from the sword (brief 0.7; D6, D8).
- **Free energy:** an arm motor pushes the arm and sword with no equal push back on the torso (brief 0.7; D9).
- **2026-10-03, on approving PLAN.md**, Sam took the agent's recommendation on every question in PLAN.md §8:
  - **Q1.** A waist cut drains ink fast and does not end the round at once.
  - **Q2.** The text says ink.
  - **Q3.** The entry page shows `game.content_note`.
  - **Q4.** Public relays introduce online players, and pasted codes are the alternative.
  - **Q5.** No About link until `/projects/vagrancy/` exists.
  - **Q6.** `TONE.md` is public.
  - **Q7.** The music slot stands until a license exists.
  - **Q8.** Three rounds win a match, and the road has eight stops.
  - **Q9.** The arms-only rule reads "only arm bits drive a joint motor".
  - **Q10.** `local.keyboard_limit` stays as written, on the lint's allow-list, for Sam to rule on.
  - **Q11.** The repository root is `~/Documents/vagrancy`.
  - **Q12.** Lower-case identifiers may spell the name.
  - **Q13.** The deny-list is the draft in `crates/content/tests/copy.rs`.
  - **Q14.** An opponent's blade cuts a hand.
  - **Q15.** A sword nobody holds does not cut.
  - **Q16.** The keys drive only the joints still attached.
  - **Q17.** Speed is bounded by the cap and the drag.
  - **Q18.** The gate stays strict, and online checks are a separate mode.

  He also gave the agent push rights for this run, and asked that every check needing him be carried as a row, not waited on.

- **2026-10-03, during M1:** "make the legs more rigid so its easier to move ala nidhogg, the arms should be full fluid though." This revises D8. The legs are a rigid frame, and the step keys drive the fighter through the ground at `sim::balance::RUN_SPEED`. The arms and the sword stay fully simulated.
- **2026-10-03, after the MVP:** "you gotta find a way to get the song vagrancy to play, even if its in a weird way like a youtube player playing the song with only auido." This overrides brief 0.5's "does not embed a stream" for YouTube. The agent built a "Music from YouTube" section in Settings. The player pastes any YouTube link, and the video loops in a small, visible 200 by 200 player in the corner. It is not hidden: YouTube's terms do not allow a player that is hidden or shown only as audio. The game ships no audio. On 2026-10-04 Sam chose the video: "the song link should be pre-loaded in and you just have to hit play". His link, https://www.youtube.com/watch?v=L7dqdw2i5JM, is `DEFAULT_LINK` in `web/youtube.js`. It is filled in already, and a player may paste another. The page's own text still names neither the track nor the show; the YouTube player shows the video's title. Nothing contacts YouTube until the player presses Play, so the gate's "no request leaves the origin" still holds for everything else. The gate checks the Play path in its own browser context, with YouTube stubbed. The local-file slot is unchanged.
- **2026-10-04:** "remove all the ai-sms and captions that claude is known to leave to itself", with `game.content_note` as the example. This overrides Q3: the entry page shows no content note. The agent removed the lines that explain the game to the reader rather than tell them what to do: `game.content_note`, `game.description`, `road.intro`, `practice.intro`, `practice.done`, `online.intro`, `online.music`, `settings.music.privacy` and `settings.youtube.privacy`. It also trimmed the asides from 22 more strings (SECOND-ORDER-M5 row 31). Sam's request outranks the "report, do not repair" rule for these strings only.
- **2026-10-04: the road is a tree.** Sam asked for "a map of battles kind of like the errands page from gear master 2d, and then make a tree of fights that branch off of each other and have various requirements ... laid out in a downwards facing tree ... think a hasse diagram for a poset where each level is the number of requirements". He also asked for "10 more fights that are as difficult as the reader, and 5 that are as difficult as the archivist", for the Soul Calibur II Weapon Master map as inspiration for requirements and rewards, and for the agent to make every decision itself. What the agent decided:
  - **The structure.** `data/road.json` lists each fight with its requirements, and the row a fight sits in is how many requirements it has. A requirement may only name a fight with fewer requirements, so the tree runs downward and nothing can depend on itself. The scarecrow alone is open from the start. There are 33 fights in eight rows, 0 to 7 requirements.
  - **Three kinds of requirement:** beat a fight, beat it without losing a round, or beat it within a number of seconds. The save keeps each fight's best result (fewest rounds lost and shortest match, each on its own), so it is now save version 2. Version 1 saves keep their wins, but the margin is unknown, so those wins count as wins and nothing more.
  - **From Soul Calibur II:** some fights carry a condition (the opponent has twice the ink; half gravity; a third, one round decides, was dropped when the ladder showed the opponent's opening move decided it), shown on the fight's card. The reward is the map itself: a win names the fights it opened, and the first of them is the next button. Nothing else is awarded. Rewards beyond opening fights would need something to spend them on, and the game has nothing.
  - **The look,** after gear-master-2d's errands map: a row of buttons per level, ordered by where their requirements sit, with an SVG of curves behind them. Hovering a fight or a line lights its lines, and a card says what is asked and what is done. Clicking a fight shows its card above the tree, with Fight only if it is open.
  - **Difficulty is measured, not asserted.** `make ladder` plays every fight with its condition. `the_tree_gets_no_easier_going_down` checks every requirement against the ladder, within the noise of 200 matches, and checks that each row averages no easier than the row above. The 15 new fights are checked to be within that noise of the reader or the archivist.
- **2026-10-04: a tutorial built from a knowledge-component analysis.** Sam: "analyze what the game is trying to teach you with each fight using the kli kg kc proposal work ... what knowledge components each fight is trying to teach you, how each component is related, and how the fight concretizes the abstract kc. use this analysis ... to create a dedicated tutorial mode that is a map of missions following the teaching flow". The agent applied his proposal (GameAI-Fall26/KGCurriculum/.../kc_graph_proposal.pdf) with his kit, run unmodified, and wrote it up in `analysis/kc/RESULTS.md`. The decisions it took:
  - **One definition for the analysis and the game.** The graph's structure is in `data/kc_graph.json` and its words are copy strings, so the game shows the same condition-response sentences and edge labels the analysis uses. `analysis/kc/vagrancy_graph.py` only reshapes them for the kit.
  - **Items are the yard's drills and the road's fights.** Concretization is the transcript's idea in the proposal's form: an abstract component is learned through the items that require it. In a game the item is a fight, and its pilot is what makes the component concrete.
  - **The tutorial is a map of missions** (`data/tutorial.json`, "Learn in missions"). Each mission teaches components, shows the edges back to what the player has, and makes them concrete in a drill or a fight. Comparison missions play two fights that differ in one thing. Its chapters follow the clusters the analysis found, not the groups the agent first wrote. Its order is checked against the graph.
  - **Goals are read from the world by core** (`content::tutorial::Tracker`), and each is shown to be reachable with keys and not met by standing still. Where the first idea failed a measurement, the goal changed: the tip above the head became one whole turn of the shoulder, and a rise became a plant.
  - **Save version 3** keeps the finished tasks. A version 2 save loads with none done.
- **2026-10-04: weapons, and the road drawn three ways.** Sam: "play around with a few more designs, that potentially look even more similar to weapon master in soul caliber, and also add the ability to change your weapon like weapon master, and you unlock new ones as you go ... long swords, multi pronged swords, curvy swords. the intent is the normal sword is the most powerful / balanced sword"; then "the various weapons should also work online too"; then "if the longsword is too strong then dont make it accessible to players, only for enemies to have". What the agent decided:
  - **A weapon is the sword reshaped in data** (`data/weapons.json`): length, mass and hilt as percentages, and a shape. A curve bends through points set off the blade; prongs branch from a fork. The sim gained extra rigid points and a list of cutting edges per sword (SIM_VERSION 7). Cuts and clashes run over every edge, and a plain sword is one edge. A hash of every particle position over scripted matches was the same before and after the change (`1231bb3cb0ef2229`), so the sword plays exactly as it did.
  - **Six weapons a player can carry:** sword, short sword, scimitar (one curve), kris (three bends), forked blade (two prongs) and trident (three). Each is won at a fight on the tree, Weapon Master's reward. The fight's card names the reward, and the map marks it with a diamond. The longsword measured stronger than the sword at every length and weight tried (76 to 91 % against 70 %), so it is the enemies' alone, as Sam said. The ferryman carries it, and his long blade is now that weapon rather than a field of his pilot.
  - **"The sword is the strongest" is measured,** like the ladder: `make weapons` plays the yardstick carrying each weapon against eight opponents, and `the_sword_is_the_strongest_weapon` reads it. Heavier did not mean weaker: a heavier scimitar hit harder and won more. Length was what decided.
  - **Weapons online:** the joiner's hello carries its weapon (wire PROTO 4). The host rebuilds the match with both weapons before its welcome sends it, so both sides step one world. At one keyboard, each seat picks from what the save has unlocked. The tutorial stays with the sword, since its goals were measured with it.
  - **Save version 4** keeps the weapon carried. One the player has not unlocked, an enemy's, or an unknown one becomes the sword rather than refusing the file.
  - **The road's designs:** the tree; a chart after Weapon Master's map (each row a named region, fights as seals scattered across it, dashed routes); and chapters after its chapter select (a list of chapters, and each chapter's fights as cards that list what they ask for). A switch above the map picks one and is remembered in the browser. The tree stays the default until Sam picks.
- **2026-10-04: two weapon carriers per chapter, opened by weapon challenges.** Sam: "more of the enemies should have varying weapons early on, add 2 enemies per chapter that use different weapons, that are unlockable with weapon challenges like beat the courier with the trident". The agent added sixteen opponents, two to each row of the tree, each carrying a weapon from `data/weapons.json`. Three carry the enemies' longsword. Decisions:
  - **A fourth kind of requirement,** "beat X carrying W" (`with`). The save keeps the weapons each fight was won with (format 5). A format 4 file reads as it is, with none named, so old wins count toward no challenge.
  - **A weapon is won by beating the first opponent who carries it**, which puts every weapon in reach early: the short sword and scimitar in chapter 1 (the tinker and the pilgrim, open from the start), the kris and forked blade in chapter 2, and the trident in chapter 3. A test checks that a challenge's weapon is won in a row above the fight that asks for it, and never asks for the enemies' longsword.
  - **Chapter 1 has no challenges.** Its fights are open from the start, which is what the first row means. Its two new opponents are where the first weapons are won.
  - **Each new fight was measured on the ladder's own seeds and tuned** until every requirement and every row's average held, as for the rest of the tree.

## The build

- **The red band is hue 330°–20° at saturation 0.20 or more.** It lives in `crates/content/tests/palette.rs`, beside its check, and not in `data/palette.json`, where editing the band would be a way to pass. Ochre `#B8862B` sits at hue 39°, 19° clear.
- **`index.html` holds `{{key}}` tokens and no words.** `package-web.sh` fills them from the copy file. The title, the fallback and the loading line must be on the page before any script runs, and a hand-copied string would be a second copy.
- **CSS colors are variables.** `package-web.sh` writes their values from `data/palette.json`, so the palette file stays the only place a color is written. `palette.rs` fails on a hex or `rgb(` in `web/`.
- **The copy file ships inside the wasm module** (`content::copy::COPY_JSON`), so the strings and the build that uses them cannot drift apart.
- **`build.txt` holds the content hash and the commit.** Against the live page, `drive.py` reads it back. The content hash differs between a laptop build and a CI build (different linkers), so the commit is what ties a live page to a push.
- **Fixed-point narrowing panics** (`fx::narrow`). An `as` cast wraps, and overflow checks do not cover casts.
- **Friction is applied in the ground projection, every relaxation pass.** Applied once per tick, it let the feet slide apart under the passes.
- **The balance rule and every constraint are exact pair shifts.** `a` moves by `k·m_b` and `b` by `−k·m_a` with one integer `k`, so only the arm motor, the ground, the walls, the cap and drag change momentum. This is what makes H5 meaningful.
- **Bit 6 of the input byte is "ready for the next round".** The round restarts through `World::step` like everything else.
- **Points in `data/body.json` are numbered in name order**, so the numbering does not depend on how the JSON object was written.
- **A replay carries its `Setup`, physics included.** A constant that lives in `Setup` can change without invalidating old replays. A constant that lives in code cannot, and changing one means bumping `SIM_VERSION`.
