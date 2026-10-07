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
                if not k.startswith("_") and k != "review":
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
            // A value filled from data (a key's name) is not a sentence.
            if (el.hasAttribute('data-fill')) return;
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
    # The road's other views are folded away (Sam, 2026-10-07); unfold first.
    if key.startswith("road.view.") and page.locator("#road-views-box:not([open])").count():
        page.click('#road-views-box summary')
    page.click(f'[data-copy="{key}"]')


def pick_view(page, view):
    if page.locator("#road-views-box:not([open])").count():
        page.click('#road-views-box summary')
    page.click(f'#road-views [data-view="{view}"]')


@check
def a_replay_downloaded_and_loaded_plays_the_same_match(page, name, tmp=Path("/tmp")):
    fails = []
    click_copy(page, "menu.practice.label")
    page.wait_for_selector('[data-copy="practice.step.shoulder"]')
    fails += every_visible_line_is_a_copy_string(page, name + " (practice)")
    page.keyboard.down("KeyI")
    page.wait_for_timeout(900)
    page.keyboard.up("KeyI")
    page.keyboard.down("KeyL")
    page.wait_for_timeout(500)
    page.keyboard.up("KeyL")
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


@check
def a_youtube_link_plays_in_a_visible_player_only_when_asked(page, name):
    """Sam's override of brief 0.5: a YouTube link the player pastes loops in
    a visible player. Nothing reaches YouTube until Play is pressed; this
    walk's own page never presses it, so its no-request-left-the-origin line
    still covers everything else."""
    fails = []
    click_copy(page, "menu.settings.label")
    page.wait_for_selector('[data-copy="settings.youtube.title"]')
    page.fill("#youtube-link", "not a link")
    click_copy(page, "settings.youtube.play.label")
    page.wait_for_selector('[data-copy="settings.youtube.error"]')
    fails += every_visible_line_is_a_copy_string(page, name + " (settings, YouTube)")
    if page.locator("#youtube-dock").count():
        fails.append(f"{name}: a link that is not YouTube's opened a player")
    ids = page.evaluate("""async () => { const y = await import('./youtube.js');
        return ['https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=3', 'https://youtu.be/dQw4w9WgXcQ?si=x',
                'https://m.youtube.com/shorts/dQw4w9WgXcQ', 'dQw4w9WgXcQ', 'https://example.com/watch?v=dQw4w9WgXcQ',
                'https://www.youtube.com/watch?v=short'].map(y.videoId); }""")
    want = ["dQw4w9WgXcQ"] * 4 + [None, None]
    if ids != want:
        fails.append(f"{name}: YouTube links read as {ids}, not {want}")
    click_copy(page, "menu.back.label")
    # Pressing Play, in a context of its own, with YouTube answered by a stub
    # so the gate does not depend on it.
    ctx = page.context.browser.new_context()
    try:
        p2 = ctx.new_page()
        away = []
        p2.on("request", lambda r: away.append(r.url) if not r.url.startswith(ORIGIN) else None)
        p2.route("https://www.youtube-nocookie.com/**", lambda route: route.fulfill(status=200, content_type="text/html", body="<p>stub</p>"))
        p2.goto(ORIGIN + "/", wait_until="load")
        p2.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
        click_copy(p2, "menu.settings.label")
        # Sam's video is filled in already: Play alone plays it.
        default = p2.evaluate("async () => { const y = await import('./youtube.js'); return [y.DEFAULT_LINK, y.videoId(y.DEFAULT_LINK)]; }")
        if p2.input_value("#youtube-link") != default[0] or not default[1]:
            fails.append(f"{name}: the link field holds {p2.input_value('#youtube-link')!r}, not Sam's {default[0]!r}")
        if away:
            fails.append(f"{name}: YouTube was contacted before Play: {away}")
        click_copy(p2, "settings.youtube.play.label")
        p2.wait_for_selector("#youtube-dock iframe")
        src = p2.get_attribute("#youtube-dock iframe", "src")
        if not src.startswith(f"https://www.youtube-nocookie.com/embed/{default[1]}?"):
            fails.append(f"{name}: Play with nothing pasted loads {src}")
        p2.click('#youtube-dock [data-copy="settings.youtube.stop.label"]')
        p2.fill("#youtube-link", "https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        click_copy(p2, "settings.youtube.play.label")
        p2.wait_for_selector("#youtube-dock iframe")
        src = p2.get_attribute("#youtube-dock iframe", "src")
        if not src.startswith("https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ?") or "loop=1" not in src:
            fails.append(f"{name}: the player loads {src}")
        box = p2.locator("#youtube-dock iframe").bounding_box()
        if not box or box["width"] < 200 or box["height"] < 200 or not p2.is_visible("#youtube-dock iframe"):
            fails.append(f"{name}: the YouTube player is not visible at 200 by 200: {box}")
        others = [u for u in away if not u.startswith("https://www.youtube-nocookie.com/")]
        if others:
            fails.append(f"{name}: Play reached beyond YouTube: {others}")
        # It outlives Settings, and Stop removes it.
        click_copy(p2, "menu.back.label")
        if not p2.is_visible("#youtube-dock iframe"):
            fails.append(f"{name}: the YouTube player went away with Settings")
        p2.click('#youtube-dock [data-copy="settings.youtube.stop.label"]')
        if p2.locator("#youtube-dock").count():
            fails.append(f"{name}: Stop left the YouTube player")
    finally:
        ctx.close()
    if not fails:
        print(f"ok: {name}: a YouTube link plays in a visible player, and only once asked")
    return fails


def red_pixels(page):
    """Count canvas pixels in the red band (hue 330-20, saturation >= 0.20),
    the same band crates/content/tests/palette.rs holds the palette to.
    The palette lint guards the file; this guards what was drawn, blending
    and antialiasing included."""
    return page.evaluate("""() => {
        const c = document.getElementById('stage');
        const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data;
        let red = 0, drawn = 0;
        for (let i = 0; i < d.length; i += 4) {
            const r = d[i], g = d[i + 1], b = d[i + 2];
            const mx = Math.max(r, g, b), mn = Math.min(r, g, b), dl = mx - mn;
            if (dl === 0) continue;
            drawn++;
            let h;
            if (mx === r) h = ((60 * (g - b) / dl) % 360 + 360) % 360;
            else if (mx === g) h = 60 * (b - r) / dl + 120;
            else h = 60 * (r - g) / dl + 240;
            const sum = mx + mn, den = sum <= 255 ? sum : 510 - sum;
            const sat = dl / Math.max(1, den);
            if (sat >= 0.20 && (h >= 330 || h <= 20)) red++;
        }
        return [red, drawn];
    }""")


@check
def a_round_that_ends_pops_a_card_and_a_headshot_holds_the_clock(page, name):
    # Sam: "pop a popup on the screen about how someone died, and if someone
    # gets headshot, stop the simulation to focus on it". A replay whose
    # first round ends with a cut across the head, and plays on after it
    # (`lab fixture-headshot`).
    fails = []
    fixture = ROOT / "testing" / "replays" / "headshot.replay"
    with page.expect_file_chooser() as fc:
        click_copy(page, "menu.replay.label")
    fc.value.set_files(str(fixture))
    page.wait_for_selector('[data-copy="replay.playing"]')
    try:
        page.wait_for_selector("#death-popup.headshot:not([hidden])", timeout=30000)
    except Exception:
        # Say where the page was, so a failure on CI names its cause; it
        # timed out there once without reproducing here (SECOND-ORDER-M5
        # row 53).
        state = page.evaluate("({ tick: window.vagrancy.tick(), phase: document.body.dataset.phase, done: document.body.dataset.replayDone || null, card: (document.getElementById('death-popup') || {}).className || null })")
        fails.append(f"{name}: no headshot card within 30 s; the page was at {state}")
        click_copy(page, "replay.stop.label")
        return fails
    fails += every_visible_line_is_a_copy_string(page, name + " (a headshot's card)")
    # Sam: "colored based on who won the round".
    if page.locator("#death-popup.won-0, #death-popup.won-1").count() != 1:
        fails.append(f"{name}: the headshot's card is not in the winner's color")
    if page.inner_text("#death-popup .popup-head") != COPY["results"]["popup"]["head"]:
        fails.append(f"{name}: a headshot's card says {page.inner_text('#death-popup .popup-head')!r}")
    t0 = page.evaluate("window.vagrancy.tick()")
    page.wait_for_timeout(800)
    t1 = page.evaluate("window.vagrancy.tick()")
    if t1 != t0:
        fails.append(f"{name}: the clock ran from tick {t0} to {t1} during a headshot")
    # And then it runs on: the next round starts.
    try:
        page.wait_for_function(f"window.vagrancy.tick() > {t0} + 30", timeout=5000)
    except Exception:
        fails.append(f"{name}: the clock never ran again after the headshot at tick {t0}")
    page.wait_for_function("document.body.dataset.replayDone === '1'", timeout=20000)
    click_copy(page, "replay.stop.label")
    if not fails:
        print(f"ok: {name}: a headshot pops its card, holds the clock at tick {t0}, then lets it run on")
    return fails


@check
def a_scripted_match_runs_to_its_result_and_nothing_drawn_is_red(page, name):
    fails = []
    fixture = ROOT / "testing" / "replays" / "match.replay"
    with page.expect_file_chooser() as fc:
        click_copy(page, "menu.replay.label")
    fc.value.set_files(str(fixture))
    page.wait_for_selector('[data-copy="replay.playing"]')
    page.wait_for_function("document.body.dataset.replayDone === '1'", timeout=60000)
    page.wait_for_selector('[data-copy="results.versus.match"]', timeout=5000)
    got, want = page.evaluate("[window.vagrancy.checksum(), window.vagrancy.recordedChecksum()]")
    text = page.inner_text('[data-copy="results.versus.match"]')
    if got != want:
        fails.append(f"{name}: the scripted match ended on {got}, and it recorded {want}")
    elif text != "Indigo won the match, 3 rounds to 0.":
        fails.append(f"{name}: the scripted match ended with {text!r}")
    else:
        print(f"ok: {name}: the scripted match ran to its result ({text}) on its recorded checksum {got}")
    fails += every_visible_line_is_a_copy_string(page, name + " (match result)")
    red, drawn = red_pixels(page)
    if red:
        fails.append(f"{name}: {red} of {drawn} colored canvas pixels are in the red band after the cuts")
    else:
        print(f"ok: {name}: none of {drawn} colored canvas pixels is in the red band, after the cuts")
    click_copy(page, "replay.stop.label")
    return fails


@check
def two_players_start_a_match_at_one_keyboard(page, name):
    fails = []
    click_copy(page, "menu.local.label")
    page.wait_for_selector('[data-copy="local.intro"]')
    fails += every_visible_line_is_a_copy_string(page, name + " (local)")
    click_copy(page, "local.start.label")
    page.wait_for_selector('[data-copy="hud.round"]')
    page.keyboard.down("KeyD"); page.keyboard.down("KeyJ")
    page.wait_for_timeout(500)
    page.keyboard.up("KeyD"); page.keyboard.up("KeyJ")
    want = COPY["hud"]["round"].replace("{round}", "1")
    if page.inner_text('[data-copy="hud.round"]') != want:
        fails.append(f"{name}: the HUD says {page.inner_text('#hud')!r}")
    else:
        print(f"ok: {name}: a match at one keyboard starts and its HUD reads from core")
    fails += every_visible_line_is_a_copy_string(page, name + " (match)")
    click_copy(page, "menu.back.label")
    return fails


@check
def a_key_can_be_rebound_and_a_clash_is_refused(page, name):
    fails = []
    click_copy(page, "menu.settings.label")
    page.wait_for_selector('#keys')
    page.click('button.bind[data-group="solo"][data-action="shoulder_up"]')
    page.keyboard.press("KeyZ")
    page.wait_for_selector('button.bind[data-group="solo"][data-action="shoulder_up"]')
    if page.inner_text('button.bind[data-group="solo"][data-action="shoulder_up"]') != "Z":
        fails.append(f"{name}: rebinding shoulder-up to Z did not take")
    page.click('button.bind[data-group="solo"][data-action="shoulder_down"]')
    page.keyboard.press("KeyZ")
    page.wait_for_selector('[data-copy="settings.keys.conflict"]')
    want = COPY["settings"]["keys"]["conflict"].replace("{key}", "Z").replace("{action}", COPY["settings"]["keys"]["actions"]["shoulder_up"])
    if page.inner_text('[data-copy="settings.keys.conflict"]') != want:
        fails.append(f"{name}: a clashing key was refused with the wrong words")
    else:
        print(f"ok: {name}: a key can be rebound, and a key already in use is refused with its sentence")
    fails += every_visible_line_is_a_copy_string(page, name + " (keys)")
    click_copy(page, "settings.keys.reset.label")
    click_copy(page, "menu.back.label")
    return fails


@check
def the_online_lobby_says_only_its_own_words(page, name):
    # Opening the lobby connects to nothing; only hosting or joining does.
    click_copy(page, "menu.online.label")
    page.wait_for_selector('[data-copy="online.host_room.label"]')
    fails = every_visible_line_is_a_copy_string(page, name + " (online lobby)")
    click_copy(page, "menu.back.label")
    return fails


@check
def the_road_is_a_tree_that_says_what_opens_each_fight_and_its_first_fight_starts(page, name):
    fails = []
    road = shown_road()
    click_copy(page, "menu.road.label")
    page.wait_for_selector("#road-tree .node")
    # Arcade mode opens on the chart (Sam, 2026-10-06).
    if page.locator("#road-tree.chart").count() != 1:
        fails.append(f"{name}: arcade mode did not open on the chart")
    click_copy(page, "road.view.tree.label")
    page.wait_for_selector("#road-tree .level")
    fails += every_visible_line_is_a_copy_string(page, name + " (road)")
    # One row per number of requirements, and every fight in its row; the
    # final fight names its own row (content::road::Stop::row).
    for s in road:
        row = s.get("row", len(s["requires"]))
        lvl = page.locator(f'#road-tree .level[data-level="{row}"] [data-stop="{s["id"]}"]').count()
        if lvl != 1:
            fails.append(f"{name}: {s['id']} is not in row {row}")
    # A line for every requirement.
    want = sum(len(s["requires"]) for s in road)
    got = page.locator("#road-tree .wires path.unmet, #road-tree .wires path.met").count()
    if got != want:
        fails.append(f"{name}: the tree draws {got} lines for {want} requirements")
    # On a fresh road only the first row is open.
    first_row = [s["id"] for s in road if not s["requires"]]
    if page.locator("#road-tree .node.open").count() != len(first_row) or any(page.locator(f'[data-stop="{i}"].open').count() != 1 for i in first_row):
        fails.append(f"{name}: a fresh road does not open exactly its first row, {first_row}")
    # Hovering a locked fight lists what it asks for, and lights its lines.
    last = road[-1]
    page.hover(f'[data-stop="{last["id"]}"]')
    page.wait_for_selector("#road-tip:not([hidden])")
    lines = page.locator("#road-tip p").count()
    hot = page.locator("#road-tree .wires path.hot").count()
    if lines != 1 + len(last["requires"]) or hot != len(last["requires"]):
        fails.append(f"{name}: hovering {last['id']} shows {lines} lines and lights {hot}, for {len(last['requires'])} requirements")
    fails += every_visible_line_is_a_copy_string(page, name + " (road, hovering a locked fight)")
    # Hovering a line says the one requirement it stands for.
    page.mouse.move(0, 0)
    page.evaluate("document.querySelector('#road-tree .wires path.hit').scrollIntoView({ block: 'center' })")
    hit = page.locator("#road-tree .wires path.hit").first
    box = hit.bounding_box()
    pt = page.evaluate("""() => { const p = document.querySelector('#road-tree .wires path.hit');
        const l = p.getTotalLength(); const q = p.getPointAtLength(l / 2); const m = p.getScreenCTM();
        return [q.x * m.a + m.e, q.y * m.d + m.f]; }""")
    page.mouse.move(pt[0], pt[1])
    try:
        page.wait_for_selector("#road-tip:not([hidden])", timeout=3000)
        said = page.inner_text("#road-tip")
        if page.locator("#road-tree .wires path.hot").count() != 1 or not said.startswith("Beat "):
            fails.append(f"{name}: hovering a line says {said!r}")
    except Exception:
        fails.append(f"{name}: hovering a line showed nothing (box {box})")
    page.mouse.move(0, 0)
    # The thresher's numbers come from its pilot.
    page.click('[data-stop="thresher"]')
    does = page.inner_text('#stop-detail [data-copy="opponents.thresher.does"]')
    if "1.5 seconds" not in does:
        fails.append(f"{name}: the thresher's introduction does not state its pause from the pilot data: {does!r}")
    if page.locator('#stop-detail [data-copy="road.fight.label"]').count():
        fails.append(f"{name}: a locked fight offers a Fight button")
    page.click(f'[data-stop="{road[0]["id"]}"]')
    click_copy(page, "road.fight.label")
    page.wait_for_selector('[data-copy="hud.round"]')
    page.wait_for_timeout(500)
    fails += every_visible_line_is_a_copy_string(page, name + " (road fight)")
    click_copy(page, "results.to_road.label")
    page.wait_for_selector("#road-tree .node")
    click_copy(page, "menu.back.label")
    if not fails:
        print(f"ok: {name}: the road is a tree with a row per count of requirements and a line per requirement; hovering a fight or a line says what it asks for; the first fight starts")
    return fails


@check
def the_tutorial_is_a_map_of_missions_and_its_first_mission_is_played_with_the_keys(page, name):
    # Sam: "a dedicated tutorial mode that is a map of missions following the
    # teaching flow" of the knowledge-component analysis.
    fails = []
    missions = json.loads((ROOT / "data" / "tutorial.json").read_text())["missions"]
    click_copy(page, "menu.tutorial.label")
    page.wait_for_selector("#road-tree .node")
    fails += every_visible_line_is_a_copy_string(page, name + " (tutorial)")
    if page.locator("#road-tree [data-mission]").count() != len(missions):
        fails.append(f"{name}: the map shows {page.locator('#road-tree [data-mission]').count()} of {len(missions)} missions")
    want = sum(len(m.get("requires", [])) for m in missions)
    if page.locator("#road-tree .wires path.met, #road-tree .wires path.unmet").count() != want:
        fails.append(f"{name}: the map draws the wrong number of lines for {want} requirements")
    last = missions[-1]
    page.hover(f'[data-mission="{last["id"]}"]')
    page.wait_for_selector("#road-tip:not([hidden])")
    if page.locator("#road-tip p").count() != 1 + len(last["requires"]):
        fails.append(f"{name}: hovering the last mission does not list what it requires")
    fails += every_visible_line_is_a_copy_string(page, name + " (tutorial, hovering a locked mission)")
    page.mouse.move(0, 0)
    # The first mission: one whole turn of the shoulder, by holding its key.
    first = missions[0]["id"]
    page.click(f'[data-mission="{first}"]')
    click_copy(page, "tutorial.start.label")
    page.wait_for_selector("#mission-goal")
    fails += every_visible_line_is_a_copy_string(page, name + " (tutorial mission)")
    page.keyboard.down("KeyI")
    try:
        page.wait_for_function("document.body.dataset.mission === 'done'", timeout=15000)
    except Exception:
        fails.append(f"{name}: holding the shoulder key did not finish the first mission in 15 s")
    finally:
        page.keyboard.up("KeyI")
    if not fails:
        fails += every_visible_line_is_a_copy_string(page, name + " (tutorial mission done)")
        if first not in page.evaluate("window.vagrancy.save().state.tutorial"):
            fails.append(f"{name}: the finished mission is not in the save")
        opened = [m["id"] for m in missions if m.get("requires") == [first]]
        page.keyboard.press("Enter")
        page.wait_for_selector("#road-tree .node")
        for o in opened:
            if page.locator(f'[data-mission="{o}"].open').count() != 1:
                fails.append(f"{name}: finishing {first} did not open {o}")
        if page.locator(f'[data-mission="{first}"].won').count() != 1:
            fails.append(f"{name}: the map does not show {first} as done")
    click_copy(page, "menu.back.label")
    if not fails:
        print(f"ok: {name}: the tutorial maps its missions with their requirements; the first is played with the keys and opens the next")
    return fails


@check
def a_weapon_won_on_the_road_is_carried_and_the_road_draws_three_ways(page, name):
    # Sam: change your weapon like Weapon Master, unlocking new ones as you
    # go; and road designs closer to Weapon Master's map.
    fails = []
    weapons = json.loads((ROOT / "data" / "weapons.json").read_text())["weapons"]
    carryable = [w for w in weapons if not w.get("enemy_only")]
    road = shown_road()
    click_copy(page, "menu.road.label")
    page.wait_for_selector("#weapons")
    shown = page.locator('#weapons .weapon:not([data-weapon="four_arms"])').count()
    if shown != len(carryable):
        fails.append(f"{name}: the road shows {shown} weapons, not the {len(carryable)} a player can carry")
    # The cursed blade waits for the final fight, shown locked and last;
    # four arms is not shown until it is won.
    if page.locator('#weapons [data-weapon="longsword"].locked').count() != 1:
        fails.append(f"{name}: a fresh save does not show the cursed blade locked")
    # Sam, 2026-10-07: nothing says where the cursed blade is until it is found.
    if page.locator('#weapons [data-weapon="longsword"] [data-copy="road.weapon_unfound"]').count() != 1 \
            or page.locator('#weapons [data-weapon="longsword"] [data-copy="road.weapon_locked"]').count() != 0:
        fails.append(f"{name}: the locked cursed blade's card says how it is won")
    if page.evaluate("[...document.querySelectorAll('#weapons .weapon')].map(e => e.dataset.weapon).pop()") != "longsword":
        fails.append(f"{name}: the cursed blade is not last in the weapon list")
    if page.locator('#weapons [data-weapon="four_arms"]').count() != 0:
        fails.append(f"{name}: a fresh save shows four arms")
    click_copy(page, "menu.back.label")
    # A save that has beaten the pilgrim, who carries the scimitar.
    save = page.evaluate("window.vagrancy.save()")
    save["state"]["road"]["best"] = {"pilgrim": {"losses": 1, "ticks": 3000, "with": ["sword"]}}
    page.evaluate("s => localStorage.setItem('vagrancy.autosave', JSON.stringify(s))", save)
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.road.label")
    page.wait_for_selector("#weapons")
    page.click('[data-weapon="scimitar"] [data-copy="road.carry.label"]')
    page.wait_for_selector('[data-weapon="scimitar"].carried')
    fails += every_visible_line_is_a_copy_string(page, name + " (road, carrying the scimitar)")
    if page.evaluate("window.vagrancy.save().state.weapon") != "scimitar":
        fails.append(f"{name}: carrying the scimitar did not reach the save")
    # The three designs.
    for view, sel, want in (("chart", "#road-tree.chart .node", len(road)), ("sunburst", "#road-tree.sunburst .node", len(road)), ("chapters", "#chapters li", len({s.get("row", len(s['requires'])) for s in road}))):
        pick_view(page, view)
        page.wait_for_selector(sel)
        if page.locator(sel).count() != want:
            fails.append(f"{name}: the {view} shows {page.locator(sel).count()} of {want}")
        fails += every_visible_line_is_a_copy_string(page, f"{name} (road as {view})")
        if view == "chart":
            lines = page.locator("#road-tree.chart .wires path.met, #road-tree.chart .wires path.unmet").count()
            below = sum(1 for s in road if s["requires"])
            if lines != below:
                fails.append(f"{name}: the chart draws {lines} routes, not one into each of the {below} fights below the first row")
    page.click('#chapters [data-chapter="1"]')
    page.wait_for_selector("#chapter-stages .stage")
    if page.locator("#chapter-stages .stage").count() != sum(1 for s in road if len(s["requires"]) == 1):
        fails.append(f"{name}: chapter 2 does not show the fights with one requirement")
    pick_view(page, "tree")
    page.wait_for_selector("#road-tree.rows .node")
    click_copy(page, "menu.back.label")
    # In the yard the scimitar is drawn with its two edges.
    click_copy(page, "menu.practice.label")
    page.wait_for_timeout(500)
    if page.evaluate("window.vagrancy.edges()") != [2]:
        fails.append(f"{name}: the yard's sword has {page.evaluate('window.vagrancy.edges()')} edges, not the scimitar's 2")
    click_copy(page, "menu.back.label")
    # Back to a fresh save for the checks that follow.
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    if not fails:
        print(f"ok: {name}: a weapon won on the road is carried into the yard; the enemies' longsword is not offered; the road draws as a tree, a chart, chapters and a sunburst")
    return fails


@check
def a_flanked_fight_puts_an_opponent_on_each_side_and_ledges_are_drawn(page, name):
    # Sam: "an enemy on each side of you", "fights that have platforms",
    # and "select different weapons when fighting online, and different
    # maps with platforms".
    fails = []
    road = shown_road()
    maps = {m["id"]: m for m in json.loads((ROOT / "data" / "maps.json").read_text())["maps"]}
    stop = next(s for s in road if s.get("companion") and s.get("map"))
    # A save that has met every requirement on the road.
    save = page.evaluate("window.vagrancy.save()")
    save["state"]["road"]["best"] = {s["id"]: {"losses": 0, "ticks": 1, "with": ["sword"], "headshot": True, "untouched": True} for s in road}
    page.evaluate("s => localStorage.setItem('vagrancy.autosave', JSON.stringify(s))", save)
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.road.label")
    page.wait_for_selector("#road-tree .node")
    page.click(f'[data-stop="{stop["id"]}"]')
    page.wait_for_selector('#stop-detail [data-copy="road.companion"]')
    if not page.locator('#stop-detail [data-copy="road.map_heading"]').count():
        fails.append(f"{name}: the {stop['id']}'s card does not say what ground it is fought on")
    fails += every_visible_line_is_a_copy_string(page, name + " (a flanked fight's card)")
    click_copy(page, "road.fight.label")
    page.wait_for_timeout(700)
    got = (page.evaluate("window.vagrancy.fighters()"), page.evaluate("window.vagrancy.platforms()"))
    want = (3, len(maps[stop["map"]]["platforms"]))
    if got != want:
        fails.append(f"{name}: the {stop['id']} draws {got[0]} fighters and {got[1]} ledges, not {want[0]} and {want[1]}")
    fails += every_visible_line_is_a_copy_string(page, name + " (a flanked fight)")
    click_copy(page, "results.to_road.label")
    page.wait_for_selector("#road-tree .node")
    click_copy(page, "menu.back.label")
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    # At one keyboard, on the bridge.
    click_copy(page, "menu.local.label")
    page.wait_for_selector("#map-local")
    page.select_option("#map-local", "bridge")
    page.wait_for_selector('[data-copy="maps.bridge.desc"]')
    fails += every_visible_line_is_a_copy_string(page, name + " (local, the bridge chosen)")
    click_copy(page, "local.start.label")
    page.wait_for_timeout(500)
    if page.evaluate("window.vagrancy.platforms()") != len(maps["bridge"]["platforms"]):
        fails.append(f"{name}: a match at one keyboard on the bridge draws {page.evaluate('window.vagrancy.platforms()')} ledges")
    click_copy(page, "menu.back.label")
    # The online lobby offers a weapon and the ground.
    click_copy(page, "menu.online.label")
    page.wait_for_selector("#weapon-online")
    if not page.locator("#map-online").count():
        fails.append(f"{name}: the online lobby offers no choice of ground")
    click_copy(page, "menu.back.label")
    if not fails:
        print(f"ok: {name}: the {stop['id']} puts an opponent on each side of the player on ledges; a match at one keyboard can be on the bridge; online, a player picks a weapon and the host the ground")
    return fails


@check
def the_encyclopedia_and_training_show_each_opponents_tree_and_light_it_in_a_fight(page, name):
    # Sam: "add an encyclopedia and a mode where you can fight each enemy
    # individually after you unlock them ... see each behavior tree state
    # they enter as you fight them".
    fails = []
    road = shown_road()
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.encyclopedia.label")
    page.wait_for_selector("#entries .entry")
    if page.locator("#entries .entry").count() != len(road):
        fails.append(f"{name}: the encyclopedia has {page.locator('#entries .entry').count()} entries, not {len(road)}")
    first_row = sum(1 for s in road if not s["requires"])
    if page.locator("#entries .entry.open").count() != first_row:
        fails.append(f"{name}: a fresh save shows {page.locator('#entries .entry.open').count()} opponents in full, not the {first_row} open from the start")
    page.wait_for_function("[...document.querySelectorAll('#entries .tree-img')].every(i => i.complete)", timeout=15000)
    broken = page.evaluate("[...document.querySelectorAll('#entries .tree-img')].filter(i => !i.naturalWidth).length")
    if broken:
        fails.append(f"{name}: {broken} drawn trees in the encyclopedia did not load")
    fails += every_visible_line_is_a_copy_string(page, name + " (encyclopedia)")
    click_copy(page, "menu.back.label")
    # Training with nothing beaten says so.
    click_copy(page, "menu.train.label")
    page.wait_for_selector('[data-copy="train.none"]')
    click_copy(page, "menu.back.label")
    # A save that has beaten three, one of them a flanked stop.
    won = ["thresher", "archivist", "tea_picker"]
    save = page.evaluate("window.vagrancy.save()")
    save["state"]["road"]["best"] = {k: {"losses": 1, "ticks": 9999, "with": ["sword"]} for k in won}
    page.evaluate("s => localStorage.setItem('vagrancy.autosave', JSON.stringify(s))", save)
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.train.label")
    page.wait_for_selector("#train-tiles .tile")
    if page.locator("#train-tiles .tile").count() != len(won):
        fails.append(f"{name}: training offers {page.locator('#train-tiles .tile').count()} opponents, not the {len(won)} beaten")
    fails += every_visible_line_is_a_copy_string(page, name + " (training)")
    page.click('#train-tiles [data-stop="tea_picker"]')
    page.click('#train-detail [data-copy="train.fight.label"]')
    page.wait_for_selector('[data-copy="hud.round"]')
    seen = set()
    for _ in range(20):
        page.wait_for_timeout(100)
        for tr in page.evaluate("window.vagrancy.traces()"):
            if tr["active"]:
                seen.add(tr["seat"])
    if seen != {1, 2}:
        fails.append(f"{name}: in a flanked training fight the trees lit for seats {sorted(seen)}, not both opponents")
    fails += every_visible_line_is_a_copy_string(page, name + " (a training fight)")
    click_copy(page, "menu.train.label")
    click_copy(page, "menu.back.label")
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    if not fails:
        print(f"ok: {name}: the encyclopedia shows {len(road)} opponents with their drawn trees; training offers the beaten ones, and both trees in a flanked fight light as the opponents run them")
    return fails


def shown_road():
    """The road a player sees before the final fight: the secret fight (the
    guardian deity) is left out until the village deity is beaten."""
    return [st for st in json.loads((ROOT / "data" / "road.json").read_text())["stops"] if not st.get("secret")]


@check
def the_guardian_deity_appears_once_the_village_deity_is_beaten(page, name):
    # Sam: "a secret final boss that only appears on the chart /
    # visualizations after ... the bubble only appears on the chart after
    # you've defeated the local deity". His two test saves, loaded as he
    # would load them.
    fails = []
    for save, shown in (("all-but-the-village-deity.save.json", False), ("all-but-the-guardian-deity.save.json", True)):
        text = (ROOT / "testing" / "saves" / save).read_text()
        page.evaluate("t => localStorage.setItem('vagrancy.autosave', t)", text)
        page.reload(wait_until="load")
        page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
        click_copy(page, "menu.road.label")
        page.wait_for_selector("#road-tree .node")
        page.wait_for_selector('[data-copy="road.quest"]')
        for view in ("tree", "chart", "sunburst"):
            click_copy(page, f"road.view.{view}.label")
            page.wait_for_selector("#road-tree .node")
            n = page.locator('#road-tree [data-stop="guardian_deity"]').count()
            if n != (1 if shown else 0):
                fails.append(f"{name}: with {save} the {view} shows the guardian deity {n} times")
        if shown and page.locator('#road-tree [data-stop="guardian_deity"].open').count() != 1:
            fails.append(f"{name}: with {save} the guardian deity is not open")
        click_copy(page, "menu.back.label")
        click_copy(page, "menu.encyclopedia.label")
        page.wait_for_selector("#entries .entry")
        if page.locator('#entries [data-copy="opponents.guardian_deity.name"]').count() != (1 if shown else 0):
            fails.append(f"{name}: with {save} the encyclopedia's guardian deity is wrong")
        click_copy(page, "menu.back.label")
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    if not fails:
        print(f"ok: {name}: the guardian deity is hidden until the village deity is beaten, then on each view and open")
    return fails


@check
def watch_mode_pits_two_beaten_opponents_and_lights_both_trees(page, name):
    # Sam: "an enemy vs enemy ai watching mode ... each characters behavior
    # tree is being shown above their head as they battle".
    fails = []
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.watch.label")
    page.wait_for_selector('[data-copy="watch.none"]')
    click_copy(page, "menu.back.label")
    won = ["thresher", "juggler", "local_deity"]
    save = page.evaluate("window.vagrancy.save()")
    save["state"]["road"]["best"] = {k: {"losses": 1, "ticks": 9999, "with": ["sword"]} for k in won}
    page.evaluate("s => localStorage.setItem('vagrancy.autosave', JSON.stringify(s))", save)
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.watch.label")
    page.wait_for_selector("#watch-0")
    offered = page.evaluate("[...document.querySelectorAll('#watch-0 option')].map(o => o.value)")
    if sorted(offered) != sorted(won):
        fails.append(f"{name}: watch mode offers {offered}, not the beaten {won}")
    fails += every_visible_line_is_a_copy_string(page, name + " (watch)")
    page.select_option("#watch-0", "local_deity")
    page.select_option("#watch-1", "juggler")
    click_copy(page, "watch.start.label")
    page.wait_for_selector('[data-copy="hud.round"]')
    # Quarter speed: the clock runs about a quarter as fast (Sam: "1/2 speed
    # and at 1/4 speed to make the behavior trees easier to observe").
    # Measured from the match's start, slow first, while no round can be
    # over: measured later, Firefox's match had ended and its clock stood.
    def rate():
        t0 = page.evaluate("window.vagrancy.tick()")
        page.wait_for_timeout(1000)
        return page.evaluate("window.vagrancy.tick()") - t0
    # Against the game's own rate (sim::balance::TICKS_PER_SECOND): a
    # second at full speed can fall across the pause between rounds.
    import re
    tps = int(re.search(r"pub const TICKS_PER_SECOND: u32 = (\d+);", (ROOT / "crates" / "sim" / "src" / "balance.rs").read_text()).group(1))
    click_copy(page, "watch.speed.quarter.label")
    slow = rate()
    click_copy(page, "watch.speed.full.label")
    if not (0 < slow <= tps * 0.45):
        fails.append(f"{name}: at quarter speed the clock ran {slow} ticks a second; the game runs {tps}")
    seen = set()
    for _ in range(20):
        page.wait_for_timeout(100)
        for tr in page.evaluate("window.vagrancy.traces()"):
            if tr["active"]:
                seen.add(tr["seat"])
    if seen != {0, 1}:
        fails.append(f"{name}: in watch mode the trees lit for seats {sorted(seen)}, not both sides")
    score = page.locator('[data-copy="hud.score"]').inner_text()
    copy = json.loads((ROOT / "data" / "copy.en.json").read_text())
    for o in ("local_deity", "juggler"):
        if copy["opponents"][o]["name"] not in score:
            fails.append(f"{name}: the score line does not name {o}: {score!r}")
    # Hidden, no tree is drawn.
    click_copy(page, "watch.hide_trees.label")
    page.wait_for_timeout(300)
    if page.evaluate("window.vagrancy.trees()")["trees"] != 0:
        fails.append(f"{name}: hiding the trees left them drawn")
    click_copy(page, "watch.show_trees.label")
    # Two of the widest trees, whose homes overlap: they float apart
    # (content::bubbles) and do not overlap.
    click_copy(page, "menu.watch.label")
    page.wait_for_selector("#watch-0")
    page.select_option("#watch-0", "local_deity")
    page.select_option("#watch-1", "local_deity")
    click_copy(page, "watch.start.label")
    page.wait_for_selector('[data-copy="hud.round"]')
    page.wait_for_timeout(1500)
    shown = page.evaluate("window.vagrancy.trees()")
    bs = shown["bubbles"]
    if shown["trees"] != 2 or len(bs) != 2:
        fails.append(f"{name}: watch mode draws {shown['trees']} trees, not both")
    elif abs(bs[0]["x"] - bs[1]["x"]) < (bs[0]["w"] + bs[1]["w"]) / 2 and abs(bs[0]["y"] - bs[1]["y"]) < (bs[0]["h"] + bs[1]["h"]) / 2:
        fails.append(f"{name}: the two trees overlap: {[(round(b['x']), round(b['y']), round(b['w']), round(b['h'])) for b in bs]}")
    fails += every_visible_line_is_a_copy_string(page, name + " (watching)")
    click_copy(page, "menu.watch.label")
    click_copy(page, "menu.back.label")
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    if not fails:
        print(f"ok: {name}: watch mode offers the beaten opponents, names both on the score line, and lights both trees")
    return fails


@check
def story_mode_plays_a_scene_and_moves_on(page, name):
    # Sam, 2026-10-06: "start building the story mode". The first chapter's
    # first scene, played by keyboard; the run moves on as its outcome says.
    fails = []
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.story.label")
    page.wait_for_selector("#story-chapters .chapter")
    story = json.loads((ROOT / "data" / "story.json").read_text())
    if page.locator("#story-chapters .chapter").count() != len(story["chapters"]):
        fails.append(f"{name}: the story shows {page.locator('#story-chapters .chapter').count()} chapters, not {len(story['chapters'])}")
    if page.locator("#story-chapters .chapter.open").count() != 1:
        fails.append(f"{name}: a fresh save opens {page.locator('#story-chapters .chapter.open').count()} chapters, not the first")
    fails += every_visible_line_is_a_copy_string(page, name + " (story)")
    page.click('[data-chapter="0"] [data-copy="story.start.label"]')
    page.wait_for_selector('[data-copy="story.begin.label"]')
    fails += every_visible_line_is_a_copy_string(page, name + " (a scene's card)")
    page.click('[data-copy="story.begin.label"]')
    page.wait_for_selector('[data-copy="hud.round"]')
    for _ in range(6):
        page.keyboard.down("KeyD"); page.keyboard.down("KeyI")
        try:
            page.wait_for_function("document.body.dataset.storyOutcome || ['round_over','match_over'].includes(document.body.dataset.phase)", timeout=30000)
        finally:
            page.keyboard.up("KeyD"); page.keyboard.up("KeyI")
        if page.evaluate("document.body.dataset.storyOutcome"):
            break
    try:
        page.wait_for_function("document.body.dataset.storyNext", timeout=10000)
    except Exception:
        fails.append(f"{name}: the fight never moved the run on")
    # The first scene is a stage to cross, and whether holding keys gets past
    # the opponents depends on the spawn; what is checked is that the run
    # moves on the way the outcome says (a win to the next scene, a loss to
    # the scene again with a life gone).
    got = (page.evaluate("document.body.dataset.storyOutcome"), page.evaluate("document.body.dataset.storyNext"))
    if got not in (("won", "scene"), ("lost", "retry"), ("out_of_time", "retry")):
        fails.append(f"{name}: the first scene ended {got}, which is not how a run moves on")
    fails += every_visible_line_is_a_copy_string(page, name + " (after a fight)")
    page.evaluate("delete document.body.dataset.storyOutcome; delete document.body.dataset.storyNext")
    click_copy(page, "menu.back.label")
    click_copy(page, "menu.back.label")
    if not fails:
        print(f"ok: {name}: story mode shows its chapters, plays the first scene ({got[0]}) and moves the run on ({got[1]})")
    return fails


@check
def story_mode_runs_in_full_and_opens_its_extra_chapter(page, name):
    # Sam, 2026-10-06: "a run where you start from chapter 1 and have to
    # complete all the chapters until the final chapter on 3 lives ... based
    # on how far you managed to get ... you unlock fights in an EXTRA chapter
    # ... the final one being having to fight the hardest enemy in the game".
    fails = []
    story = json.loads((ROOT / "data" / "story.json").read_text())
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.story.label")
    page.wait_for_selector("#story-extra .chapter")
    n = page.locator("#story-extra .chapter").count()
    if n != len(story["extra"]) or page.locator("#story-extra .chapter.open").count() != 0:
        fails.append(f"{name}: a fresh save shows {n} extra fights with {page.locator('#story-extra .chapter.open').count()} open")
    click_copy(page, "story.full.start.label")
    page.wait_for_selector('[data-copy="story.begin.label"]')
    if "1" not in page.locator('[data-copy="road.chapter"]').inner_text():
        fails.append(f"{name}: a full run did not start at the first chapter")
    click_copy(page, "menu.back.label")
    # A save whose full run cleared the walk: each extra fight open, and the
    # last is the guardian deity.
    text = (ROOT / "testing" / "saves" / "all-but-the-guardian-deity.save.json").read_text()
    page.evaluate("t => localStorage.setItem('vagrancy.autosave', t)", text)
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.story.label")
    page.wait_for_selector("#story-extra .chapter")
    if page.locator("#story-extra .chapter.open").count() != len(story["extra"]):
        fails.append(f"{name}: a save that cleared the walk opens {page.locator('#story-extra .chapter.open').count()} extra fights")
    fails += every_visible_line_is_a_copy_string(page, name + " (story with the extra chapter)")
    page.click(f'[data-extra="{len(story["extra"]) - 1}"] [data-copy="story.extra.start.label"]')
    page.wait_for_selector('[data-copy="story.begin.label"]')
    if page.locator('[data-copy="story.extra.heading"]').count() != 1 or page.locator('[data-copy="opponents.guardian_deity.name"]').count() != 1:
        fails.append(f"{name}: the last extra fight's card is not the guardian deity's")
    fails += every_visible_line_is_a_copy_string(page, name + " (the last extra fight)")
    click_copy(page, "menu.back.label")
    click_copy(page, "menu.back.label")
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    if not fails:
        print(f"ok: {name}: a full run starts at chapter one, and the extra chapter opens by the best run, ending with the guardian deity")
    return fails


@check
def the_bt_lab_shows_what_each_tree_ran_and_the_keys_it_pressed(page, name):
    # Sam, 2026-10-06: a classroom demo of "how the high level behaviors and
    # nodes get converted into controls execution, like how does the
    # behavior tree high guard get converted to inputs".
    fails = []
    page.goto(ORIGIN + "/bt-lab.html", wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    fails += every_visible_line_is_a_copy_string(page, name + " (BT Lab)")
    if page.locator("#tree-panel .node").count() < 3:
        fails.append(f"{name}: the BT Lab drew no tree")
    # Paused, a tick at a time: the keys the panel lights are the keys core
    # pressed for the inspected seat.
    page.click("#lab-pause")
    for _ in range(12):
        before = page.evaluate("window.btlab.tick()")
        page.click("#lab-step")
        page.wait_for_function(f"window.btlab.tick() > {before}", timeout=5000)
        rep = page.evaluate("window.btlab.report()")
        keys = page.evaluate("window.btlab.keys()")
        st = page.evaluate("window.btlab.state()")
        r = next((x for x in rep if x["seat"] == st["inspect"]), None)
        if r is None or r["keys"] != keys[st["inspect"]]:
            fails.append(f"{name}: the lab reported keys {r and r['keys']} for seat {st['inspect']}, which pressed {keys[st['inspect']]}")
            break
        lit = page.evaluate("[...document.querySelectorAll('#keys-panel .cap.down')].reduce((a, c) => a | +c.dataset.bit, 0)")
        if lit != keys[st["inspect"]]:
            fails.append(f"{name}: the lab lit the keys {lit:#b} where core pressed {keys[st['inspect']]:#b}")
            break
    # Sam, 2026-10-07: the lesson is pages to scroll through, the lecture
    # beside a fight that shows it. The page in view plays, and one at a time.
    # Sam, 2026-10-07: each page's node type is in the fight beside it,
    # ringed in its tree, and the lab and the pages start at an eighth speed.
    rings = page.evaluate("window.btlab.lessonRings()")
    bare = [k for k, n in rings.items() if n < 1]
    if bare:
        fails.append(f"{name}: lesson pages whose fight has none of the nodes they are about: {bare}")
    if page.evaluate("window.btlab.state().speed") != 0.125:
        fails.append(f"{name}: the lab does not start at an eighth of full speed")
    page.evaluate("document.getElementById('lesson-factor').scrollIntoView()")
    page.wait_for_function("window.btlab.lesson() && window.btlab.lesson().page === 'factor' && window.btlab.lesson().tick > 4", timeout=30000)
    page.wait_for_function("window.btlab.lesson().rings > 0", timeout=30000)
    if page.evaluate("window.btlab.lesson().speed") != 0.125:
        fails.append(f"{name}: the lesson's fights do not play at an eighth of full speed")
    if page.locator("#lesson-factor svg.lecture-fig").count() != 2:
        fails.append(f"{name}: the factoring page did not draw the lecture's two trees")
    fails += every_visible_line_is_a_copy_string(page, name + " (BT Lab, a lesson page)")
    # The page about chance plays the same fight again from the same start.
    page.evaluate("document.getElementById('lesson-chance').scrollIntoView()")
    page.wait_for_function("window.btlab.lesson() && window.btlab.lesson().page === 'chance' && window.btlab.lesson().tick > 8", timeout=30000)
    seed = page.evaluate("window.btlab.lesson().seed")
    page.click("#lesson-chance .lesson-replay")
    again = page.evaluate("window.btlab.lesson()")
    if again["seed"] != seed or again["tick"] > 5:
        fails.append(f"{name}: playing again from the same start gave {again}, from seed {seed}")
    page.evaluate("document.getElementById('lab-section').scrollIntoView()")
    page.wait_for_function("window.btlab.lesson() === null", timeout=5000)
    # The pose page opens the lab with high guard running, and shows its
    # controller.
    page.evaluate("document.getElementById('lesson-pose').scrollIntoView()")
    page.click("#lesson-pose .lesson-open")
    page.wait_for_selector('[data-copy="btlab.pose.rule"]', timeout=15000)
    # Each page's fight opened in the lab, and left to run a moment: a move
    # with no name once threw in the animation loop and froze the page.
    errors = []
    page.on("pageerror", lambda e: errors.append(str(e)))
    for pid in page.evaluate("[...document.querySelectorAll('.lesson-page .lesson-open')].map(b => b.closest('.lesson-page').dataset.page)"):
        page.evaluate(f"document.getElementById('lesson-{pid}').scrollIntoView()")
        page.wait_for_timeout(500)
        page.click(f"#lesson-{pid} .lesson-open")
        page.wait_for_timeout(800)
    if errors:
        fails.append(f"{name}: opening the lesson's fights in the lab threw: {errors[:2]}")
    before = page.evaluate("window.btlab.tick()")
    page.wait_for_timeout(1500)
    if not page.evaluate("window.btlab.state().paused") and page.evaluate("window.btlab.tick()") <= before:
        fails.append(f"{name}: the lab stopped running after the lesson's fights were opened in it")
    fails += every_visible_line_is_a_copy_string(page, name + " (BT Lab, a pose)")
    # The editor: a number out of range is refused with its rule named; a
    # tree that runs can be watched; a share code loads back the same tree.
    page.evaluate("localStorage.removeItem('vagrancy.btlab.tree')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    page.wait_for_selector('#editor-status[data-copy="btlab.editor.ok"]')
    num = page.locator('#editor li.rule[data-rule="2"] input[type=number]').first
    num.fill("5000"); num.dispatch_event("change")
    page.wait_for_selector('#editor-status[data-copy="btlab.editor.refuse.number"]')
    if page.locator('#editor li.rule.refused[data-rule="2"]').count() != 1:
        fails.append(f"{name}: the editor did not mark the refused rule")
    num = page.locator('#editor li.rule[data-rule="2"] input[type=number]').first
    num.fill("180"); num.dispatch_event("change")
    page.wait_for_selector('#editor-status[data-copy="btlab.editor.ok"]')
    page.click("#editor-watch")
    # The lab plays at an eighth of full speed, about eight ticks a second.
    try:
        page.wait_for_function("window.btlab.tick() >= 8", timeout=15000)
    except Exception:
        pass
    if page.evaluate("window.btlab.tick()") < 8 or page.locator("#tree-panel .node").count() < 4:
        fails.append(f"{name}: watching the written tree did not run it")
    page.click("#editor-make-code")
    code = page.input_value("#editor-code")
    page.click('[data-copy="btlab.editor.reset.label"]')
    page.fill("#editor-code-in", code)
    page.click("#editor-load-code")
    if page.locator('#editor li.rule[data-rule="2"] input[type=number]').first.input_value() != "180":
        fails.append(f"{name}: a share code did not load back the tree it was made from")
    fails += every_visible_line_is_a_copy_string(page, name + " (BT Lab editor)")
    page.evaluate("localStorage.removeItem('vagrancy.btlab.tree')")
    page.goto(ORIGIN + "/", wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    if not fails:
        print(f"ok: {name}: the BT Lab lesson plays the page in view and replays a start, the lab lights the tree and shows the keys core pressed tick by tick, the pose page stops on high guard, and the editor refuses, runs and shares a tree")
    return fails


@check
def the_chart_marks_the_next_fight_over_region_art_and_chapters_show_what_is_left(page, name):
    # Sam, 2026-10-07: art for each region, a wide chart with the deepest
    # fight to take next set apart, and chapter cards that show a locked
    # fight's requirements and drop them once it is open.
    fails = []
    road = shown_road()
    lvl = lambda st: st.get("row", len(st["requires"]))
    won = [st["id"] for st in road if lvl(st) <= 1]
    save = page.evaluate("window.vagrancy.save()")
    save["state"]["road"]["best"] = {k: {"losses": 1, "ticks": 9999, "with": ["sword"]} for k in won}
    page.evaluate("s => localStorage.setItem('vagrancy.autosave', JSON.stringify(s))", save)
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.road.label")
    page.wait_for_selector("#road-tree.chart .node")
    nxt = page.evaluate("[...document.querySelectorAll('#road-tree.chart .node.next-up')].map(e => [e.dataset.stop, e.className])")
    if len(nxt) != 1 or "open" not in nxt[0][1] or nxt[0][0] in won:
        fails.append(f"{name}: the chart marks {nxt} as next, not one open fight not yet won")
    elif lvl(next(st for st in road if st["id"] == nxt[0][0])) != 2:
        fails.append(f"{name}: the next fight {nxt[0][0]} is not in the deepest open row")
    art = page.evaluate("Promise.all([...document.querySelectorAll('#road-tree.chart .band-art')].map(e => new Promise(r => { const i = new Image(); i.onload = () => r(i.naturalWidth); i.onerror = () => r(0); i.src = getComputedStyle(e).backgroundImage.slice(5, -2); })))")
    if not art or 0 in art:
        fails.append(f"{name}: region art that did not load: {art}")
    # Sam, 2026-10-07: each fight in a scene of its own, set off from the art.
    page.wait_for_function("[...document.querySelectorAll('#road-tree.chart .node img.seal-art')].every(i => i.complete)", timeout=15000)
    scenes = page.evaluate("[...document.querySelectorAll('#road-tree.chart .node')].map(n => { const i = n.querySelector('img.seal-art'); return [n.dataset.stop, i ? i.naturalWidth : -1]; })")
    unscened = [s for s, w in scenes if w <= 0]
    if unscened:
        fails.append(f"{name}: chart fights with no scene drawn: {unscened[:5]}")
    click_copy(page, "road.view.chapters.label")
    page.wait_for_selector("#chapter-stages")
    page.click('[data-chapter="2"]')
    page.wait_for_timeout(200)
    bad_open = page.evaluate("[...document.querySelectorAll('#chapter-stages .stage.open')].filter(e => e.querySelector('p')).length")
    locked = page.evaluate("[...document.querySelectorAll('#chapter-stages .stage.locked')].map(e => e.querySelectorAll('p').length)")
    if bad_open or not locked or 0 in locked:
        fails.append(f"{name}: in the chapter view open fights show {bad_open} requirement lists and locked ones show {locked}")
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    if not fails:
        print(f"ok: {name}: the chart marks {nxt[0][0]} as next over {len(art)} regions' art with each of {len(scenes)} fights in its scene, and the chapter cards show what is left")
    return fails


@check
def the_arena_runs_fights_on_its_own_with_no_bot(page, name):
    # The stream's page (Sam, 2026-10-07): with no bot it runs exhibitions,
    # so the stream always has a fight; its words are the copy file's.
    fails = []
    page.goto(ORIGIN + "/arena.html", wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    page.wait_for_timeout(1500)
    st = page.evaluate("window.arena.state()")
    if st["tick"] < 20 or len(st["fighters"]) != 2:
        fails.append(f"{name}: the arena is not running a fight: {st}")
    fails += every_visible_line_is_a_copy_string(page, name + " (arena)")
    page.goto(ORIGIN + "/", wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    if not fails:
        print(f"ok: {name}: the arena runs a fight on its own")
    return fails


@check
def a_save_file_round_trips_and_a_bad_one_is_refused(page, name, tmp=Path("/tmp")):
    fails = []
    click_copy(page, "menu.settings.label")
    page.wait_for_selector('#save')
    with page.expect_download() as d:
        click_copy(page, "settings.save.download.label")
    path = tmp / f"vagrancy-gate-{name}.save.json"
    d.value.save_as(path)
    data = json.loads(path.read_text())
    if data.get("format") != "vagrancy.save" or data["state"]["road"]["best"] != {}:
        fails.append(f"{name}: the downloaded save is not a fresh save: {data}")
    # The same file with the first stop cleared, loaded back.
    first = json.loads((ROOT / "data" / "road.json").read_text())["stops"][0]["id"]
    data["state"]["road"]["best"] = {first: {"losses": 1, "ticks": 3000}}
    path.write_text(json.dumps(data))
    with page.expect_file_chooser() as fc:
        click_copy(page, "settings.save.load.label")
    fc.value.set_files(str(path))
    page.wait_for_selector('[data-copy="settings.save.loaded"]')
    click_copy(page, "menu.back.label")
    click_copy(page, "menu.road.label")
    page.wait_for_selector("#road-tree .node")
    page.click(f'#road-tree [data-stop="{first}"]')
    if page.locator(f'[data-stop="{first}"].won').count() != 1 or page.locator('#stop-detail [data-copy="road.cleared"]').count() != 1:
        fails.append(f"{name}: after loading a save with the first stop won, the road does not say so")
    # The fights that ask only for a win there, besides the rest of the
    # first row, counted from the data.
    road_stops = json.loads((ROOT / "data" / "road.json").read_text())["stops"]
    first_id = road_stops[0]["id"]
    want_open = sum(1 for s in road_stops if not s["requires"]) - 1 + sum(1 for s in road_stops if s["requires"] == [{"beat": first_id}])
    if page.locator("#road-tree .node.open").count() != want_open:
        fails.append(f"{name}: a win at the first stop with a round lost leaves {page.locator('#road-tree .node.open').count()} fights open, not {want_open}")
    click_copy(page, "menu.back.label")
    # A save from a newer version, and a file that is not a save.
    for content, key in ((json.dumps({**data, "version": data["version"] + 1}), "settings.save.error.newer"), ("not a save", "settings.save.error.format")):
        bad = tmp / f"vagrancy-gate-{name}.bad.json"
        bad.write_text(content)
        click_copy(page, "menu.settings.label")
        with page.expect_file_chooser() as fc:
            click_copy(page, "settings.save.load.label")
        fc.value.set_files(str(bad))
        page.wait_for_selector(f'[data-copy="{key}"]')
        fails += every_visible_line_is_a_copy_string(page, f"{name} ({key})")
        click_copy(page, "menu.back.label")
    # And the convenience copy survives a reload.
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    if list(page.evaluate("window.vagrancy.save().state.road.best")) != [first]:
        fails.append(f"{name}: the road progress did not survive a reload")
    if not fails:
        print(f"ok: {name}: a save downloads, loads back with its progress, survives a reload, and newer or foreign files are refused by name")
    return fails


@check
def enter_goes_on_without_the_mouse(page, name):
    # Sam: "you should be able to press a button to reset the match / go to
    # the next battle instead of having to click". Win at the first stop by
    # keyboard alone (it does not fight back), pressing Enter after each
    # round, and Enter after the match starts the next stop.
    fails = []
    stops = [s["id"] for s in json.loads((ROOT / "data" / "road.json").read_text())["stops"]]
    # From a fresh road, whatever the checks before this one won: a flawless
    # win at the first stop opens the second, which comes first in the list
    # of what it opened.
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.road.label")
    page.click(f'[data-stop="{stops[0]}"]')
    page.click('#stop-detail [data-copy="road.fight.label"]')
    page.wait_for_selector('[data-copy="hud.round"]')
    rounds = 0
    for _ in range(8):
        page.keyboard.down("KeyD"); page.keyboard.down("KeyI")
        try:
            page.wait_for_function("['round_over','match_over'].includes(document.body.dataset.phase)", timeout=30000)
        except Exception:
            fails.append(f"{name}: no round ended against the {stops[0]} in 30 s")
            break
        finally:
            page.keyboard.up("KeyD"); page.keyboard.up("KeyI")
        phase = page.evaluate("document.body.dataset.phase")
        want = "road.fight.label" if phase == "match_over" else "results.next_round.label"
        focused = page.evaluate("document.activeElement && document.activeElement.dataset.copy")
        if focused != want:
            fails.append(f"{name}: after a {phase}, the focus is on {focused!r}, not {want!r}")
            break
        if phase == "match_over":
            # The match's own card follows the last round's, and names what
            # the win opened (Sam: "it should pop up in a box like the other
            # boxes").
            try:
                page.wait_for_selector('#death-popup [data-copy="results.popup.match_won"]', timeout=6000)
                if not page.locator('#death-popup [data-copy="road.opened"]').count():
                    fails.append(f"{name}: the match's card does not say what the win opened")
            except Exception:
                fails.append(f"{name}: no card came up for the won match")
            onward = page.inner_text('#result [data-copy="road.fight.label"]')
            page.keyboard.press("Enter")
            try:
                page.wait_for_function("document.body.dataset.phase === 'fight'", timeout=5000)
            except Exception:
                fails.append(f"{name}: after the match, Enter did not start the next stop: phase {page.evaluate('window.vagrancy.phase()')}")
                break
            nxt = COPY["opponents"][stops[1]]["name_mid"]
            if onward != COPY["road"]["fight"]["label"].replace("{opponent_mid}", nxt):
                fails.append(f"{name}: the button after a win says {onward!r}")
            elif page.evaluate("window.vagrancy.tick()") > 120:
                fails.append(f"{name}: Enter did not start a new match")
            else:
                print(f"ok: {name}: Enter took the next round {rounds} times, then started the next stop ({nxt})")
            break
        page.keyboard.press("Enter")
        try:
            page.wait_for_function("document.body.dataset.phase === 'fight'", timeout=5000)
        except Exception:
            fails.append(f"{name}: after round {rounds + 1}, Enter did not start the next round: phase "
                         f"{page.evaluate('window.vagrancy.phase()')}, focus "
                         f"{page.evaluate('document.activeElement && (document.activeElement.dataset.copy || document.activeElement.tagName)')}")
            break
        rounds += 1
    fails += every_visible_line_is_a_copy_string(page, name + " (after Enter)")
    click_copy(page, "results.to_road.label")
    click_copy(page, "menu.back.label")
    return fails


@check
def a_win_that_opens_nothing_names_the_next_goal(page, name):
    # Sam: "if you dont have any new fights available, prompt you to do the
    # next fight that you need to do that has constraints you havent met
    # yet". The first stop already won every way, so a second win there
    # opens nothing; its card names the goal core picked, and the first
    # button goes to that goal's fight.
    fails = []
    stops = [s["id"] for s in json.loads((ROOT / "data" / "road.json").read_text())["stops"]]
    weapons = [w["id"] for w in json.loads((ROOT / "data" / "weapons.json").read_text())["weapons"] if not w.get("enemy_only")]
    save = page.evaluate("window.vagrancy.save()")
    # The pilgrim beaten too, so the scimitar is won: the goal core picks is
    # then the ox herd's challenge, the tinker beaten carrying the scimitar.
    save["state"]["road"]["best"] = {stops[0]: {"losses": 0, "ticks": 1, "with": sorted(weapons), "headshot": True, "untouched": True},
                                     "pilgrim": {"losses": 1, "ticks": 9999, "with": ["sword"]}}
    page.evaluate("s => localStorage.setItem('vagrancy.autosave', JSON.stringify(s))", save)
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    click_copy(page, "menu.road.label")
    page.click(f'[data-stop="{stops[0]}"]')
    page.click('#stop-detail [data-copy="road.fight.label"]')
    page.wait_for_selector('[data-copy="hud.round"]')
    for _ in range(8):
        page.keyboard.down("KeyD"); page.keyboard.down("KeyI")
        try:
            page.wait_for_function("['round_over','match_over'].includes(document.body.dataset.phase)", timeout=30000)
        finally:
            page.keyboard.up("KeyD"); page.keyboard.up("KeyI")
        if page.evaluate("document.body.dataset.phase") == "match_over":
            break
        page.keyboard.press("Enter")
        page.wait_for_function("document.body.dataset.phase === 'fight'", timeout=5000)
    try:
        page.wait_for_selector('#death-popup [data-copy="road.next"]', timeout=6000)
    except Exception:
        fails.append(f"{name}: a win that opened nothing did not name a next goal")
    fails += every_visible_line_is_a_copy_string(page, name + " (the next goal)")
    first = page.evaluate("(() => { const b = document.querySelector('#result button'); return b && b.dataset.copy; })()")
    if first != "road.fight.label":
        fails.append(f"{name}: the first button after the win is {first!r}, not the next goal's fight")
    if not page.locator('#death-popup [data-copy="road.next_carry"]').count():
        fails.append(f"{name}: the card for a weapon challenge does not say going on switches the weapon")
    # Enter goes on: the weapon switches, and the fight carries the goal in
    # bold beside it (Sam, 2026-10-06).
    page.keyboard.press("Enter")
    try:
        page.wait_for_selector("#goal-callout", timeout=5000)
    except Exception:
        fails.append(f"{name}: the next goal's fight shows no goal beside it")
    if page.evaluate("window.vagrancy.save().state.weapon") != "scimitar":
        fails.append(f"{name}: going on to a scimitar challenge left the player with the {page.evaluate('window.vagrancy.save().state.weapon')}")
    if page.locator('#goal-callout [data-copy="road.req.with"]').count() != 1:
        fails.append(f"{name}: the goal beside the fight does not state the challenge")
    fails += every_visible_line_is_a_copy_string(page, name + " (the goal's fight)")
    click_copy(page, "results.to_road.label")
    click_copy(page, "menu.back.label")
    page.evaluate("localStorage.removeItem('vagrancy.autosave')")
    page.reload(wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    if not fails:
        print(f"ok: {name}: a won match pops its card; a win that opens nothing names the next goal, and Enter starts its fight with the goal beside it and the challenge's weapon in hand")
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
