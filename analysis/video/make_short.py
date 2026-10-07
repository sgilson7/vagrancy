#!/usr/bin/env python3
"""Edit a short, vertical clip of the wildest round endings (Sam, 2026-10-07:
"a shorts version of some of the craziest stuff that is happening on stream
ready to post to tik tok ... not have any behavior trees in it except for a
single clip where the behavior trees are easily legible and poignant ... very
engaging").

The rounds come from `lab highlights` (stream-like exhibitions, scored by
what each round's end does to the loser). The edit follows the short-form
research in the notebook: open mid-swing on the wildest cut, cut every two or
three seconds, ramp each kill into slow motion with a punch in on the cut,
one slower clip with the killer's tree drawn large, captions burned in, and
an impact on each kill; it ends on a cut, so it loops into its opening.

The game draws every frame (web/clip.html), stepped from here one frame at a
time, so the clip is exact whatever the machine; ffmpeg (from the
imageio-ffmpeg package) encodes it. No music: a sound is added in TikTok.

    .venv-test/bin/python analysis/video/make_short.py [out.mp4]
"""
import math, random, struct, subprocess, sys, wave
from pathlib import Path

import imageio_ffmpeg
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.home() / "Movies" / "Vagrancy" / "vagrancy-short.mp4"
PAGE = "http://127.0.0.1:8766/clip.html"
SITE = "sgilson7.github.io/vagrancy"
FPS = 30
TICKS = 2  # sim ticks per frame at full speed (60 ticks a second)

# The cut list: each a round from `lab highlights`, and how it is shown.
# lead: seconds of fight before the cut; slow: seconds of slow motion after.
CLIPS = [
    {"left": "herbalist", "right": "monk", "seed": 1370, "kill": 454, "lead": 0.45, "slow": 1.8, "caption": "clip.hook"},
    {"left": "general", "right": "tanner", "seed": 785, "kill": 373, "lead": 1.1, "slow": 1.3, "caption": "clip.hook"},
    {"left": "herbalist", "right": "monk", "seed": 374, "kill": 689, "lead": 1.0, "slow": 1.4},
    {"left": "drover", "right": "hare_hunter", "seed": 1311, "kill": 364, "lead": 4.2, "slow": 1.6, "trees": 0, "caption": "clip.trees"},
    {"left": "bell_founder", "right": "acrobat", "seed": 548, "kill": 295, "lead": 1.0, "slow": 1.2},
    {"left": "field_warden", "right": "potter", "seed": 112, "kill": 497, "lead": 1.0, "slow": 1.2},
    {"left": "ferryman", "right": "harpooner", "seed": 1482, "kill": 1602, "lead": 1.2, "slow": 2.6, "caption": "clip.cta"},
]


def ease(x):
    return x * x * (3 - 2 * x)


def impact_wav(path, times, length):
    """A low thump and a short hiss at each kill, synthesized: a sine that
    falls in pitch and fades, over a burst of noise."""
    rate = 44100
    n = int(length * rate) + rate
    buf = [0.0] * n
    rng = random.Random(7)
    for t0 in times:
        start = int(t0 * rate)
        for k in range(int(0.45 * rate)):
            if start + k >= n:
                break
            t = k / rate
            thump = math.sin(2 * math.pi * (70 - 40 * t) * t) * math.exp(-t * 9) * 0.9
            hiss = (rng.random() * 2 - 1) * math.exp(-t * 40) * 0.35
            buf[start + k] += thump + hiss
    peak = max(1e-9, max(abs(x) for x in buf))
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(b"".join(struct.pack("<h", int(32767 * 0.8 * x / peak)) for x in buf))


def main():
    OUT.parent.mkdir(parents=True, exist_ok=True)
    silent = OUT.with_suffix(".video.mp4")
    ff = imageio_ffmpeg.get_ffmpeg_exe()
    enc = subprocess.Popen([ff, "-y", "-loglevel", "error", "-f", "image2pipe", "-vcodec", "png", "-r", str(FPS), "-i", "-",
                            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "17", "-preset", "slow", str(silent)], stdin=subprocess.PIPE)
    hits, frames = [], 0
    with sync_playwright() as p:
        b = p.chromium.launch()
        page = b.new_page(viewport={"width": 1080, "height": 1920})
        errors = []
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.goto(PAGE)
        page.wait_for_function("document.body.dataset.ready === '1'", timeout=60000)
        for c in CLIPS:
            lead_ticks = int(c["lead"] * 60)
            got = page.evaluate("s => window.clip.load(s)", {"left": c["left"], "right": c["right"], "seed": c["seed"], "from": c["kill"] - lead_ticks})
            page.evaluate("([k, v]) => window.clip.caption(k, v)", [c.get("caption"), {"site": SITE}])
            trees = {"seat": c["trees"]} if "trees" in c else False
            # The tree clip plays at half speed, the tree large and legible;
            # the others at full speed with the fighters close.
            speed = 1 if trees else TICKS
            zoom = 1.45 if trees else 3.1
            ended_at, k = None, 0
            limit = int((c["lead"] + c["slow"]) * FPS * (2 if trees else 1)) + 90
            while k < limit:
                if ended_at is None:
                    ticks, punch = speed, 0.0
                else:
                    # The kill: ramp into slow motion and punch in, over a
                    # fifth of a second, then hold.
                    r = min(1.0, (k - ended_at) / 6)
                    ticks = speed + (0.35 - speed) * ease(r)
                    punch = ease(r)
                    if k - ended_at >= int(c["slow"] * FPS):
                        break
                state = page.evaluate("o => window.clip.frame(o)", {"ticks": ticks, "zoom": zoom, "punch": punch, "showTrees": trees, "treeScale": 2.6})
                if ended_at is None and state["ended"]:
                    ended_at = k
                    hits.append(frames / FPS)
                enc.stdin.write(page.locator("#clip-app").screenshot(type="png"))
                frames += 1
                k += 1
            page.evaluate("window.clip.ended = null")
            print(f"{c['left']} v {c['right']}: {k} frames, cut {'seen' if ended_at is not None else 'NOT REACHED'}", flush=True)
        b.close()
        if errors:
            print("page errors:", errors[:3])
    enc.stdin.close()
    enc.wait()
    wav = OUT.with_suffix(".hits.wav")
    impact_wav(wav, hits, frames / FPS)
    subprocess.run([ff, "-y", "-loglevel", "error", "-i", str(silent), "-i", str(wav), "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest", str(OUT)], check=True)
    silent.unlink()
    wav.unlink()
    print(f"wrote {OUT}: {frames / FPS:.1f} s, {len(hits)} kills")


if __name__ == "__main__":
    main()
