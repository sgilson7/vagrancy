# Second-order notebook, M0

`B3 Build in self-questioning` · `C3 Interrogate the claim`

| # | row | kind | status |
|---|---|---|---|
| 1 | **The packet sat in `vagrancy-packet/` inside an empty folder, and the GitHub repository did not exist.** Moved to the root per Q11. `reference/` holds clones at the HOUSE-STYLE commits and is gitignored. | finding | done |
| 2 | **Floodline's boundary test never read `sim`'s source.** The new `sim_has_no_float_no_hashmap_and_no_clock` strips comments and strings before it scans, and has its own test (`the_source_scan_sees_what_it_claims_to`), because a source-text lint is the shortcut E.2 warns about. It was broken with `pub const SPEED: f32 = 1.5;` and failed at `boundary.rs:67`. | finding | done |
| 3 | **The packaging fill met its own documentation.** The comment in `index.html` contained a literal `{{key}}`, and the fill tried to look it up (`KeyError: 'key'`). Packaging now strips HTML comments before filling. The shipped page carries no comments. | finding | done |
| 4 | **In a repository with no commits, `git rev-parse HEAD` prints `HEAD`.** `build.txt` read "a25cf501 HEAD". Changed to `rev-parse --verify -q HEAD`. | finding | done |
| 5 | **Every lint was broken once and watched failing**, each in a copy, restored from the copy. The cases: a third dependency in `sim`; an `f32`; an exclamation mark in a label; "Mugen" in `web/`; the name in `styles.css`; text in `index.html`; `{foo}`; ochre turned `#C0392B`; `#ff0000` in CSS; an unlicensed `.wav`. In the gate: a visible "Welcome, warrior", a `console.error`, and an `<img>` from example.com. Each gave its red line. | finding | done |
| 6 | **M0 recon: the empty suite runs in 0.16 s warm.** The guess was under 30 s (`time cargo test --workspace -q`). It is the baseline before physics arrives. | finding | done |
| 7 | **`local.keyboard_limit` breaks TONE rule 3 and is carried on an allow-list**, not repaired (Q10). | worklist | the human's |
| 8 | **Sam has not seen deploy gate 1 before M1 starts.** He asked the agent not to stop. Carried. | worklist | the human's |
