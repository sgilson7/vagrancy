"""Two browser tabs play one match online, introduced by pasted codes.

The gate's online mode (PLAN.md §8 Q18). `?ice=none` drops the STUN servers,
so the two tabs connect on host candidates alone and no request leaves the
origin; relays and STUN are checked by hand. Each session compares checksums
with the other every tick and stops on the first disagreement, so a match that
is still playing at the end stayed together: that is what this checks.

    python testing/online.py [chromium|firefox|webkit ...] [--seconds N]

With `--seconds 180` it is the referee: a scripted three-minute match,
written down in analysis/referee.md.
"""
import functools
import http.server
import json
import random
import socketserver
import sys
import threading
import time
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "dist" / "web"
PORT = 8133
ORIGIN = f"http://127.0.0.1:{PORT}"


class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a):
        pass


def serve():
    socketserver.TCPServer.allow_reuse_address = True
    httpd = socketserver.ThreadingTCPServer(("127.0.0.1", PORT), functools.partial(Quiet, directory=str(WEB)))
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


def page_for(browser, problems, offsite, who):
    ctx = browser.new_context()
    page = ctx.new_page()
    page.on("console", lambda m: problems.append(f"{who} console.{m.type}: {m.text}") if m.type == "error" else None)
    page.on("pageerror", lambda e: problems.append(f"{who} pageerror: {e}"))
    page.on("request", lambda r: offsite.append(r.url) if not r.url.startswith((ORIGIN, "blob:", "data:")) else None)
    page.goto(ORIGIN + "/?ice=none", wait_until="load")
    page.wait_for_function("document.body.dataset.ready === '1'", timeout=30000)
    return ctx, page


def play(browser, name, seconds):
    problems, offsite = [], []
    hctx, host = page_for(browser, problems, offsite, "host")
    jctx, join = page_for(browser, problems, offsite, "joiner")
    try:
        host.click('[data-copy="menu.online.label"]')
        host.click('[data-copy="online.host_paste.label"]')
        host.wait_for_function("document.body.dataset.invite === '1'", timeout=20000)
        invite = host.input_value("#invite")
        join.click('[data-copy="menu.online.label"]')
        join.click('[data-copy="online.join_paste.label"]')
        join.fill("#invite", invite)
        join.wait_for_function("document.body.dataset.reply === '1'", timeout=20000)
        reply = join.input_value("#reply")
        host.fill("#reply", reply)
        host.wait_for_selector('[data-copy="online.connected.host"]', timeout=30000)
        join.wait_for_selector('[data-copy="online.connected.join"]', timeout=30000)
        delay_text = host.inner_text('[data-copy="online.connected.host"]')
        # People take a while to press Start. The connected lobby once went
        # quiet and gave up after 15 s; wait longer than that, then check
        # both sides are still connected.
        time.sleep(20)
        if not (host.locator('[data-copy="online.connected.host"]').count() and join.locator('[data-copy="online.connected.join"]').count()):
            raise RuntimeError("the connected lobby did not survive 20 s of waiting for Start")
        host.click('[data-copy="online.start.label"]')
        join.wait_for_function("document.body.dataset.online === 'playing'", timeout=10000)
        print(f"ok: {name}: two tabs met by pasted codes: {delay_text}")
        keys = ["KeyI", "KeyK", "KeyJ", "KeyL", "KeyA", "KeyD"]
        rng = random.Random(7)
        end = time.time() + seconds
        while time.time() < end:
            for p in (host, join):
                k = rng.choice(keys)
                p.keyboard.down(k)
            time.sleep(0.25)
            for p in (host, join):
                for k in keys:
                    p.keyboard.up(k)
        h = host.evaluate("window.vagrancy.online()")
        j = join.evaluate("window.vagrancy.online()")
        return h, j, problems, offsite
    finally:
        hctx.close()
        jctx.close()


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    seconds = 6
    if "--seconds" in sys.argv:
        seconds = int(sys.argv[sys.argv.index("--seconds") + 1])
        args = [a for a in args if a != str(seconds)]
    engines = args or ["chromium"]
    if not (WEB / "index.html").exists():
        sys.exit("dist/web is not built. Run: make web")
    httpd = serve()
    fails, lines = [], []
    try:
        with sync_playwright() as pw:
            for name in engines:
                # Headless Firefox hides host addresses behind mDNS names and
                # will not use loopback, so two tabs on one machine never meet
                # without these. They are the test machine's settings, not the
                # page's; a player's browser connects through STUN.
                prefs = {"media.peerconnection.ice.obfuscate_host_addresses": False,
                         "media.peerconnection.ice.loopback": True} if name == "firefox" else None
                b = getattr(pw, name).launch(**({"firefox_user_prefs": prefs} if prefs else {}))
                try:
                    h, j, problems, offsite = play(b, name, seconds)
                except Exception as e:
                    fails.append(f"{name}: the online walk stopped: {str(e).splitlines()[0]}")
                    b.close()
                    continue
                b.close()
                together = h["status"]["kind"] in ("playing", "waiting_on") and j["status"]["kind"] in ("playing", "waiting_on")
                ticks = min(h["tick"], j["tick"])
                line = f"{name}: after {seconds} s the host is at tick {h['tick']} ({h['status']['kind']}), the joiner at {j['tick']} ({j['status']['kind']})"
                lines.append(line)
                if not together:
                    fails.append(f"{line}: they did not stay one match")
                elif ticks < seconds * 30:
                    fails.append(f"{line}: too few ticks; the match barely ran")
                else:
                    print(f"ok: {line}; every tick's checksum agreed")
                fails += [f"{name}: {p}" for p in problems]
                fails += [f"{name}: a request left the origin: {u}" for u in offsite]
    finally:
        httpd.shutdown()
    if seconds >= 60:
        out = ROOT / "analysis" / "referee.md"
        out.parent.mkdir(exist_ok=True)
        out.write_text("# Referee\n\nA scripted match between two tabs, written down by `testing/online.py --seconds "
                       f"{seconds}` at {time.strftime('%Y-%m-%d %H:%M')}.\n\n" + "\n".join(f"- {l}" for l in lines)
                       + ("\n\nThey stayed together.\n" if not fails else "\n\nThey did not:\n" + "\n".join(f"- {f}" for f in fails) + "\n"))
    if fails:
        print("\n".join(f"FAIL: {f}" for f in fails))
        sys.exit(1)
    print(f"ok: the online walk passed in {', '.join(engines)}")


if __name__ == "__main__":
    main()
