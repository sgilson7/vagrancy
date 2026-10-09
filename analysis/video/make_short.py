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

ROOT = Path(__file__).resolve().parents[2]
PAGE = "http://127.0.0.1:8766/clip.html"
SITE = "sgilson7.github.io/vagrancy"
FPS = 30
TICKS = 2  # sim ticks per frame at full speed (60 ticks a second)

# The cut lists, from `lab highlights`: one per variant, no clip in two
# (shorts.json says how they were chosen).
import json
VARIANTS = json.loads((Path(__file__).with_name("shorts.json")).read_text())["variants"]
# And a local guest clip's own (analysis/video/guests/*.json, not in the repo).
for _g in sorted(Path(__file__).with_name("guests").glob("*.json")):
    VARIANTS.update(json.loads(_g.read_text()).get("variants", {}))


def ease(x):
    return x * x * (3 - 2 * x)


def impact_wav(path, times, length, soft=()):
    """A low thump and a short hiss at each kill, synthesized: a sine that
    falls in pitch from 110 Hz and fades (at 70 Hz, below what a phone's
    speaker plays, it only set off the phone's own limiter, which pulled the
    song down at each kill: Sam, 2026-10-08, "broken audio parts where it
    cuts out"), over a burst of noise; a lighter one, at a
    third of the strength, at each cut of a long fight (`soft`)."""
    rate = 44100
    n = int(length * rate) + rate
    buf = [0.0] * n
    rng = random.Random(7)
    for t0, gain in [(t, 1.0) for t in times] + [(t, 0.33) for t in soft]:
        start = int(t0 * rate)
        for k in range(int(0.45 * rate)):
            if start + k >= n:
                break
            t = k / rate
            buf[start + k] += gain * (math.sin(2 * math.pi * (110 - 50 * t) * t) * math.exp(-t * 9) * 0.9 + (rng.random() * 2 - 1) * math.exp(-t * 40) * 0.35)
    peak = max(1e-9, max(abs(x) for x in buf))
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(b"".join(struct.pack("<h", int(32767 * 0.8 * x / peak)) for x in buf))


def render(variant, out, music=None, music_start=None, gain=0.6):
    clips = VARIANTS[variant]
    out.parent.mkdir(parents=True, exist_ok=True)
    silent = out.with_suffix(".video.mp4")
    ff = imageio_ffmpeg.get_ffmpeg_exe()
    enc = subprocess.Popen([ff, "-y", "-loglevel", "error", "-f", "image2pipe", "-vcodec", "png", "-r", str(FPS), "-i", "-",
                            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "17", "-preset", "slow", str(silent)], stdin=subprocess.PIPE)
    hits, soft, frames = [], [], 0
    # Where each take was left, so a later segment can pick it up again.
    left_at = {}
    with sync_playwright() as p:
        b = p.chromium.launch()
        page = b.new_page(viewport={"width": 1080, "height": 1920})
        errors = []
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.goto(PAGE)
        page.wait_for_function("document.body.dataset.ready === '1'", timeout=60000)
        for c in clips:
            # A guest clip (analysis/video/guests/<name>.json, local only):
            # its own duel, and its pictures copied into the local build for
            # this render alone.
            guest = None
            if c.get("guests"):
                gdir = Path(__file__).with_name("guests")
                g = json.loads((gdir / f"{c['guests']}.json").read_text())
                for sub in ("costume", "scene"):
                    for f in (gdir / "art" / sub).glob("*.png"):
                        dest = ROOT / "dist" / "web" / "art" / sub / f.name
                        dest.parent.mkdir(parents=True, exist_ok=True)
                        dest.write_bytes(f.read_bytes())
                guest = g["page"]
                c = {**c, "duel": {**c["duel"], "weapons": [g["weapons"].get(w) and json.dumps(g["weapons"][w]) or w for w in c["duel"]["weapons"]]}}
            # A take a person played (record_take.py): its replay, cut from a
            # second in to the end of its fight.
            replay = None
            if c.get("replay"):
                replay = list(Path(c["replay"]).expanduser().read_bytes())
                end = page.evaluate("r => window.clip.endOf(r)", replay)
                # `"from": "continue"`: on from where the last segment stopped,
                # in the same take (the vision's snap back to the real fight).
                if c.get("from") == "continue":
                    c = {**c, "from": left_at.get(c["replay"], 60)}
                # A segment of a take: from `from` to `until`, or to the end
                # of its fight.
                stop = c.get("until", end)
                c = {**c, "kill": stop, "lead": (stop - c.get("from", c.get("start", 60))) / 60}
            page.evaluate("s => window.clip.load(s)", {"left": c["left"], "right": c["right"], "seed": c["seed"], "from": c["kill"] - int(c["lead"] * 60), "kill": c["kill"],
                                                      "duel": c.get("duel"), "guests": guest, "replay": replay})
            if c.get("rank"):
                cap = ["clip.rank", {"n": c["rank"]}, "big"]
            elif c.get("count"):
                cap = ["clip.countdown", {"count": 5}, "loud"]
            else:
                cap = [c.get("caption"), {"site": SITE, "pieces": c["pieces"], **c.get("vars", {})}, c.get("size", "loud" if c.get("loud") else "")]
            if c.get("say") is not None:
                page.evaluate("([t, s]) => window.clip.say(t, s)", [c["say"], c.get("size", "")])
            else:
                page.evaluate("([k, v, s]) => window.clip.caption(k, v, s)", cap)
            trees = {"seat": c["trees"]} if c.get("half") and "trees" in c else False
            # Draw once without moving, so the pictures are asked for, and
            # wait for them: the first frame of a clip showed the arena bare.
            page.evaluate("o => window.clip.frame(o)", {"ticks": 0, "zoom": c.get("zoom", 3.1)})
            page.wait_for_function("window.clip.picturesReady()", timeout=20000)
            # Ticks a frame: full speed, half for a tree clip, or the clip's
            # own (an eighth of full speed is 0.25).
            speed = c.get("ticks", 1 if c.get("half") else TICKS)
            zoom = c.get("zoom", 1.45 if trees else 3.1)
            low = c.get("ramp", 0.35)
            ended_at, k, beat = None, 0, -99
            said = set()
            banner_up = False
            limit = c.get("frames") or int((c["lead"] * TICKS / speed + c["slow"]) * FPS) + 90
            est = max(1.0, c["lead"] * 60 / speed)
            while k < limit:
                if ended_at is None:
                    ticks, punch = speed, 0.0
                    # On a long fight, each clash and cut gets a beat: a
                    # third of a second slowed, the camera pushing in a
                    # little and easing back.
                    if c.get("beats") and k - beat < 10:
                        e = 1 - (k - beat) / 10
                        ticks = speed * (1 - 0.6 * e)
                else:
                    # The kill: ramp into slow motion and punch in, over a
                    # fifth of a second, then hold.
                    r = min(1.0, (k - ended_at) / 6)
                    ticks, punch = speed + (low - speed) * ease(r), ease(r)
                    if k - ended_at >= int(c["slow"] * FPS):
                        break
                pulse = 1 + 0.12 * max(0.0, 1 - (k - beat) / 10) if c.get("beats") and ended_at is None else 1
                # A segment's own course: slowing toward `ticks_end`, the
                # camera onto the head (`head` [from, to]), a vision, a flash in.
                prog = min(1.0, k / (c.get("frames") or est))
                if c.get("ticks_end") is not None and ended_at is None:
                    ticks = speed + (c["ticks_end"] - speed) * ease(prog)
                head = c["head"][0] + (c["head"][1] - c["head"][0]) * ease(prog) if c.get("head") else 0
                flash = max(0.0, 1 - k / c["flash_in"]) if c.get("flash_in") else 0
                state = page.evaluate("o => window.clip.frame(o)", {"ticks": ticks, "zoom": zoom * pulse, "punch": punch, "showTrees": trees, "treeScale": c.get("treeScale", 2.6),
                                                                  "head": head, "headZoom": c.get("headZoom", 6), "vision": bool(c.get("vision")), "flash": flash})
                if c.get("replay"):
                    left_at[c["replay"]] = state["tick"]
                # A segment that stops short of its fight's end: cut there.
                if c.get("until") and state["tick"] >= c["until"]:
                    enc.stdin.write(page.locator("#clip-app").screenshot(type="png"))
                    frames += 1
                    break
                # A guest clip's lines at given ticks of the fight (`says`).
                for at, text in c.get("says", []):
                    if state["tick"] >= at and at not in said:
                        said.add(at)
                        page.evaluate("([t, s]) => window.clip.say(t, s)", [text, c.get("size", "")])
                if c.get("beats") and ended_at is None and (state["clashes"] or state["cuts"] or state.get("shields")) and k - beat >= 10:
                    beat = k
                    if state["cuts"]:
                        soft.append(frames / FPS)
                # The site banner, at the kill or `banner_after` seconds
                # after it, so the fall is seen before the link covers it.
                if c.get("end_banner") and state["ended"] and not banner_up:
                    since = 0 if ended_at is None else (k - ended_at) / FPS
                    if since >= c.get("banner_after", 0):
                        banner_up = True
                        page.evaluate("s => window.clip.banner(s)", SITE)
                if ended_at is None and state["ended"] and c.get("caption_end"):
                    page.evaluate("([k, v, s]) => window.clip.caption(k, v, s)", [c["caption_end"], {"site": SITE, "pieces": c["pieces"]}, ""])
                if ended_at is None and state["ended"]:
                    if state["ended"]["tick"] < c["kill"] - 2:
                        print(f"  {c['left']} v {c['right']}: a round ended at {state['ended']['tick']}, before the chosen cut at {c['kill']}", flush=True)
                    ended_at = k
                    hits.append(frames / FPS)
                enc.stdin.write(page.locator("#clip-app").screenshot(type="png"))
                frames += 1
                k += 1
            print(f"  segment done: {k} frames, tick {state["tick"]}, {frames / FPS:.1f}s so far", flush=True)
            page.evaluate("window.clip.ended = null")
            page.evaluate("window.clip.banner()")
            if ended_at is None and not c.get("until"):
                print(f"  {c['left']} v {c['right']}: the cut was NOT reached", flush=True)
        b.close()
        if errors:
            print("page errors:", errors[:3])
    enc.stdin.close()
    enc.wait()
    length = frames / FPS
    wav = out.with_suffix(".hits.wav")
    impact_wav(wav, hits, length, soft)
    if music:
        # The song plays from the first frame (Sam: "the music should start at
        # the very beginning instant of the video"), started early by the
        # time before the first cut, so its punchiest bar (song_hook.py)
        # still lands on the cut; it fades out at the end, the hits on top.
        if music_start is None:
            import song_hook
            music_start = song_hook.analyse(music)["start"]
        first = hits[0] if hits else 0.0
        if all(c.get("beats") for c in clips) and hits:
            # One long fight: its punchiest bar lands on the kill at its end,
            # unless that would start the song before its beginning.
            first = hits[-1] if music_start >= hits[-1] else first
        begin = max(0.0, music_start - first)
        mix = (f"[1:a]volume=0.6[h];[2:a]atrim={begin:.3f}:{begin + length:.3f},asetpts=PTS-STARTPTS,"
               f"afade=t=out:st={max(0, length - 0.8):.2f}:d=0.8,volume={gain}[m];"
               f"[m][h]amix=inputs=2:normalize=0,alimiter=limit=0.89:level=disabled,atrim=0:{length:.2f}[a]")
        # A hit on a loud bar summed past full scale and broke up (Sam,
        # 2026-10-08: "some broken audio parts where it cuts out"): the
        # limiter holds the mix a decibel under it.
        print(f"  music from {begin:.2f} s of the song, its punchiest bar ({music_start:.2f} s) on the first cut at {first:.2f} s")
        cmd = [ff, "-y", "-loglevel", "error", "-i", str(silent), "-i", str(wav), "-i", str(music),
               "-filter_complex", mix, "-map", "0:v", "-map", "[a]", "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", str(out)]
        # (An output -t here left the mixed audio silent with this ffmpeg; the
        # length is trimmed inside the mix instead.)
    else:
        cmd = [ff, "-y", "-loglevel", "error", "-i", str(silent), "-i", str(wav), "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest", str(out)]
    subprocess.run(cmd, check=True)
    level = subprocess.run([ff, "-hide_banner", "-i", str(out), "-vn", "-af", "volumedetect", "-f", "null", "-"], capture_output=True, text=True).stderr
    mean = float(level.split("mean_volume:")[1].split("dB")[0]) if "mean_volume:" in level else -99.0
    if mean < -40:
        raise SystemExit(f"{out}: the audio is silent (mean {mean} dB)")
    peak = float(level.split("max_volume:")[1].split("dB")[0]) if "max_volume:" in level else 0.0
    if peak > -0.1:
        raise SystemExit(f"{out}: the audio clips (peak {peak} dB)")
    silent.unlink()
    wav.unlink()
    print(f"wrote {out}: {length:.1f} s, {len(hits)} kills{', with music' if music else ''}, audio mean {mean:.1f} dB, peak {peak:.1f} dB")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("variant", choices=sorted(VARIANTS))
    ap.add_argument("--music")
    ap.add_argument("--music-start", type=float, help="where in the song to start, in seconds (default: song_hook.py's choice)")
    ap.add_argument("--gain", type=float, default=0.6, help="the song's level under the hits (a quiet track wants more)")
    ap.add_argument("--out")
    a = ap.parse_args()
    out = Path(a.out) if a.out else Path.home() / "Movies" / "Vagrancy" / f"short-{a.variant}.mp4"
    render(a.variant, out, Path(a.music).expanduser() if a.music else None, a.music_start, a.gain)
