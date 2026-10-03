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

## The build

- **The red band is hue 330°–20° at saturation 0.20 or more.** It lives in `crates/content/tests/palette.rs`, beside its check, and not in `data/palette.json`, where editing the band would be a way to pass. Ochre `#B8862B` sits at hue 39°, 19° clear.
- **`index.html` holds `{{key}}` tokens and no words.** `package-web.sh` fills them from the copy file. The title, the fallback and the loading line must be on the page before any script runs, and a hand-copied string would be a second copy.
- **CSS colors are variables.** `package-web.sh` writes their values from `data/palette.json`, so the palette file stays the only place a color is written. `palette.rs` fails on a hex or `rgb(` in `web/`.
- **The copy file ships inside the wasm module** (`content::copy::COPY_JSON`), so the strings and the build that uses them cannot drift apart.
- **`build.txt` holds the content hash and the commit.** Against the live page, `drive.py` reads it back. The content hash differs between a laptop build and a CI build (different linkers), so the commit is what ties a live page to a push.
