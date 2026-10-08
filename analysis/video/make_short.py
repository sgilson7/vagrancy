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
    VARIANT: one of shorts.json's (loop, tree_first, text_hook, countdown, fast)
"""
import argparse, math, random, struct, subprocess, wave
from pathlib import Path

import imageio_ffmpeg
from playwright.sync_api import sync_playwright

PAGE = "http://127.0.0.1:8766/clip.html"
SITE = "sgilson7.github.io/vagrancy"
FPS = 30
TICKS = 2  # sim ticks per frame at full speed (60 ticks a second)

# The cut lists, from `lab highlights`: one per variant, no clip in two
# (shorts.json says how they were chosen).
import json
VARIANTS = json.loads((Path(__file__).with_name("shorts.json")).read_text())["variants"]


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


def render(variant, out, music=None, music_start=None):
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
            page.evaluate("s => window.clip.load(s)", {"left": c["left"], "right": c["right"], "seed": c["seed"], "from": c["kill"] - int(c["lead"] * 60), "kill": c["kill"]})
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
                    if state["ended"]["tick"] < c["kill"] - 2:
                        print(f"  {c['left']} v {c['right']}: a round ended at {state['ended']['tick']}, before the chosen cut at {c['kill']}", flush=True)
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
        # The song comes in as the first cut lands (Sam: "the song should
        # start on the first cut as it lands"), from its punchiest bar
        # (song_hook.py), and fades out at the end; the hits stay on top.
        if music_start is None:
            import song_hook
            music_start = song_hook.analyse(music)["start"]
        first = hits[0] if hits else 0.0
        body = length - first
        delay = int(first * 1000)
        mix = (f"[2:a]atrim={music_start:.3f}:{music_start + body:.3f},asetpts=PTS-STARTPTS,"
               f"afade=t=out:st={max(0, body - 0.8):.2f}:d=0.8,volume=0.6,adelay={delay}|{delay}[m];"
               "[m][1:a]amix=inputs=2:normalize=0[a]")
        print(f"  music from {music_start:.2f} s of the song, in at {first:.2f} s of the clip")
        cmd = [ff, "-y", "-loglevel", "error", "-i", str(silent), "-i", str(wav), "-i", str(music),
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
    ap.add_argument("--music-start", type=float, help="where in the song to start, in seconds (default: song_hook.py's choice)")
    ap.add_argument("--out")
    a = ap.parse_args()
    out = Path(a.out) if a.out else Path.home() / "Movies" / "Vagrancy" / f"short-{a.variant}.mp4"
    render(a.variant, out, Path(a.music).expanduser() if a.music else None, a.music_start)
