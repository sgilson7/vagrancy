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
    page.click(f'[data-copy="{key}"]')


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
    road = json.loads((ROOT / "data" / "road.json").read_text())["stops"]
    click_copy(page, "menu.road.label")
    page.wait_for_selector("#road-tree .node")
    fails += every_visible_line_is_a_copy_string(page, name + " (road)")
    # One row per number of requirements, and every fight in its row.
    for s in road:
        lvl = page.locator(f'#road-tree .level[data-level="{len(s["requires"])}"] [data-stop="{s["id"]}"]').count()
        if lvl != 1:
            fails.append(f"{name}: {s['id']} is not in the row for {len(s['requires'])} requirements")
    # A line for every requirement.
    want = sum(len(s["requires"]) for s in road)
    got = page.locator("#road-tree .wires path.unmet, #road-tree .wires path.met").count()
    if got != want:
        fails.append(f"{name}: the tree draws {got} lines for {want} requirements")
    # On a fresh road only the top fight is open.
    if page.locator("#road-tree .node.open").count() != 1 or page.locator(f'[data-stop="{road[0]["id"]}"].open').count() != 1:
        fails.append(f"{name}: a fresh road does not open exactly its first fight")
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
    road = json.loads((ROOT / "data" / "road.json").read_text())["stops"]
    click_copy(page, "menu.road.label")
    page.wait_for_selector("#weapons")
    if page.locator("#weapons .weapon").count() != len(carryable):
        fails.append(f"{name}: the road shows {page.locator('#weapons .weapon').count()} weapons, not the {len(carryable)} a player can carry")
    if page.locator('#weapons [data-weapon="longsword"]').count():
        fails.append(f"{name}: the enemies' longsword is offered to the player")
    click_copy(page, "menu.back.label")
    # A save that has beaten the lamplighter, so the scimitar is open.
    save = page.evaluate("window.vagrancy.save()")
    save["state"]["road"]["best"] = {"lamplighter": {"losses": 1, "ticks": 3000}}
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
    for view, sel, want in (("chart", "#road-tree.chart .node", len(road)), ("chapters", "#chapters li", len({len(s['requires']) for s in road}))):
        page.click(f'#road-views [data-view="{view}"]')
        page.wait_for_selector(sel)
        if page.locator(sel).count() != want:
            fails.append(f"{name}: the {view} shows {page.locator(sel).count()} of {want}")
        fails += every_visible_line_is_a_copy_string(page, f"{name} (road as {view})")
        if view == "chart":
            lines = page.locator("#road-tree.chart .wires path.met, #road-tree.chart .wires path.unmet").count()
            if lines != len(road) - 1:
                fails.append(f"{name}: the chart draws {lines} routes, not one into each of the {len(road) - 1} fights below the first")
    page.click('#chapters [data-chapter="1"]')
    page.wait_for_selector("#chapter-stages .stage")
    if page.locator("#chapter-stages .stage").count() != sum(1 for s in road if len(s["requires"]) == 1):
        fails.append(f"{name}: chapter 2 does not show the fights with one requirement")
    page.click('#road-views [data-view="tree"]')
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
        print(f"ok: {name}: a weapon won on the road is carried into the yard; the enemies' longsword is not offered; the road draws as a tree, a chart and chapters")
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
    if page.locator("#road-tree .node.open").count() != 2:
        fails.append(f"{name}: a win at the first stop with a round lost opens {page.locator('#road-tree .node.open').count()} fights, not the two that ask only for a win")
    click_copy(page, "menu.back.label")
    # A save from a newer version, and a file that is not a save.
    for content, key in ((json.dumps({**data, "version": 9}), "settings.save.error.newer"), ("not a save", "settings.save.error.format")):
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
