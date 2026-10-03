"""Drive the built page in real browsers and walk the deploy gate.

`cargo test` cannot reach any of this: whether the wasm instantiates in a
browser, whether the stamped imports resolve, whether a download produces a
file and a file input feeds it back, and whether the browser's arithmetic is
the native build's arithmetic.

A console error or a request that leaves the origin fails the run, so "nothing
is uploaded" is tested rather than asserted. Every visible line of text must
be a string from data/copy.en.json, so "the page writes no words of its own"
is tested from what the player sees, not from the source.

Ported in shape from gear-master-2d's testing/drive.py. `ORIGIN` points the
gate at a page that is already served, which in practice means the live one
(PLANNING-BRIEF Part G; GM2D calls the same variable GM2D_ORIGIN).

    python testing/drive.py [chromium|firefox|webkit ...]
    ORIGIN=https://sgilson7.github.io/vagrancy python testing/drive.py chromium firefox webkit
"""
import functools
import http.server
import json
import os
import re
import socketserver
import subprocess
import sys
import threading
import traceback
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
WEB = Path(os.environ.get("VAGRANCY_WEB") or ROOT / "dist" / "web")
PORT = 8131
LIVE = (os.environ.get("ORIGIN") or "").rstrip("/")
ORIGIN = LIVE or f"http://127.0.0.1:{PORT}"
COPY = json.loads((ROOT / "data" / "copy.en.json").read_text())

# Every check registers here, in order. A check takes (page, name) and returns
# a list of failures; it prints its own `ok:` line when it passes.
CHECKS = []


def check(fn):
    CHECKS.append(fn)
    return fn


class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a):
        pass


def serve():
    handler = functools.partial(Quiet, directory=str(WEB))
    socketserver.TCPServer.allow_reuse_address = True
    httpd = socketserver.TCPServer(("127.0.0.1", PORT), handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


def copy_strings():
    out = []

    def walk(v, path):
        if isinstance(v, dict):
            for k, c in v.items():
                if not k.startswith("_"):
                    walk(c, f"{path}.{k}" if path else k)
        elif isinstance(v, str):
            out.append((path, v))

    walk(COPY, "")
    return out


def copy_patterns():
    """Each copy string as a regex: placeholders match any text."""
    pats = []
    for key, s in copy_strings():
        s = s.replace("{game}", COPY["game"]["name"])
        parts = re.split(r"\{[a-z_.]+\}", s)
        pats.append((key, re.compile("^" + ".+?".join(re.escape(p) for p in parts) + "$", re.S)))
    return pats


PATTERNS = copy_patterns()


def text_is_copy(text):
    return any(p.match(text) for _, p in PATTERNS)


def expected_build():
    if LIVE:
        import urllib.request
        with urllib.request.urlopen(ORIGIN + "/build.txt") as r:
            return r.read().decode().split()
    return (WEB / "build.txt").read_text().split()


def open_page(browser):
    ctx = browser.new_context(accept_downloads=True)
    page = ctx.new_page()
    problems, offsite = [], []
    page.on("console", lambda m: problems.append(f"console.{m.type}: {m.text}")
            if m.type == "error" else None)
    page.on("pageerror", lambda e: problems.append(f"pageerror: {e}"))
    # blob: and data: URLs are the page talking to itself; anything else that
    # does not start with the origin has left it.
    page.on("request", lambda r: offsite.append(r.url)
            if not (r.url.startswith(ORIGIN) or r.url.startswith(("blob:", "data:"))) else None)
    page.goto(ORIGIN + "/", wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    return ctx, page, problems, offsite


# --- the checks --------------------------------------------------------------

@check
def the_page_names_its_build(page, name):
    want = expected_build()[0]
    got = page.inner_text("#build").strip()
    if got != f"Build {want}":
        return [f"{name}: the page says {got!r}, and the build is {want}"]
    print(f"ok: {name}: the page prints its build hash, {want}")
    return []


@check
def every_visible_line_is_a_copy_string(page, name):
    texts = page.evaluate("""() => {
        const out = [];
        const walk = (el) => {
            if (el.nodeType !== 1) return;
            const st = getComputedStyle(el);
            if (st.display === 'none' || st.visibility === 'hidden' || el.hidden) return;
            if (['SCRIPT', 'STYLE', 'NOSCRIPT'].includes(el.tagName)) return;
            let own = '';
            for (const c of el.childNodes) if (c.nodeType === 3) own += c.textContent;
            own = own.trim();
            if (own) out.push(own);
            for (const a of ['aria-label', 'title', 'placeholder', 'alt'])
                if (el.getAttribute(a)) out.push(el.getAttribute(a));
            for (const c of el.children) walk(c);
        };
        walk(document.body);
        out.push(document.title);
        return out;
    }""")
    bad = [t for t in texts if not text_is_copy(t)]
    if bad:
        return [f"{name}: text on the page that is not in the copy file: {bad[:5]}"]
    print(f"ok: {name}: all {len(texts)} visible lines are strings from the copy file")
    return []


@check
def the_stylesheet_applied_with_the_palettes_paper(page, name):
    # Packaging once shipped a stylesheet that was only its color variables
    # (SECOND-ORDER-M1); every check above still passed.
    pal = json.loads((ROOT / "data" / "palette.json").read_text())
    h = pal["paper"].lstrip("#")
    want = f"rgb({int(h[0:2], 16)}, {int(h[2:4], 16)}, {int(h[4:6], 16)})"
    got = page.evaluate("getComputedStyle(document.body).backgroundColor")
    if got != want:
        return [f"{name}: the page background is {got}, and the palette's paper is {want}"]
    print(f"ok: {name}: the stylesheet applied; the page is the palette's paper")
    return []


@check
def the_canvas_has_its_name(page, name):
    want = COPY["game"]["canvas_name"].replace("{game}", COPY["game"]["name"])
    got = page.get_attribute("#stage", "aria-label")
    if got != want:
        return [f"{name}: the canvas is named {got!r}, not {want!r}"]
    print(f"ok: {name}: the canvas is named {want!r}")
    return []


@functools.lru_cache(maxsize=None)
def native_script_checksum(ticks):
    out = subprocess.run(["cargo", "run", "-q", "--release", "-p", "lab", "--", "script-checksum", str(ticks)],
                         cwd=ROOT, capture_output=True, text=True, check=True)
    return out.stdout.strip()


@check
def the_browser_computes_what_the_native_build_computes(page, name):
    # D3: integers, so the same inputs give the same world everywhere. The
    # same fixed script runs in this engine's wasm and in a native build.
    want = native_script_checksum(600)
    got = page.evaluate("window.vagrancy.scriptChecksum(600)")
    if got != want:
        return [f"{name}: after 600 ticks of the script the browser says {got}, native says {want}"]
    print(f"ok: {name}: the fixed script ends on {got} in wasm and natively")
    return []


def click_copy(page, key):
    page.click(f'[data-copy="{key}"]')


@check
def a_replay_downloaded_and_loaded_plays_the_same_match(page, name, tmp=Path("/tmp")):
    fails = []
    click_copy(page, "menu.practice.label")
    page.wait_for_selector('[data-copy="practice.intro"]')
    fails += every_visible_line_is_a_copy_string(page, name + " (practice)")
    page.keyboard.down("KeyQ")
    page.wait_for_timeout(900)
    page.keyboard.up("KeyQ")
    page.keyboard.down("KeyP")
    page.wait_for_timeout(500)
    page.keyboard.up("KeyP")
    with page.expect_download() as d:
        click_copy(page, "replay.download.label")
    path = tmp / f"vagrancy-gate-{name}.replay"
    d.value.save_as(path)
    data = path.read_bytes()
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    with page.expect_file_chooser() as fc:
        click_copy(page, "menu.replay.label")
    fc.value.set_files(str(path))
    page.wait_for_selector('[data-copy="replay.playing"]')
    fails += every_visible_line_is_a_copy_string(page, name + " (replay)")
    page.wait_for_function("document.body.dataset.replayDone === '1'", timeout=60000)
    got, want, ticks = page.evaluate("[window.vagrancy.checksum(), window.vagrancy.recordedChecksum(), window.vagrancy.tick()]")
    if got != want:
        fails.append(f"{name}: the replay ended on {got}, and it recorded {want}")
    elif ticks < 60:
        fails.append(f"{name}: the replay is only {ticks} ticks long; the keys were not held")
    else:
        print(f"ok: {name}: moved the arm, downloaded {len(data)} bytes, reloaded, and the replay ended on {got} after {ticks} ticks")
    click_copy(page, "replay.stop.label")
    # A file that is not a replay is refused with its sentence.
    bad = tmp / f"vagrancy-gate-{name}.bad"
    bad.write_bytes(b"not a replay")
    with page.expect_file_chooser() as fc:
        click_copy(page, "menu.replay.label")
    fc.value.set_files(str(bad))
    page.wait_for_selector('[data-copy="replay.error.format"]')
    want_text = COPY["replay"]["error"]["format"].replace("{game}", COPY["game"]["name"])
    if page.inner_text('[data-copy="replay.error.format"]') != want_text:
        fails.append(f"{name}: a bad file was refused with the wrong words")
    else:
        print(f"ok: {name}: a file that is not a replay is refused with its sentence")
    click_copy(page, "menu.back.label")
    return fails


def tone(path):
    """One second of a 440 Hz tone, written by this test. It is never
    committed: no audio enters the repository (PLANNING-BRIEF 0.5)."""
    import math, struct, wave
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(22050)
        w.writeframes(b"".join(struct.pack("<h", int(8000 * math.sin(2 * math.pi * 440 * i / 22050)))
                               for i in range(22050)))


@check
def a_track_from_this_device_loops_during_a_fight(page, name, tmp=Path("/tmp")):
    fails = []
    track = tmp / f"vagrancy-gate-tone-{name}.wav"
    tone(track)
    click_copy(page, "menu.settings.label")
    page.wait_for_selector('[data-copy="settings.music.none"]')
    fails += every_visible_line_is_a_copy_string(page, name + " (settings)")
    with page.expect_file_chooser() as fc:
        click_copy(page, "settings.music.load.label")
    fc.value.set_files(str(track))
    want = COPY["settings"]["music"]["loaded"].replace("{file_name}", track.name)
    page.wait_for_selector('[data-copy="settings.music.loaded"]')
    if page.inner_text('[data-copy="settings.music.loaded"]') != want:
        fails.append(f"{name}: after loading, the settings say {page.inner_text('#music-status')!r}")
    page.check("#music-remember")
    page.wait_for_timeout(300)
    click_copy(page, "menu.back.label")
    click_copy(page, "menu.practice.label")
    page.wait_for_timeout(800)
    state = page.evaluate("window.vagrancy.music()")
    if not state["playing"]:
        fails.append(f"{name}: the loaded track is not playing during practice: {state}")
    else:
        print(f"ok: {name}: a track loaded from this device plays during practice")
    click_copy(page, "menu.back.label")
    if not page.evaluate("window.vagrancy.music()")["playing"] is False:
        fails.append(f"{name}: the track kept playing outside a fight")
    # The remembered copy survives a reload, from this browser's storage.
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.settings.label")
    page.wait_for_selector("#music-status")
    if page.inner_text("#music-status") != want:
        fails.append(f"{name}: after a reload the remembered track is gone: {page.inner_text('#music-status')!r}")
    else:
        print(f"ok: {name}: the remembered track came back after a reload, from this browser's storage")
    click_copy(page, "settings.music.remove.label")
    page.wait_for_selector('[data-copy="settings.music.none"]')
    click_copy(page, "menu.back.label")
    return fails


def walk(browser, name):
    fails = []
    ctx, page, problems, offsite = open_page(browser)
    try:
        for c in CHECKS:
            try:
                fails += c(page, name)
            except Exception as e:
                fails.append(f"{name}: {c.__name__} stopped: {str(e).splitlines()[0]}\n"
                             + traceback.format_exc())
    finally:
        ctx.close()
    fails += [f"{name}: {p}" for p in problems]
    fails += [f"{name}: a request left the origin: {u}" for u in offsite]
    if not problems:
        print(f"ok: {name}: no console error")
    if not offsite:
        print(f"ok: {name}: no request left the origin")
    return fails


def main():
    if not LIVE and not (WEB / "index.html").exists():
        sys.exit(f"{WEB} is not built. Run: make web")
    wanted = sys.argv[1:] or ["chromium"]
    httpd = None if LIVE else serve()
    if LIVE:
        build, commit = expected_build()
        head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True,
                              text=True).stdout.strip()
        print(f"walking {ORIGIN}: build {build} from commit {commit[:8]}")
        if commit != head:
            print(f"note: the live page was built from {commit[:8]}, and HEAD here is {head[:8]}")
    fails = []
    try:
        with sync_playwright() as p:
            for name in wanted:
                engine = getattr(p, name, None)
                if engine is None:
                    fails.append(f"{name}: no such browser")
                    continue
                try:
                    b = engine.launch()
                except Exception as e:
                    # A browser that is not installed is reported, not skipped:
                    # the gate names three and a quiet skip lets two rot.
                    fails.append(f"{name}: could not launch ({e})")
                    continue
                try:
                    mine = walk(b, name)
                except Exception as e:
                    mine = [f"{name}: the walk stopped: {e}\n{traceback.format_exc()}"]
                fails += mine
                b.close()
                if not mine:
                    print(f"ok: {name} walked the gate")
    finally:
        if httpd:
            httpd.shutdown()
    if fails:
        print("\n".join(f"FAIL: {f}" for f in fails))
        sys.exit(1)
    print(f"ok: the gate passed in {', '.join(wanted)}")


if __name__ == "__main__":
    main()
