#!/usr/bin/env python3
"""Edit short, vertical clips of the wildest round endings (Sam, 2026-10-07:
"a shorts version of some of the craziest stuff that is happening on stream
ready to post to tik tok ... not have any behavior trees in it except for a
single clip where the behavior trees are easily legible and poignant ... very
engaging"; then five variations, each testing a different idea).

The rounds come from `lab highlights` (stream-like exhibitions, scored by
what each round's end does to the loser). The edits follow the short-form
research in ~/Movies/Vagrancy/tiktok-post.txt: open on action, cut every one
to three seconds, ramp each kill into slow motion with a punch-in on the cut,
captions burned in, an impact on each kill, end on a cut so the clip loops.
At most one clip shows a behavior tree, drawn large.

The game draws every frame (web/clip.html), stepped from here one frame at a
time, so the clip is exact whatever the machine; ffmpeg (from the
imageio-ffmpeg package) encodes it. Music is optional and comes from a file
the caller names; it is mixed under the hits and never enters the repo.

    .venv-test/bin/python analysis/video/make_short.py VARIANT [--music FILE] [--out FILE]
    VARIANT: original | loop | tree_first | text_hook | countdown | fast
"""
import argparse, math, random, struct, subprocess, wave
from pathlib import Path

import imageio_ffmpeg
from playwright.sync_api import sync_playwright

PAGE = "http://127.0.0.1:8766/clip.html"
SITE = "sgilson7.github.io/vagrancy"
FPS = 30
TICKS = 2  # sim ticks per frame at full speed (60 ticks a second)

# Rounds from `lab highlights 1500` (seed, the tick of the deciding cut, and
# how many pieces the loser was cut into).
R = {
    "herbalist": {"left": "herbalist", "right": "monk", "seed": 1370, "kill": 454, "pieces": 125},
    "general": {"left": "general", "right": "tanner", "seed": 785, "kill": 373, "pieces": 75},
    "ferryman": {"left": "ferryman", "right": "harpooner", "seed": 1482, "kill": 1602, "pieces": 62},
    "bell": {"left": "bell_founder", "right": "acrobat", "seed": 548, "kill": 295, "pieces": 55},
    "boatwright": {"left": "boatwright", "right": "salt_trader", "seed": 1348, "kill": 368, "pieces": 47},
    "field": {"left": "field_warden", "right": "potter", "seed": 112, "kill": 497, "pieces": 45},
    "duelist": {"left": "duelist", "right": "sheaf_binder", "seed": 807, "kill": 450, "pieces": 45},
    "potter": {"left": "potter", "right": "sampler", "seed": 1475, "kill": 491, "pieces": 44},
    "headshot": {"left": "herbalist", "right": "monk", "seed": 374, "kill": 689, "pieces": 64},
    "drover": {"left": "drover", "right": "hare_hunter", "seed": 1311, "kill": 364, "pieces": 42, "trees": 0},
}


def clip(name, lead, slow, **kw):
    return {**R[name], "lead": lead, "slow": slow, **kw}


# The one clip with a tree: half speed, the killer's tree large.
TREE = dict(caption="clip.trees", half=True)
VARIANTS = {
    # The first cut: hook, quick kills, the tree, quick kills, the finale.
    "original": [
        clip("herbalist", 0.45, 1.8, caption="clip.hook"), clip("general", 1.1, 1.3, caption="clip.hook"),
        clip("headshot", 1.0, 1.4), clip("drover", 4.2, 1.6, **TREE), clip("bell", 1.0, 1.2), clip("field", 1.0, 1.2),
        clip("ferryman", 1.2, 2.6, caption="clip.cta"),
    ],
    # 1. Shorter: the three wildest, about 12 s, so more viewers finish it
    #    and it loops sooner.
    "loop": [
        clip("herbalist", 0.5, 2.6, caption="clip.hook"), clip("general", 1.0, 2.2, caption="clip.hook"),
        clip("ferryman", 1.2, 2.8, caption="clip.cta"),
    ],
    # 2. The tree first: the lit branch is the hook, then the kills.
    "tree_first": [
        clip("drover", 3.4, 2.0, **TREE), clip("herbalist", 0.8, 2.2, caption="clip.hook"),
        clip("general", 1.0, 1.8), clip("bell", 1.0, 1.8), clip("ferryman", 1.2, 2.8, caption="clip.cta"),
    ],
    # 3. A text hook: the count on screen from the first frame, and each
    #    kill's count after.
    "text_hook": [
        clip("herbalist", 0.6, 2.6, caption="clip.pieces", loud=True), clip("general", 1.0, 2.0, caption="clip.pieces", loud=True),
        clip("bell", 1.0, 2.0, caption="clip.pieces", loud=True), clip("drover", 3.8, 1.8, **TREE),
        clip("ferryman", 1.2, 2.8, caption="clip.cta"),
    ],
    # 4. A countdown, the worst last: viewers stay to see number one.
    "countdown": [
        clip("field", 2.0, 1.8, count=True), clip("bell", 1.2, 1.8, rank=4),
        clip("drover", 3.6, 1.8, rank=3, half=True), clip("general", 1.2, 2.0, rank=2),
        clip("herbalist", 1.2, 3.2, rank=1),
    ],
    # 5. Faster: a cut every second or so, harder ramps.
    "fast": [
        clip("herbalist", 0.3, 1.1, caption="clip.hook", ramp=0.22), clip("general", 0.5, 0.8, ramp=0.22),
        clip("headshot", 0.5, 0.8, ramp=0.22), clip("bell", 0.5, 0.8, ramp=0.22), clip("boatwright", 0.5, 0.8, ramp=0.22),
        clip("drover", 2.4, 1.1, **TREE), clip("field", 0.5, 0.8, ramp=0.22), clip("duelist", 0.5, 0.8, ramp=0.22),
        clip("potter", 0.5, 0.8, ramp=0.22), clip("ferryman", 0.6, 1.8, caption="clip.cta", ramp=0.22),
    ],
}


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
            buf[start + k] += math.sin(2 * math.pi * (70 - 40 * t) * t) * math.exp(-t * 9) * 0.9 + (rng.random() * 2 - 1) * math.exp(-t * 40) * 0.35
    peak = max(1e-9, max(abs(x) for x in buf))
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(b"".join(struct.pack("<h", int(32767 * 0.8 * x / peak)) for x in buf))


def render(variant, out, music=None):
    clips = VARIANTS[variant]
    out.parent.mkdir(parents=True, exist_ok=True)
    silent = out.with_suffix(".video.mp4")
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
        for c in clips:
            page.evaluate("s => window.clip.load(s)", {"left": c["left"], "right": c["right"], "seed": c["seed"], "from": c["kill"] - int(c["lead"] * 60)})
            if c.get("rank"):
                cap = ["clip.rank", {"n": c["rank"]}, "big"]
            elif c.get("count"):
                cap = ["clip.countdown", {"count": 5}, "loud"]
            else:
                cap = [c.get("caption"), {"site": SITE, "pieces": c["pieces"]}, "loud" if c.get("loud") else ""]
            page.evaluate("([k, v, s]) => window.clip.caption(k, v, s)", cap)
            trees = {"seat": c["trees"]} if c.get("half") and "trees" in c else False
            speed = 1 if c.get("half") else TICKS
            zoom = 1.45 if trees else 3.1
            low = c.get("ramp", 0.35)
            ended_at, k = None, 0
            limit = int((c["lead"] + c["slow"]) * FPS * (2 if speed == 1 else 1)) + 90
            while k < limit:
                if ended_at is None:
                    ticks, punch = speed, 0.0
                else:
                    # The kill: ramp into slow motion and punch in, over a
                    # fifth of a second, then hold.
                    r = min(1.0, (k - ended_at) / 6)
                    ticks, punch = speed + (low - speed) * ease(r), ease(r)
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
            if ended_at is None:
                print(f"  {c['left']} v {c['right']}: the cut was NOT reached", flush=True)
        b.close()
        if errors:
            print("page errors:", errors[:3])
    enc.stdin.close()
    enc.wait()
    length = frames / FPS
    wav = out.with_suffix(".hits.wav")
    impact_wav(wav, hits, length)
    if music:
        # The music under the hits, faded in and out, the hits on top.
        mix = (f"[2:a]atrim=0:{length:.2f},afade=t=in:d=0.3,afade=t=out:st={max(0, length - 0.8):.2f}:d=0.8,volume=0.55[m];"
               "[m][1:a]amix=inputs=2:normalize=0[a]")
        cmd = [ff, "-y", "-loglevel", "error", "-i", str(silent), "-i", str(wav), "-stream_loop", "-1", "-i", str(music),
               "-filter_complex", mix, "-map", "0:v", "-map", "[a]", "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-t", f"{length:.2f}", str(out)]
    else:
        cmd = [ff, "-y", "-loglevel", "error", "-i", str(silent), "-i", str(wav), "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest", str(out)]
    subprocess.run(cmd, check=True)
    silent.unlink()
    wav.unlink()
    print(f"wrote {out}: {length:.1f} s, {len(hits)} kills{', with music' if music else ''}")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("variant", choices=sorted(VARIANTS))
    ap.add_argument("--music")
    ap.add_argument("--out")
    a = ap.parse_args()
    out = Path(a.out) if a.out else Path.home() / "Movies" / "Vagrancy" / f"short-{a.variant}.mp4"
    render(a.variant, out, Path(a.music).expanduser() if a.music else None)
