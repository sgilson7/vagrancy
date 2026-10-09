#!/usr/bin/env python3
"""A window for Sam to play a match in, with every finished match kept as a
take (Sam, 2026-10-08: "set me up a little window that you are recording,
and I will play as paul to get good footage").

It opens the local build (make serve, or dist/web on port 8766) in a real
browser window, opens the match named on the command line (data/duels.json),
and watches: each time a match ends, its replay file is written to
~/Movies/Vagrancy/takes/<match>-<n>.replay, with the result beside it. Play
as many as you like (Enter plays again); close the window to stop.
make_short.py renders a take with `"replay"` in its cut list.

    .venv-test/bin/python analysis/video/record_take.py arrakeen
"""
import json, sys, time
from pathlib import Path

from playwright.sync_api import sync_playwright

PAGE = "http://127.0.0.1:8766/"
OUT = Path.home() / "Movies" / "Vagrancy" / "takes"


def main(match):
    OUT.mkdir(parents=True, exist_ok=True)
    n = len(list(OUT.glob(f"{match}-*.replay")))
    with sync_playwright() as p:
        b = p.chromium.launch(headless=False, args=["--window-size=1300,1050"])
        page = b.new_page(viewport={"width": 1280, "height": 950})
        page.goto(PAGE)
        page.wait_for_function("document.body.dataset.ready === '1'", timeout=60000)
        # This window's own save: the first chapter done, so the match is open.
        save = page.evaluate("window.vagrancy.save()")
        save["state"]["story"] = max(1, save["state"].get("story", 0))
        page.evaluate("s => localStorage.setItem('vagrancy.autosave', JSON.stringify(s))", save)
        page.reload()
        page.wait_for_function("document.body.dataset.ready === '1'", timeout=60000)
        page.click('[data-copy="menu.story.label"]')
        page.click(f'[data-copy="duels.{match}.start"]')
        print(f"Playing {match}. Each finished match is saved to {OUT}. Close the window to stop.", flush=True)
        # One take a match, kept as it ends: the replay keeps growing while
        # the fighters run on after it, and keeping each new length kept the
        # same match hundreds of times.
        over = False
        while True:
            try:
                phase = page.evaluate("window.vagrancy.phase()")
                if phase != "match_over":
                    over = False
                elif not over:
                    over = True
                    data = page.evaluate("window.vagrancy.replay()")
                    if data:
                        n += 1
                        wins = page.evaluate("window.vagrancy.wins()")
                        path = OUT / f"{match}-{n}.replay"
                        path.write_bytes(bytes(data))
                        path.with_suffix(".json").write_text(json.dumps({"match": match, "wins": wins, "saved": time.strftime("%Y-%m-%d %H:%M:%S")}))
                        print(f"  take {n}: Paul {wins[0]}, Feyd {wins[1]} -> {path.name}", flush=True)
                time.sleep(0.25)
            except Exception:
                # The window was closed.
                break
        try:
            b.close()
        except Exception:
            pass


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "arrakeen")
