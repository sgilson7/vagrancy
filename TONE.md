# TONE.md

Every string a player reads is checked against this file: menus, the practice yard, opponent introductions, results, the lobby, errors, settings, the page's own heading and status lines, and every accessible name.

The strings themselves are in `data/copy.en.json`. They were written before the build. The agent implements them exactly and does not rewrite them (`PLANNING-BRIEF.md` 0.6). This file is for the two cases where a string has to be judged: a new one the build needs, and an old one somebody wants to change.

## Where this comes from

1. **Sam's editorial and voice guide.** It is private and is not in this repository, and neither is the transcript evidence behind it. What is here are the rules derived from it, restated for a game.
2. **`gear-master-2d/TONE.md`**, for the form: each rule ends in a check that can be answered yes or no about one sentence. If a reviewer cannot point at a clause and rule on it, it is not a rule.
3. **The lecture's Procedure B**, for one rule: same label, same words, every time it appears.

## The voice, in one line

**A teacher who is patient and concrete, explaining a sword fight.**

The game is about cuts, and the voice describes them the way a careful instructor describes a worked problem: what happened, why, and what to try next. It does not sell the game, raise its voice, or make a joke of the player. The fight is where the surprises are. The words stay level.

---

## The rules

### 1. The first sentence says what this is or what to do

A screen opens with its purpose. A step opens with its action.

> Practice: "Swing at a post that does not fight back."

*Check:* cover everything after the first sentence. Does the player still know what the screen is for?

### 2. The reason comes with the instruction

Say why before, or just after, asking the player to do something. Use "because" and "so" when they state a real connection.

> "The sword cannot go into the ground, so the push lifts your fighter instead."

*Check:* does the instruction have a reason within the same string? A bare command with no reason needs one, unless it is a button label.

### 3. "You" for what the player does; names for everyone else

Address the player directly when describing an action they can take. Name fighters and opponents explicitly. In versus, where "you" is two people, use the fighters' names.

*Check:* in a versus string, search for "you". It should not be there.

### 4. Concrete verbs, and no praise words

Swing, plant, push, cut, drop, spill, step, block, load, download. The words that are out: epic, brutal, ultimate, deadly, awesome, insane, intense, legendary, unleash, destroy, and any word whose job is to tell the player how to feel.

*Check:* `tests/copy.rs` holds the list. A hit is a failure.

### 5. A term is introduced in plain words, named once, and then never renamed

"Ink" is explained the first time it appears and is called ink everywhere after. The same goes for round, match, replay, save file, room code, input delay. A control's name in the instructions is the same as its name in Settings.

*Check:* for each term in the glossary block of `data/copy.en.json`, search for its listed synonyms. A synonym is a failure.

### 6. A label says what the control does

"Download replay", not "Replay". "Host with a room code", not "Host". A label is a verb and its object.

*Check:* read the label with no screen around it. Is it clear what pressing it does?

### 7. A refusal names what is in the way and what to do next

One or two sentences. No apology, no code, no "something went wrong".

> "No relay answered, so your two browsers could not be introduced. Try hosting with a pasted code, which does not need a relay."

*Check:* does the message name the thing that failed, and does it offer a next step where one exists?

### 8. No invented universals

"Always", "never", "every", "all", "only", "impossible", "unbeatable", "guarantees", "instantly", "nothing is stored". Each of these is a claim about the whole game. It may appear only where a test establishes it, and the string's entry in `_universals` names that test.

> Allowed: "Any cut on the neck ends the round." Established by `a_neck_cut_ends_the_round_on_the_tick_it_lands`.

*Check:* `tests/copy.rs` searches for the list. A hit with no named test is a failure.

### 9. A limit sits beside the claim it qualifies

Say what a feature does not do in the same string that says what it does.

> "The file is read by your browser and is not uploaded. In an online match your friend does not hear it."

*Check:* does the string make a promise about privacy, storage, saving or the network? Then its limit is in the same string, not on another screen.

### 10. A number a player reads comes from core

Seconds, rounds, percentages, counts and key names are placeholders filled from the constant or the data that decides them. Nobody types one into a sentence. This also covers a spelled-out count: "three ways" is a number. "One" and "two" are left alone, because the game is for two players at one keyboard and those words are also ordinary English.

*Check:* `tests/copy.rs` fails on a digit, or on a number word from three up, in a string. The allow-list is short: the file format MP3.

### 11. Advice is offered as one thing to try

An opponent's introduction says what that opponent does and suggests one approach. It does not promise that the approach wins. What an opponent "usually" does must be what its pilot data makes it do.

> "The suggestion is a starting point, and there are other answers."

*Check:* does the advice contain "will win", "the way to beat", or "the trick is"? Rewrite it as something to try.

### 12. No slogans, and no fragments for effect

A result screen says what happened and why, in a whole sentence. No single-word lines. No exclamation marks.

> "You lost the round. The gatekeeper's blade cut across your neck."

*Check:* is there a sentence with no verb? Is there an exclamation mark? Either is a failure.

### 13. A cut is described by its rule, not by its sensation

Name the part and the consequence. Nothing describes pain, flesh or suffering, and nothing is vulgar. The fighters are paper figures and what they spill is ink; the text says so once and then uses "ink".

*Check:* would this line need a content warning if it were read aloud to a classroom? Then it is out. `tests/copy.rs` also holds a short list of gore words.

### 14. No color word for ink or a cut is a red

The palette lint guards the pixels. This rule guards the words: red, crimson, scarlet, blood-red and their relatives do not appear.

*Check:* `tests/copy.rs`.

### 15. Nothing from the show

The mood is borrowed and the material is not. No name, title, line or reference from it appears in any string.

*Check:* `tests/copy.rs` holds the deny-list.

### 16. The mechanics of the page

American spelling. Sentence case for labels and headings. No speech filler ("basically", "essentially", "just", "simply"). An ellipsis only in a loading line.

*Check:* `tests/copy.rs` for the filler list; a reviewer for the rest.

### 17. A rule here beats a good sentence, and a good sentence beats a habit

If a line breaks a rule and is clearly right anyway, change the rule in this file in the same commit, with the sentence as the reason. Leaving both and hoping is what is not allowed.

---

## Working notes

**Two registers, kept apart.** A sentence is written by a person and lives in the copy file. A number is derived by core and fills a placeholder. A sentence that states a mechanic is listed in `_depends` with the decision it rests on, so that a change to the decision surfaces the sentence.

**Places.** Each opponent's place line is one plain sentence about where the fight happens. The earlier lines each carried an anachronism (a radio at a ferry landing, a turntable in a tea house); Sam called them weird, and the 2026-10-05 rewrite in his voice removed them.

**Where the strings live.** `data/copy.en.json`. If you are editing a `.rs`, `.js` or `.html` file to change what a player reads, you are in the wrong file.
