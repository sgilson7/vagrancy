# Plan: the BT Lab, a tree editor, the Twitch arena, and the study

Sam asked (2026-10-06/07) for, in this order: the BT Lab for the behavior tree
class with a short deck to prompt it; a behavior tree editor so people can make
their own characters in the lab; a Twitch stream run by a subagent where viewers
earn experience for predicting winners, spend it to get a join code and fight
live, and submit their own trees; and a paper for Learning at Scale or
Educational Data Mining on learnersourcing through games and open platforms.
Alongside: art for each layer of the arcade tree, a wider scrollable chart, a
clearer chapter view, and a highlight on the deepest fight a player can take
next.

This file is the plan. Nothing in it is built until the step before it is
deployed and checked.

## 0. Constraints that shape everything

- **The game stays on GitHub Pages** (CLAUDE.md: no server of ours). The lab,
  the editor and the arena page are pages of the game, built and deployed with
  it.
- **The stream needs a process that is not a web page**: a chat bot that keeps
  experience, hands out join codes and takes submissions, and OBS to send the
  picture. That process runs on Sam's own machine, in its own repository
  (`vagrancy-arena`), never on a server of ours. It talks to Twitch's chat and
  APIs and to the arena page in a local browser.
- **The stream key is a secret.** It is kept in the macOS Keychain
  (`vagrancy-twitch-stream-key`), read by the setup script at the moment it
  writes OBS's local profile, and never written to a repository, a log or a
  page. Because it was pasted into a chat transcript, rotate it in the Twitch
  dashboard once the stream is set up, and store the new one the same way.
- **Data for a paper is human-subjects research.** Collecting viewers'
  predictions, fights and submitted trees for a study needs NC State IRB
  approval (likely exempt, category 2 or 3) before any data counted for the
  paper is kept. The stream can run before that; the study's logging switches
  on only with an approval number in the config, and the consent notice is
  pinned in the stream's chat and panels. Viewers' Twitch names are replaced by
  random ids in anything kept.

## 1. The BT Lab (built; deploying)

`web/bt-lab.html`, at `/vagrancy/bt-lab.html`: any opponent, watched or
fought, its tree drawn in full and lit each tick, and a panel from the running
action down to the keys (`pilot::moves`: scripts, the pose controller, the
search, the throw). A class tour of eleven steps follows the course's two
behavior tree lessons. Website entry `/projects/bt-lab/` (kind classroom).

Remaining: the deck (section 2), and the deploy of both.

## 2. The demo deck

Six to eight slides in Sam's voice, to run before and during the lab:
1. The question: how does "guard high" become shoulder and elbow keys?
2. A tree is tasks; selector and sequence, in the course's notation.
3. Stateful: a move runs over many ticks (the "still running" question).
4. From action to keys: a script, a controller, a search.
5. Parallel trees in one body.
6. The tour, and the discussion questions.
7. What comes next: build your own tree (section 3).

## 3. The behavior tree editor (in the lab)

- A tree is data already: `data/pilots.json`'s `kind: tree` with `rules`
  (`if` conditions, `do` a move, `interrupt`). The editor edits that shape and
  nothing else, so an edited tree runs on the same pilot code.
- **Core:** `content::custom` validates a tree from JSON (known conditions and
  moves, bounded rule count and parameters) and returns a `Spec` or the
  reason it is refused; the shim gets `Lab::watch_custom(tree_json, ...)`.
- **Page:** a rule list (add, remove, reorder by drag or arrows), a condition
  picker with numeric fields, a move picker showing each move's recipe from
  `lab_moves_json`, the drawn tree beside it, and "Fight it" and "Watch it
  against" buttons. Save to the browser (localStorage) and export or import as
  a file. A shareable code (the JSON, compressed and base64) is what a viewer
  pastes to submit.
- Tests: a tree that round-trips through the editor's JSON runs the same keys
  as the same tree in pilots.json; invalid trees are refused with a reason.

## 4. The Twitch arena (MVP)

Repository `vagrancy-arena` (public), on Sam's machine:

- **Arena page** (`web/arena.html` in the game): a fight shown full screen for
  the stream, with both trees as bubbles, names, and an overlay of the current
  prediction pool and the queue. It takes commands from the bot over a local
  WebSocket (the page is opened from `localhost` by the bot, so this is local
  traffic, not a server of ours on the web).
- **Bot** (Node or Python, local): connects to Twitch chat (an OAuth token for
  a bot account, kept in the Keychain), and runs the loop:
  1. Pick two fighters (road opponents, or submitted trees in the queue).
  2. Open predictions in chat (`!left` / `!right`) for a short window.
  3. Run the match on the arena page; on the result, award experience to the
     right predictions.
  4. `!xp` shows a viewer's balance; `!fight` spends experience for a slot:
     the bot whispers a join code, the viewer opens the game's online mode with
     it, and the arena page hosts that match (the game's existing WebRTC
     online play, `crates/net`, trystero over public Nostr relays).
  5. `!submit <code>` takes a tree code from the editor; the bot validates it
     with the same rules as the editor (via the arena page's core), queues it,
     and awards experience when it fights.
- **Experience ledger**: a local SQLite file; never uploaded.
- **OBS**: a scene with a browser source on the arena page, set up by a script
  through obs-websocket; the stream key comes from the Keychain into OBS's
  local profile at setup.
- **The subagent**: a Claude Code session given the bot's runbook
  (start, health checks, moderation rules, what to do when a match hangs),
  watching the bot's log and chat, and able to restart pieces. It does not
  hold the stream key; the setup script does.

## 5. The study (Learning at Scale / EDM)

Working title: learnersourcing behavior trees through a live game stream.

- RQs: Do viewers who predict outcomes come to predict better (a learning
  curve on predictions, tied to which tree features they saw)? What trees do
  viewers submit, and how do submitted trees differ from authored ones
  (size, composites used, moves chosen)? Does watching the lit trees change
  submissions?
- Data (after IRB): pseudonymous ids, predictions with timestamps, match
  results, submitted trees, editor events from the lab (opt in).
- Analysis: prediction accuracy by exposure; tree feature distributions;
  comparison with the course's homework trees (with consent).
- Venues: L@S full papers (deadline in winter), EDM (winter); a short paper or
  poster first.

## 6. The arcade mode changes

1. **Layer art**: one TikZ backdrop per row of the arcade tree, after
   `Sam_Comm_final.png` (flat fills, dark plum ink lines, mustard sky, coral
   and sage), each showing its region's name (the rice fields, the village
   road, the river crossing, the market towns, the hill country, the mountain
   pass, the high plateau, the far gate, the hilltop shrine, beneath the
   shrine), and each blending into the next at its edge. Colors go through
   `data/palette.json` (no red).
2. **The chart**: much wider and scrollable, like a large skill tree, with
   fights farther apart so fewer lines cross; the deepest fight a player can
   fight next and has not won is set apart with the arcade button's flair.
3. **The chapter view**: under each locked fight, its requirements, greyed;
   each met requirement struck through; once open, the requirements go and the
   fight takes a "ready" color; once won, the fight is struck through.

## 7. Order of work

1. Deploy the BT Lab and its website entry; the deck.
2. The arcade changes (section 6).
3. The tree editor (section 3), deployed.
4. The arena page and the bot (section 4), run locally; OBS set up; a test
   stream; then live.
5. The IRB application text and the paper outline (section 5).
