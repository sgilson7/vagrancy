#!/usr/bin/env python3
"""A small scene for each fight on arcade mode's chart (Sam, 2026-10-07: "the
bubbles when locked and unlocked should pop out a lot more / be integrated
with the background art somehow, like the drover is next to some cows, or
the kite maker bubble is a kite, or the ring keeper bubble is in a zoo, the
harvest keeper bubble is at a silo").

Each fight is a little island of its region's ground with a few props from
its place line (copy `opponents.<id>.place`), and its seal, the part the
chart marks open, won or locked, shaped like the thing the villager works
with where there is one: a kite, a bell, a wheel, a coin. Same style and
colors as the region art (regions.py, palette `art`). The page greys a
locked scene and lights an open one; nothing here changes with progress.

    python3 analysis/art/seals.py        # writes web/art/seal/<id>.png
    python3 analysis/art/seals.py drover # just one
"""
import json, math, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PAL = json.loads((ROOT / "data" / "palette.json").read_text())["art"]
OUT = ROOT / "web" / "art" / "seal"
W, H = 4.0, 3.0           # cm
CX, CY, R = 2.0, 1.2, 0.46  # the seal, low and in front, so what stands behind it shows
PX = 320                    # pixels across, drawn at twice the size shown

LINE = "draw=ink, line width=0.7pt, line join=round, line cap=round"
THIN = "draw=ink, line width=0.45pt, line cap=round"


def colors():
    return "\n".join(f"\\definecolor{{{k.replace('_','')}}}{{HTML}}{{{v[1:]}}}" for k, v in PAL.items() if not k.startswith("_"))


def f(v):
    return f"{v:.3f}"


def P(x, y):
    return f"({f(x)},{f(y)})"


def poly(pts, fill, style=LINE):
    return f"\\filldraw[fill={fill}, {style}] " + " -- ".join(P(x, y) for x, y in pts) + " -- cycle;"


def rect(x, y, w, h, fill, style=LINE):
    return f"\\filldraw[fill={fill}, {style}] {P(x, y)} rectangle {P(x + w, y + h)};"


def circ(x, y, r, fill, style=LINE):
    return f"\\filldraw[fill={fill}, {style}] {P(x, y)} circle ({f(r)});"


def ell(x, y, rx, ry, fill, style=LINE):
    return f"\\filldraw[fill={fill}, {style}] {P(x, y)} ellipse ({f(rx)} and {f(ry)});"


def line(*pts, style=THIN):
    return f"\\draw[{style}] " + " -- ".join(P(x, y) for x, y in pts) + ";"


# --- props: each stands on (x, y), about s centimeters tall -----------------

def cow(x, y, s, flip=False, body="cream"):
    d = -1 if flip else 1
    o = [line((x - 0.3 * s, y), (x - 0.3 * s, y + 0.3 * s)), line((x + 0.25 * s, y), (x + 0.25 * s, y + 0.3 * s))]
    o.append(ell(x, y + 0.42 * s, 0.42 * s, 0.2 * s, body))
    o.append(circ(x - 0.1 * s, y + 0.45 * s, 0.07 * s, "shadow", THIN))
    o.append(ell(x + d * 0.47 * s, y + 0.55 * s, 0.13 * s, 0.1 * s, body))
    o.append(line((x + d * 0.5 * s, y + 0.64 * s), (x + d * 0.58 * s, y + 0.74 * s)))
    return o


def sheep(x, y, s, flip=False):
    d = -1 if flip else 1
    o = [line((x - 0.18 * s, y), (x - 0.18 * s, y + 0.25 * s)), line((x + 0.18 * s, y), (x + 0.18 * s, y + 0.25 * s))]
    o.append(f"\\filldraw[fill=cloud, {LINE}] " + " ".join(f"{P(x + dx * s, y + 0.4 * s + dy * s)} circle ({f(0.14 * s)})" for dx, dy in [(-0.18, 0), (0, 0.06), (0.18, 0), (0, -0.04)]) + ";")
    o.append(ell(x + d * 0.34 * s, y + 0.45 * s, 0.09 * s, 0.07 * s, "shadow"))
    return o


def ox(x, y, s, flip=False):
    o = cow(x, y, s, flip, body="wood")
    d = -1 if flip else 1
    o.append(f"\\draw[{THIN}] {P(x + d * 0.42 * s, y + 0.63 * s)} .. controls {P(x + d * 0.38 * s, y + 0.75 * s)} .. {P(x + d * 0.46 * s, y + 0.78 * s)};")
    return o


def hare(x, y, s, flip=False):
    d = -1 if flip else 1
    return [ell(x, y + 0.14 * s, 0.2 * s, 0.13 * s, "stone"), circ(x + d * 0.2 * s, y + 0.26 * s, 0.08 * s, "stone"),
            ell(x + d * 0.2 * s, y + 0.44 * s, 0.03 * s, 0.13 * s, "stone")]


def bird(x, y, s, fill="shadow", flip=False):
    d = -1 if flip else 1
    return [ell(x, y + 0.12 * s, 0.14 * s, 0.09 * s, fill), circ(x + d * 0.13 * s, y + 0.2 * s, 0.06 * s, fill),
            poly([(x + d * 0.18 * s, y + 0.21 * s), (x + d * 0.26 * s, y + 0.19 * s), (x + d * 0.18 * s, y + 0.17 * s)], "gold", THIN),
            line((x, y), (x, y + 0.05 * s))]


def flying(x, y, s):
    return [f"\\draw[{THIN}] {P(x - 0.15 * s, y + 0.05 * s)} .. controls {P(x - 0.07 * s, y + 0.1 * s)} .. {P(x, y)} .. controls {P(x + 0.07 * s, y + 0.1 * s)} .. {P(x + 0.15 * s, y + 0.05 * s)};"]


def fish(x, y, s, flip=False):
    d = -1 if flip else 1
    return [ell(x, y, 0.22 * s, 0.07 * s, "teal"), poly([(x - d * 0.2 * s, y), (x - d * 0.32 * s, y + 0.08 * s), (x - d * 0.32 * s, y - 0.08 * s)], "teal")]


def waves(x, y, s, n=3):
    return [f"\\draw[tealdark, line width=0.6pt] {P(x - 0.45 * s + i * 0.3 * s, y + 0.04 * s)} .. controls {P(x - 0.38 * s + i * 0.3 * s, y + 0.11 * s)} .. {P(x - 0.3 * s + i * 0.3 * s, y + 0.04 * s)};" for i in range(n)]


def water(x, y, w, h=0.16):
    return [ell(x, y + h / 2, w / 2, h, "teal"), *waves(x, y + 0.02, 0.8, 3)]


def boat(x, y, s, flip=False, sail=True):
    o = [poly([(x - 0.42 * s, y + 0.22 * s), (x + 0.42 * s, y + 0.22 * s), (x + 0.3 * s, y), (x - 0.3 * s, y)], "wood")]
    if sail:
        o += [line((x, y + 0.22 * s), (x, y + 0.85 * s)), poly([(x + 0.02 * s, y + 0.82 * s), (x + 0.3 * s, y + 0.3 * s), (x + 0.02 * s, y + 0.3 * s)], "cream")]
    return o


def barrel(x, y, s, fill="wood"):
    w, h = 0.24 * s, 0.4 * s
    return [f"\\filldraw[fill={fill}, {LINE}] {P(x - w, y)} -- {P(x + w, y)} .. controls {P(x + w * 1.25, y + h / 2)} .. {P(x + w, y + h)} -- {P(x - w, y + h)} .. controls {P(x - w * 1.25, y + h / 2)} .. cycle;",
            line((x - w * 1.08, y + h * 0.3), (x + w * 1.08, y + h * 0.3)), line((x - w * 1.08, y + h * 0.7), (x + w * 1.08, y + h * 0.7))]


def barrels(x, y, s):
    return barrel(x - 0.18 * s, y, 0.8 * s) + barrel(x + 0.2 * s, y, 0.8 * s) + barrel(x, y + 0.32 * s, 0.8 * s)


def crate(x, y, s, fill="wood"):
    w = 0.32 * s
    return [rect(x - w / 2, y, w, w, fill), line((x - w / 2, y), (x + w / 2, y + w)), line((x - w / 2, y + w), (x + w / 2, y))]


def crates(x, y, s):
    return crate(x - 0.17 * s, y, s) + crate(x + 0.17 * s, y, s) + crate(x, y + 0.32 * s, s)


def sacks(x, y, s):
    o = crate(x, y, s)
    for dx in (-0.12, 0.12):
        o.append(ell(x + dx * s, y + 0.42 * s, 0.13 * s, 0.1 * s, "cloud"))
    return o


def windmill(x, y, s):
    o = [poly([(x - 0.2 * s, y), (x + 0.2 * s, y), (x + 0.1 * s, y + 0.8 * s), (x - 0.1 * s, y + 0.8 * s)], "cream")]
    cx, cy = x, y + 0.8 * s
    for a in (20, 110, 200, 290):
        r = math.radians(a)
        ex, ey = cx + math.cos(r) * 0.55 * s, cy + math.sin(r) * 0.55 * s
        nx, ny = -math.sin(r) * 0.07 * s, math.cos(r) * 0.07 * s
        o.append(poly([(cx, cy), (ex, ey), (ex + nx * 2, ey + ny * 2), (cx + nx, cy + ny)], "peach", THIN))
    o.append(circ(cx, cy, 0.05 * s, "ink", THIN))
    return o


def poles_rope(x, y, s, w=1.0):
    return [line((x - w / 2, y), (x - w / 2, y + 0.9 * s), style=LINE), line((x + w / 2, y), (x + w / 2, y + 0.9 * s), style=LINE),
            f"\\draw[{THIN}] {P(x - w / 2, y + 0.85 * s)} .. controls {P(x, y + 0.75 * s)} .. {P(x + w / 2, y + 0.85 * s)};"]


def rope_coil(x, y, s):
    return [ell(x, y + 0.08 * s, 0.22 * s, 0.08 * s, "wood"), ell(x, y + 0.14 * s, 0.17 * s, 0.06 * s, "wood"), ell(x, y + 0.19 * s, 0.11 * s, 0.04 * s, "wood")]


def anvil(x, y, s):
    return [poly([(x - 0.1 * s, y), (x + 0.1 * s, y), (x + 0.08 * s, y + 0.18 * s), (x + 0.3 * s, y + 0.26 * s), (x + 0.3 * s, y + 0.32 * s),
                  (x - 0.22 * s, y + 0.32 * s), (x - 0.32 * s, y + 0.24 * s), (x - 0.08 * s, y + 0.18 * s)], "slate"),
            line((x + 0.05 * s, y + 0.33 * s), (x + 0.28 * s, y + 0.6 * s), style=LINE), rect(x + 0.2 * s, y + 0.55 * s, 0.16 * s, 0.09 * s, "shadow", THIN)]


def fire(x, y, s):
    return [poly([(x - 0.15 * s, y), (x + 0.15 * s, y), (x + 0.08 * s, y + 0.2 * s), (x + 0.02 * s, y + 0.38 * s), (x - 0.06 * s, y + 0.22 * s)], "gold"),
            poly([(x - 0.06 * s, y), (x + 0.06 * s, y), (x, y + 0.18 * s)], "peach", THIN)]


def bell_prop(x, y, s, fill="gold"):
    return [f"\\filldraw[fill={fill}, {LINE}] {P(x - 0.22 * s, y)} -- {P(x + 0.22 * s, y)} .. controls {P(x + 0.14 * s, y + 0.1 * s)} and {P(x + 0.16 * s, y + 0.38 * s)} .. {P(x, y + 0.42 * s)} .. controls {P(x - 0.16 * s, y + 0.38 * s)} and {P(x - 0.14 * s, y + 0.1 * s)} .. cycle;"]


def tower(x, y, s, fill="cream", roof="peachdark"):
    return [rect(x - 0.2 * s, y, 0.4 * s, 0.9 * s, fill), poly([(x - 0.3 * s, y + 0.9 * s), (x, y + 1.2 * s), (x + 0.3 * s, y + 0.9 * s)], roof),
            rect(x - 0.08 * s, y + 0.55 * s, 0.16 * s, 0.2 * s, "shadow", THIN)]


def gate(x, y, s, fill="wood"):
    return [rect(x - 0.4 * s, y, 0.08 * s, 0.7 * s, fill), rect(x + 0.32 * s, y, 0.08 * s, 0.7 * s, fill),
            rect(x - 0.48 * s, y + 0.62 * s, 0.96 * s, 0.08 * s, "peachdark"), line((x - 0.32 * s, y + 0.35 * s), (x + 0.32 * s, y + 0.35 * s), style=LINE)]


def torii(x, y, s):
    return [rect(x - 0.36 * s, y, 0.07 * s, 0.75 * s, "peachdark"), rect(x + 0.29 * s, y, 0.07 * s, 0.75 * s, "peachdark"),
            poly([(x - 0.5 * s, y + 0.78 * s), (x + 0.5 * s, y + 0.78 * s), (x + 0.45 * s, y + 0.86 * s), (x - 0.45 * s, y + 0.86 * s)], "peachdark"),
            rect(x - 0.42 * s, y + 0.6 * s, 0.84 * s, 0.06 * s, "peachdark", THIN)]


def signboard(x, y, s, fill="cream"):
    o = [line((x, y), (x, y + 0.45 * s), style=LINE), rect(x - 0.22 * s, y + 0.4 * s, 0.44 * s, 0.3 * s, fill)]
    for k in range(3):
        o.append(line((x - 0.15 * s, y + 0.48 * s + k * 0.07 * s), (x + 0.15 * s, y + 0.48 * s + k * 0.07 * s)))
    return o


def books(x, y, s):
    o, yy = [], y
    for w, c in [(0.5, "tealdark"), (0.44, "peachdark"), (0.48, "olive"), (0.4, "slate")]:
        o.append(rect(x - w * s / 2, yy, w * s, 0.09 * s, c, THIN))
        yy += 0.09 * s
    return o


def shelf(x, y, s):
    o = [rect(x - 0.35 * s, y, 0.7 * s, 0.85 * s, "wood")]
    for k in range(3):
        o.append(rect(x - 0.3 * s, y + 0.06 * s + k * 0.27 * s, 0.6 * s, 0.2 * s, "cream", THIN))
        for j in range(5):
            o.append(line((x - 0.25 * s + j * 0.11 * s, y + 0.07 * s + k * 0.27 * s), (x - 0.25 * s + j * 0.11 * s, y + 0.24 * s + k * 0.27 * s)))
    return o


def cloths(x, y, s, cols=("tealdark", "teal", "gold")):
    o = [line((x - 0.5 * s, y), (x - 0.5 * s, y + 0.85 * s), style=LINE), line((x + 0.5 * s, y), (x + 0.5 * s, y + 0.85 * s), style=LINE),
         line((x - 0.5 * s, y + 0.8 * s), (x + 0.5 * s, y + 0.8 * s))]
    for i, c in enumerate(cols):
        o.append(rect(x - 0.42 * s + i * 0.3 * s, y + 0.35 * s, 0.24 * s, 0.45 * s, c, THIN))
    return o


def pot(x, y, s, fill="peachdark"):
    return [f"\\filldraw[fill={fill}, {LINE}] {P(x - 0.1 * s, y)} -- {P(x + 0.1 * s, y)} .. controls {P(x + 0.3 * s, y + 0.2 * s)} .. {P(x + 0.08 * s, y + 0.38 * s)} -- {P(x + 0.1 * s, y + 0.44 * s)} -- {P(x - 0.1 * s, y + 0.44 * s)} -- {P(x - 0.08 * s, y + 0.38 * s)} .. controls {P(x - 0.3 * s, y + 0.2 * s)} .. cycle;"]


def pots(x, y, s):
    return pot(x - 0.25 * s, y, 0.8 * s) + pot(x + 0.05 * s, y, s, "peach") + pot(x + 0.32 * s, y, 0.7 * s)


def kiln(x, y, s):
    return [f"\\filldraw[fill=stone, {LINE}] {P(x - 0.4 * s, y)} -- {P(x + 0.4 * s, y)} .. controls {P(x + 0.4 * s, y + 0.6 * s)} and {P(x - 0.4 * s, y + 0.6 * s)} .. cycle;",
            rect(x - 0.1 * s, y, 0.2 * s, 0.2 * s, "shadow", THIN), *smoke(x + 0.1 * s, y + 0.5 * s, s * 0.8)]


def smoke(x, y, s):
    return [circ(x + dx * s, y + dy * s, r * s, "cloud", THIN) for dx, dy, r in [(0, 0.08, 0.07), (0.07, 0.22, 0.09), (0.02, 0.4, 0.11)]]


def mound(x, y, s):
    return [f"\\filldraw[fill=shadow, {LINE}] {P(x - 0.42 * s, y)} .. controls {P(x - 0.3 * s, y + 0.42 * s)} and {P(x + 0.3 * s, y + 0.42 * s)} .. {P(x + 0.42 * s, y)} -- cycle;", *smoke(x, y + 0.3 * s, s)]


def house(x, y, s, wall="peach", roof="peachdark", beams=False):
    o = [rect(x - 0.4 * s, y, 0.8 * s, 0.5 * s, wall if not beams else "skylight")]
    if beams:
        o += [line((x - 0.5 * s, y + 0.5 * s), (x, y + 0.85 * s), (x + 0.5 * s, y + 0.5 * s), style=LINE)]
        o += [line((x - 0.25 * s, y + 0.5 * s), (x - 0.25 * s, y + 0.68 * s)), line((x + 0.25 * s, y + 0.5 * s), (x + 0.25 * s, y + 0.68 * s)), line((x, y + 0.5 * s), (x, y + 0.85 * s))]
        o += [line((x - 0.4 * s, y), (x - 0.4 * s, y + 0.5 * s), style=LINE), line((x + 0.4 * s, y), (x + 0.4 * s, y + 0.5 * s), style=LINE)]
        return o
    o.append(poly([(x - 0.5 * s, y + 0.5 * s), (x, y + 0.85 * s), (x + 0.5 * s, y + 0.5 * s)], roof))
    o.append(rect(x - 0.08 * s, y, 0.16 * s, 0.28 * s, "wood", THIN))
    return o


def roofs(x, y, s):
    o = []
    for dx, h, c in [(-0.35, 0.3, "peach"), (0.1, 0.5, "cream"), (0.45, 0.2, "peach")]:
        o += [rect(x + dx * s - 0.22 * s, y, 0.44 * s, h * s, c), poly([(x + dx * s - 0.28 * s, y + h * s), (x + dx * s, y + h * s + 0.22 * s), (x + dx * s + 0.28 * s, y + h * s)], "slate")]
    return o


def wheat(x, y, s):
    o = []
    for dx in (-0.08, 0, 0.08):
        o.append(line((x + dx * s * 0.5, y), (x + dx * s, y + 0.6 * s), style="draw=olive, line width=0.8pt"))
        o.append(ell(x + dx * s, y + 0.66 * s, 0.035 * s, 0.09 * s, "gold", THIN))
    return o


def sheaf(x, y, s):
    return [poly([(x - 0.18 * s, y), (x + 0.18 * s, y), (x + 0.06 * s, y + 0.3 * s), (x + 0.2 * s, y + 0.7 * s), (x - 0.2 * s, y + 0.7 * s), (x - 0.06 * s, y + 0.3 * s)], "gold"),
            rect(x - 0.08 * s, y + 0.27 * s, 0.16 * s, 0.06 * s, "wood", THIN)]


def rice(x, y, s, n=3):
    o = []
    for i in range(n):
        xx = x + (i - (n - 1) / 2) * 0.18 * s
        o += [line((xx, y), (xx - 0.07 * s, y + 0.25 * s), style="draw=sagedark, line width=0.7pt"), line((xx, y), (xx, y + 0.3 * s), style="draw=sagedark, line width=0.7pt"),
              line((xx, y), (xx + 0.07 * s, y + 0.25 * s), style="draw=sagedark, line width=0.7pt")]
    return o


def silo(x, y, s):
    return [rect(x - 0.22 * s, y, 0.44 * s, 0.95 * s, "cream"), f"\\filldraw[fill=peachdark, {LINE}] {P(x - 0.22 * s, y + 0.95 * s)} arc (180:0:{f(0.22 * s)}) -- cycle;",
            line((x - 0.22 * s, y + 0.32 * s), (x + 0.22 * s, y + 0.32 * s)), line((x - 0.22 * s, y + 0.64 * s), (x + 0.22 * s, y + 0.64 * s))]


def jar(x, y, s):
    return [f"\\filldraw[fill=peach, {LINE}] {P(x - 0.1 * s, y)} -- {P(x + 0.1 * s, y)} .. controls {P(x + 0.22 * s, y + 0.3 * s)} .. {P(x + 0.08 * s, y + 0.5 * s)} -- {P(x - 0.08 * s, y + 0.5 * s)} .. controls {P(x - 0.22 * s, y + 0.3 * s)} .. cycle;"]


def tree(x, y, s, fruit=False, pine=False):
    if pine:
        return [rect(x - 0.04 * s, y, 0.08 * s, 0.2 * s, "wood", THIN), poly([(x - 0.3 * s, y + 0.15 * s), (x + 0.3 * s, y + 0.15 * s), (x, y + 0.95 * s)], "sagedark")]
    o = [rect(x - 0.05 * s, y, 0.1 * s, 0.4 * s, "wood", THIN), circ(x, y + 0.6 * s, 0.3 * s, "sage")]
    if fruit:
        o += [circ(x + dx * s, y + dy * s, 0.045 * s, "gold", THIN) for dx, dy in [(-0.12, 0.55), (0.1, 0.7), (0.15, 0.5), (-0.05, 0.78)]]
    return o


def bush(x, y, s, fill="sagedark"):
    return [f"\\filldraw[fill={fill}, {LINE}] {P(x - 0.3 * s, y)} .. controls {P(x - 0.3 * s, y + 0.32 * s)} and {P(x + 0.3 * s, y + 0.32 * s)} .. {P(x + 0.3 * s, y)} -- cycle;"]


def well(x, y, s):
    return [ell(x, y + 0.2 * s, 0.25 * s, 0.08 * s, "shadow"), rect(x - 0.25 * s, y, 0.5 * s, 0.2 * s, "stone"), line((x - 0.22 * s, y + 0.2 * s), (x - 0.22 * s, y + 0.7 * s), style=LINE),
            line((x + 0.22 * s, y + 0.2 * s), (x + 0.22 * s, y + 0.7 * s), style=LINE), line((x - 0.28 * s, y + 0.7 * s), (x + 0.28 * s, y + 0.7 * s), style=LINE),
            line((x, y + 0.7 * s), (x, y + 0.42 * s)), rect(x - 0.05 * s, y + 0.32 * s, 0.1 * s, 0.1 * s, "wood", THIN)]


def kite(x, y, s, fill="peach"):
    return [poly([(x, y + 0.6 * s), (x + 0.22 * s, y + 0.35 * s), (x, y), (x - 0.22 * s, y + 0.35 * s)], fill), line((x, y), (x, y + 0.6 * s)),
            line((x - 0.22 * s, y + 0.35 * s), (x + 0.22 * s, y + 0.35 * s)),
            f"\\draw[{THIN}] {P(x, y)} .. controls {P(x - 0.1 * s, y - 0.15 * s)} and {P(x + 0.1 * s, y - 0.25 * s)} .. {P(x, y - 0.4 * s)};"]


def bridge(x, y, s):
    return [f"\\filldraw[fill=wood, {LINE}] {P(x - 0.6 * s, y)} .. controls {P(x - 0.3 * s, y + 0.45 * s)} and {P(x + 0.3 * s, y + 0.45 * s)} .. {P(x + 0.6 * s, y)} -- {P(x + 0.5 * s, y)} .. controls {P(x + 0.25 * s, y + 0.32 * s)} and {P(x - 0.25 * s, y + 0.32 * s)} .. {P(x - 0.5 * s, y)} -- cycle;",
            *[line((x + dx * s, y + 0.22 * s + 0.12 * s * math.cos(dx * 2.6)), (x + dx * s, y + 0.5 * s + 0.12 * s * math.cos(dx * 2.6))) for dx in (-0.4, -0.2, 0, 0.2, 0.4)]]


def net(x, y, s):
    o = [line((x - 0.4 * s, y), (x - 0.4 * s, y + 0.8 * s), style=LINE), line((x + 0.4 * s, y), (x + 0.4 * s, y + 0.8 * s), style=LINE)]
    for k in range(5):
        o.append(line((x - 0.4 * s, y + 0.75 * s - k * 0.12 * s), (x + 0.4 * s, y + 0.75 * s - k * 0.12 * s)))
        o.append(line((x - 0.32 * s + k * 0.16 * s, y + 0.75 * s), (x - 0.32 * s + k * 0.16 * s, y + 0.27 * s)))
    return o


def blocks(x, y, s):
    return [rect(x - 0.35 * s, y, 0.34 * s, 0.26 * s, "stone"), rect(x + 0.01 * s, y, 0.34 * s, 0.26 * s, "stone"), rect(x - 0.17 * s, y + 0.26 * s, 0.34 * s, 0.26 * s, "stone")]


def crane(x, y, s):
    return [line((x, y), (x, y + 1.0 * s), style=LINE), line((x - 0.15 * s, y + 0.95 * s), (x + 0.55 * s, y + 0.95 * s), style=LINE),
            line((x, y + 0.6 * s), (x + 0.3 * s, y + 0.95 * s)), line((x + 0.5 * s, y + 0.95 * s), (x + 0.5 * s, y + 0.55 * s)), rect(x + 0.4 * s, y + 0.38 * s, 0.2 * s, 0.17 * s, "stone", THIN)]


def wall(x, y, s, w=0.9):
    o = [rect(x - w * s / 2, y, w * s, 0.35 * s, "stone")]
    for k in range(int(w / 0.18)):
        o.append(line((x - w * s / 2 + k * 0.18 * s + 0.09 * s * (k % 2), y + 0.17 * s), (x - w * s / 2 + k * 0.18 * s + 0.09 * s * (k % 2), y + 0.35 * s)))
    o.append(line((x - w * s / 2, y + 0.17 * s), (x + w * s / 2, y + 0.17 * s)))
    return o


def balls(x, y, s):
    o = [f"\\draw[{THIN}, dashed] {P(x - 0.3 * s, y + 0.2 * s)} .. controls {P(x - 0.2 * s, y + 0.85 * s)} and {P(x + 0.2 * s, y + 0.85 * s)} .. {P(x + 0.3 * s, y + 0.2 * s)};"]
    o += [circ(x + dx * s, y + dy * s, 0.07 * s, c) for dx, dy, c in [(-0.22, 0.5, "teal"), (0, 0.66, "gold"), (0.22, 0.5, "peach")]]
    return o


def logs(x, y, s):
    o = [ell(x + dx * s, y + dy * s, 0.09 * s, 0.09 * s, "wood") for dx, dy in [(-0.18, 0.09), (0, 0.09), (0.18, 0.09), (-0.09, 0.26), (0.09, 0.26)]]
    o += [circ(x + dx * s, y + dy * s, 0.03 * s, "cream", THIN) for dx, dy in [(-0.18, 0.09), (0, 0.09), (0.18, 0.09), (-0.09, 0.26), (0.09, 0.26)]]
    return o


def drum(x, y, s, fill="peach"):
    return [rect(x - 0.25 * s, y, 0.5 * s, 0.4 * s, fill), ell(x, y + 0.4 * s, 0.25 * s, 0.07 * s, "cream"),
            line((x - 0.25 * s, y + 0.05 * s), (x - 0.08 * s, y + 0.35 * s), (x + 0.08 * s, y + 0.05 * s), (x + 0.25 * s, y + 0.35 * s))]


def hide_frame(x, y, s):
    return [rect(x - 0.35 * s, y, 0.06 * s, 0.8 * s, "wood"), rect(x + 0.29 * s, y, 0.06 * s, 0.8 * s, "wood"),
            f"\\filldraw[fill=cream, {LINE}] {P(x - 0.28 * s, y + 0.75 * s)} .. controls {P(x, y + 0.7 * s)} .. {P(x + 0.28 * s, y + 0.75 * s)} -- {P(x + 0.25 * s, y + 0.2 * s)} .. controls {P(x, y + 0.26 * s)} .. {P(x - 0.25 * s, y + 0.2 * s)} -- cycle;"]


def flags(x, y, s):
    o = [line((x - 0.6 * s, y), (x - 0.6 * s, y + 0.8 * s), style=LINE), line((x + 0.6 * s, y), (x + 0.6 * s, y + 0.7 * s), style=LINE),
         f"\\draw[{THIN}] {P(x - 0.6 * s, y + 0.78 * s)} .. controls {P(x, y + 0.55 * s)} .. {P(x + 0.6 * s, y + 0.68 * s)};"]
    for i, c in enumerate(["teal", "gold", "peach", "cream", "sage"]):
        t = (i + 0.5) / 5
        px = x - 0.6 * s + 1.2 * s * t
        py = y + 0.78 * s - 0.2 * s * math.sin(t * math.pi) - 0.1 * s * t
        o.append(rect(px - 0.07 * s, py - 0.18 * s, 0.14 * s, 0.17 * s, c, THIN))
    return o


def flag(x, y, s, fill="teal"):
    return [line((x, y), (x, y + 0.9 * s), style=LINE), poly([(x, y + 0.9 * s), (x + 0.35 * s, y + 0.8 * s), (x, y + 0.68 * s)], fill)]


def hoop(x, y, s):
    return [circ(x, y + 0.3 * s, 0.3 * s, "none", "draw=ink, line width=1.4pt"), line((x + 0.35 * s, y + 0.5 * s), (x + 0.1 * s, y + 0.3 * s), style=LINE)]


def coins(x, y, s):
    o = []
    for k in range(4):
        o.append(ell(x - 0.15 * s, y + 0.04 * s + k * 0.07 * s, 0.15 * s, 0.05 * s, "gold", THIN))
    for k in range(2):
        o.append(ell(x + 0.2 * s, y + 0.04 * s + k * 0.07 * s, 0.15 * s, 0.05 * s, "gold", THIN))
    return o


def bangles(x, y, s):
    o = [line((x, y), (x, y + 0.8 * s), style=LINE), line((x - 0.2 * s, y + 0.75 * s), (x + 0.2 * s, y + 0.75 * s), style=LINE)]
    for k, c in enumerate(["gold", "teal", "peach"]):
        o.append(ell(x - 0.15 * s + k * 0.15 * s, y + 0.6 * s, 0.07 * s, 0.11 * s, "none", f"draw={c}, line width=1.2pt"))
    return o


def wheel(x, y, s, fill="wood"):
    r = 0.3 * s
    o = [circ(x, y + r, r, "none", f"draw=ink, line width=1.6pt")]
    for a in range(0, 180, 30):
        ra = math.radians(a)
        o.append(line((x - math.cos(ra) * r, y + r - math.sin(ra) * r), (x + math.cos(ra) * r, y + r + math.sin(ra) * r)))
    o.append(circ(x, y + r, 0.05 * s, fill, THIN))
    return o


def quoits(x, y, s):
    return [line((x, y), (x, y + 0.4 * s), style=LINE), ell(x, y + 0.08 * s, 0.14 * s, 0.05 * s, "none", "draw=gold, line width=1.2pt"),
            ell(x, y + 0.16 * s, 0.14 * s, 0.05 * s, "none", "draw=teal, line width=1.2pt")]


def cage(x, y, s):
    o = [rect(x - 0.45 * s, y, 0.9 * s, 0.75 * s, "cloud"), poly([(x - 0.5 * s, y + 0.75 * s), (x, y + 0.95 * s), (x + 0.5 * s, y + 0.75 * s)], "peachdark")]
    o += [ell(x, y + 0.25 * s, 0.25 * s, 0.17 * s, "gold"), circ(x + 0.2 * s, y + 0.42 * s, 0.11 * s, "gold"),
          circ(x + 0.14 * s, y + 0.52 * s, 0.04 * s, "gold", THIN), circ(x + 0.27 * s, y + 0.52 * s, 0.04 * s, "gold", THIN),
          line((x - 0.1 * s, y + 0.2 * s), (x - 0.05 * s, y + 0.35 * s)), line((x + 0.02 * s, y + 0.18 * s), (x + 0.06 * s, y + 0.34 * s))]
    for k in range(7):
        o.append(line((x - 0.39 * s + k * 0.13 * s, y), (x - 0.39 * s + k * 0.13 * s, y + 0.75 * s), style="draw=ink, line width=0.8pt"))
    return o


def cage_bars(x, y, s):
    o = [poly([(x - 0.42 * s, y + 0.7 * s), (x, y + 0.9 * s), (x + 0.42 * s, y + 0.7 * s)], "peachdark"), rect(x - 0.4 * s, y, 0.8 * s, 0.05 * s, "wood", THIN)]
    for k in range(6):
        o.append(line((x - 0.34 * s + k * 0.136 * s, y), (x - 0.34 * s + k * 0.136 * s, y + 0.7 * s), style="draw=ink, line width=0.9pt"))
    return o


def scythe_prop(x, y, s):
    return [line((x, y), (x + 0.15 * s, y + 0.9 * s), style="draw=wood, line width=1.4pt"),
            f"\\filldraw[fill=slate, {THIN}] {P(x + 0.15 * s, y + 0.9 * s)} .. controls {P(x - 0.1 * s, y + 0.95 * s)} .. {P(x - 0.35 * s, y + 0.78 * s)} .. controls {P(x - 0.1 * s, y + 0.86 * s)} .. {P(x + 0.13 * s, y + 0.82 * s)} -- cycle;"]


def grave(x, y, s):
    return [f"\\filldraw[fill=stone, {LINE}] {P(x - 0.15 * s, y)} -- {P(x - 0.15 * s, y + 0.35 * s)} arc (180:0:{f(0.15 * s)}) -- {P(x + 0.15 * s, y)} -- cycle;",
            line((x - 0.07 * s, y + 0.3 * s), (x + 0.07 * s, y + 0.3 * s))]


def ladder(x, y, s):
    o = [line((x - 0.1 * s, y), (x + 0.05 * s, y + 0.8 * s), style=LINE), line((x + 0.1 * s, y), (x + 0.25 * s, y + 0.8 * s), style=LINE)]
    for k in range(5):
        t = (k + 0.5) / 5
        o.append(line((x - 0.1 * s + 0.15 * s * t, y + 0.8 * s * t), (x + 0.1 * s + 0.15 * s * t, y + 0.8 * s * t)))
    return o


def stilt_hut(x, y, s):
    return [line((x - 0.25 * s, y), (x - 0.25 * s, y + 0.5 * s), style=LINE), line((x + 0.25 * s, y), (x + 0.25 * s, y + 0.5 * s), style=LINE),
            rect(x - 0.3 * s, y + 0.5 * s, 0.6 * s, 0.32 * s, "wood"), poly([(x - 0.4 * s, y + 0.82 * s), (x, y + 1.05 * s), (x + 0.4 * s, y + 0.82 * s)], "olive")]


def lanterns(x, y, s, n=3):
    o = [line((x - 0.5 * s, y + 0.85 * s), (x + 0.5 * s, y + 0.85 * s))]
    for i in range(n):
        xx = x - 0.4 * s + i * 0.8 * s / max(1, n - 1)
        o += [line((xx, y + 0.85 * s), (xx, y + 0.78 * s)), ell(xx, y + 0.66 * s, 0.09 * s, 0.12 * s, "gold" if i % 2 else "peach", THIN)]
    return o


def lamp_post(x, y, s):
    return [line((x, y), (x, y + 0.85 * s), style="draw=ink, line width=1.2pt"), line((x, y + 0.85 * s), (x + 0.2 * s, y + 0.85 * s), style=LINE),
            rect(x + 0.12 * s, y + 0.62 * s, 0.16 * s, 0.2 * s, "gold", THIN)]


def lion(x, y, s):
    o = [circ(x + dx * s, y + 0.45 * s + dy * s, 0.12 * s, "gold", THIN) for dx, dy in [(-0.22, 0.1), (0, 0.24), (0.22, 0.1), (-0.25, -0.12), (0.25, -0.12)]]
    o += [circ(x, y + 0.42 * s, 0.24 * s, "peach"), circ(x - 0.09 * s, y + 0.48 * s, 0.04 * s, "ink", THIN), circ(x + 0.09 * s, y + 0.48 * s, 0.04 * s, "ink", THIN),
          f"\\draw[{THIN}] {P(x - 0.1 * s, y + 0.34 * s)} .. controls {P(x, y + 0.28 * s)} .. {P(x + 0.1 * s, y + 0.34 * s)};", rect(x - 0.2 * s, y, 0.4 * s, 0.18 * s, "teal", THIN)]
    return o


def raft(x, y, s):
    return [rect(x - 0.45 * s, y, 0.9 * s, 0.12 * s, "wood"), line((x + 0.2 * s, y - 0.05 * s), (x + 0.45 * s, y + 0.9 * s), style="draw=wood, line width=1.2pt")]


def steps(x, y, s):
    o = []
    for k in range(4):
        o.append(rect(x - 0.5 * s + k * 0.1 * s, y + k * 0.12 * s, 1.0 * s - k * 0.2 * s, 0.12 * s, "stone", THIN))
    return o


def stage(x, y, s):
    return [*water(x, y - 0.02, 1.2 * s), rect(x - 0.45 * s, y + 0.15 * s, 0.9 * s, 0.1 * s, "wood"), line((x - 0.35 * s, y), (x - 0.35 * s, y + 0.15 * s), style=LINE),
            line((x + 0.35 * s, y), (x + 0.35 * s, y + 0.15 * s), style=LINE)]


def fan_prop(x, y, s, fill="peach"):
    o = [f"\\filldraw[fill={fill}, {LINE}] {P(x, y)} -- {P(x - 0.35 * s, y + 0.35 * s)} arc (135:45:{f(0.495 * s)}) -- cycle;"]
    for a in (60, 75, 90, 105, 120):
        ra = math.radians(a)
        o.append(line((x, y), (x + math.cos(ra) * 0.48 * s, y + math.sin(ra) * 0.48 * s)))
    return o


def sword_rack(x, y, s):
    o = [rect(x - 0.35 * s, y, 0.7 * s, 0.1 * s, "wood"), rect(x - 0.35 * s, y + 0.5 * s, 0.7 * s, 0.07 * s, "wood")]
    for k in range(3):
        xx = x - 0.2 * s + k * 0.2 * s
        o += [line((xx, y + 0.1 * s), (xx, y + 0.85 * s), style="draw=slate, line width=1.1pt"), line((xx - 0.06 * s, y + 0.22 * s), (xx + 0.06 * s, y + 0.22 * s), style=LINE)]
    return o


def table(x, y, s, top="cream"):
    return [line((x - 0.3 * s, y), (x - 0.3 * s, y + 0.35 * s), style=LINE), line((x + 0.3 * s, y), (x + 0.3 * s, y + 0.35 * s), style=LINE),
            rect(x - 0.38 * s, y + 0.35 * s, 0.76 * s, 0.07 * s, "wood", THIN), rect(x - 0.25 * s, y + 0.42 * s, 0.32 * s, 0.05 * s, top, THIN)]


def cups(x, y, s):
    o = table(x, y, s)
    o += [rect(x + dx * s - 0.04 * s, y + 0.42 * s, 0.08 * s, 0.08 * s, "cloud", THIN) for dx in (-0.2, 0, 0.2)]
    return o


def cards(x, y, s):
    o = table(x, y, s, top="teal")
    o += [rect(x + dx * s, y + 0.43 * s, 0.1 * s, 0.14 * s, "cloud", THIN) for dx in (-0.18, -0.02, 0.14)]
    return o


def scales(x, y, s):
    return [line((x, y), (x, y + 0.7 * s), style=LINE), line((x - 0.35 * s, y + 0.65 * s), (x + 0.35 * s, y + 0.65 * s), style=LINE),
            *[f"\\filldraw[fill=gold, {THIN}] {P(x + d * 0.35 * s - 0.12 * s, y + 0.4 * s)} arc (180:360:{f(0.12 * s)}) -- cycle;" for d in (-1, 1)],
            *[line((x + d * 0.35 * s, y + 0.65 * s), (x + d * 0.35 * s, y + 0.4 * s)) for d in (-1, 1)]]


def scroll(x, y, s):
    return [rect(x - 0.3 * s, y + 0.1 * s, 0.6 * s, 0.3 * s, "cream"), circ(x - 0.3 * s, y + 0.25 * s, 0.07 * s, "wood", THIN), circ(x + 0.3 * s, y + 0.25 * s, 0.07 * s, "wood", THIN),
            line((x - 0.2 * s, y + 0.3 * s), (x + 0.15 * s, y + 0.3 * s)), line((x - 0.2 * s, y + 0.2 * s), (x + 0.2 * s, y + 0.2 * s))]


def tripod(x, y, s):
    return [line((x - 0.2 * s, y), (x, y + 0.55 * s), style=LINE), line((x + 0.2 * s, y), (x, y + 0.55 * s), style=LINE), line((x, y), (x, y + 0.55 * s), style=LINE),
            rect(x - 0.15 * s, y + 0.55 * s, 0.3 * s, 0.1 * s, "gold", THIN)]


def tent(x, y, s):
    return [poly([(x - 0.55 * s, y), (x, y + 0.8 * s), (x + 0.55 * s, y)], "olive"), poly([(x - 0.12 * s, y), (x, y + 0.45 * s), (x + 0.12 * s, y)], "shadow", THIN),
            *flag(x, y + 0.8 * s, 0.4 * s, "teal")]


def cart(x, y, s, load="pot"):
    o = [rect(x - 0.35 * s, y + 0.15 * s, 0.7 * s, 0.22 * s, "wood"), circ(x - 0.2 * s, y + 0.12 * s, 0.12 * s, "stone"), circ(x + 0.2 * s, y + 0.12 * s, 0.12 * s, "stone"),
         line((x + 0.35 * s, y + 0.3 * s), (x + 0.6 * s, y + 0.2 * s), style=LINE)]
    if load == "pot":
        o += pot(x - 0.12 * s, y + 0.37 * s, 0.5 * s) + pot(x + 0.12 * s, y + 0.37 * s, 0.45 * s, "slate")
    elif load == "wheel":
        o += [circ(x, y + 0.55 * s, 0.17 * s, "stone"), circ(x, y + 0.55 * s, 0.05 * s, "slate", THIN)]
    elif load == "parcels":
        o += crate(x - 0.12 * s, y + 0.37 * s, 0.6 * s) + crate(x + 0.14 * s, y + 0.37 * s, 0.5 * s)
    return o


def horse(x, y, s, flip=False):
    o = cow(x, y, s * 1.1, flip, body="wood")
    d = -1 if flip else 1
    o.append(poly([(x + d * 0.35 * s, y + 0.6 * s), (x + d * 0.42 * s, y + 0.75 * s), (x + d * 0.3 * s, y + 0.62 * s)], "shadow", THIN))
    return o


def reeds(x, y, s):
    o = []
    for dx in (-0.15, -0.05, 0.05, 0.15):
        o.append(line((x + dx * s, y), (x + dx * s * 1.3, y + 0.7 * s), style="draw=olive, line width=0.9pt"))
        o.append(ell(x + dx * s * 1.3, y + 0.72 * s, 0.03 * s, 0.08 * s, "wood", THIN))
    return o


def rack(x, y, s, fill="olive"):
    return [line((x - 0.35 * s, y), (x - 0.35 * s, y + 0.6 * s), style=LINE), line((x + 0.35 * s, y), (x + 0.35 * s, y + 0.6 * s), style=LINE),
            line((x - 0.4 * s, y + 0.55 * s), (x + 0.4 * s, y + 0.55 * s), style=LINE),
            *[poly([(x + dx * s - 0.06 * s, y + 0.55 * s), (x + dx * s + 0.06 * s, y + 0.55 * s), (x + dx * s + 0.04 * s, y + 0.15 * s), (x + dx * s - 0.04 * s, y + 0.15 * s)], fill, THIN) for dx in (-0.2, 0, 0.2)]]


def trays(x, y, s):
    o = [line((x - 0.3 * s, y), (x - 0.3 * s, y + 0.8 * s), style=LINE), line((x + 0.3 * s, y), (x + 0.3 * s, y + 0.8 * s), style=LINE)]
    for k, c in enumerate(["shadow", "peachdark", "shadow"]):
        o.append(rect(x - 0.3 * s, y + 0.15 * s + k * 0.24 * s, 0.6 * s, 0.07 * s, c, THIN))
    return o


def sheets(x, y, s):
    o = [line((x - 0.5 * s, y + 0.8 * s), (x + 0.5 * s, y + 0.8 * s)), line((x - 0.5 * s, y), (x - 0.5 * s, y + 0.85 * s), style=LINE), line((x + 0.5 * s, y), (x + 0.5 * s, y + 0.85 * s), style=LINE)]
    o += [rect(x - 0.4 * s + k * 0.28 * s, y + 0.4 * s, 0.22 * s, 0.38 * s, "cloud", THIN) for k in range(3)]
    return o


def gravel(x, y, s):
    o = [ell(x, y + 0.08 * s, 0.5 * s, 0.12 * s, "cream")]
    for k in range(3):
        o.append(f"\\draw[stone, line width=0.5pt] {P(x - 0.42 * s, y + 0.05 * s + k * 0.04 * s)} .. controls {P(x, y + 0.1 * s + k * 0.04 * s)} .. {P(x + 0.42 * s, y + 0.05 * s + k * 0.04 * s)};")
    o.append(ell(x + 0.15 * s, y + 0.12 * s, 0.1 * s, 0.06 * s, "slate", THIN))
    return o


def chalk_ring(x, y, s):
    return [ell(x, y + 0.06 * s, 0.45 * s, 0.1 * s, "none", "draw=cloud, line width=1.2pt")]


def bench(x, y, s):
    return [rect(x - 0.4 * s, y, 0.8 * s, 0.45 * s, "wood"), rect(x - 0.5 * s, y + 0.45 * s, 1.0 * s, 0.08 * s, "peachdark"), rect(x - 0.12 * s, y + 0.53 * s, 0.24 * s, 0.25 * s, "cream", THIN)]


def bars_wall(x, y, s):
    o = wall(x, y, s, 1.0) + [rect(x - 0.18 * s, y + 0.35 * s, 0.36 * s, 0.35 * s, "shadow")]
    o += [line((x + dx * s, y + 0.35 * s), (x + dx * s, y + 0.7 * s), style="draw=stone, line width=0.9pt") for dx in (-0.09, 0, 0.09)]
    return o


def cave_mouth(x, y, s):
    return [f"\\filldraw[fill=slate, {LINE}] {P(x - 0.6 * s, y)} .. controls {P(x - 0.5 * s, y + 0.9 * s)} and {P(x + 0.5 * s, y + 0.9 * s)} .. {P(x + 0.6 * s, y)} -- cycle;",
            f"\\filldraw[fill=shadow, {THIN}] {P(x - 0.3 * s, y)} .. controls {P(x - 0.25 * s, y + 0.45 * s)} and {P(x + 0.25 * s, y + 0.45 * s)} .. {P(x + 0.3 * s, y)} -- cycle;"]


def crystals(x, y, s):
    return [poly([(x + dx * s - 0.06 * s, y), (x + dx * s + 0.06 * s, y), (x + dx * s, y + h * s)], "gold", THIN) for dx, h in [(-0.15, 0.3), (0, 0.45), (0.14, 0.25)]]


def harpoon(x, y, s):
    return [line((x - 0.3 * s, y + 0.1 * s), (x + 0.35 * s, y + 0.75 * s), style="draw=wood, line width=1.2pt"),
            poly([(x + 0.35 * s, y + 0.75 * s), (x + 0.45 * s, y + 0.9 * s), (x + 0.28 * s, y + 0.8 * s)], "slate", THIN)]


def boomerang(x, y, s):
    return [f"\\filldraw[fill=wood, {LINE}] {P(x - 0.3 * s, y + 0.2 * s)} .. controls {P(x, y + 0.6 * s)} .. {P(x + 0.3 * s, y + 0.2 * s)} -- {P(x + 0.22 * s, y + 0.16 * s)} .. controls {P(x, y + 0.45 * s)} .. {P(x - 0.22 * s, y + 0.16 * s)} -- cycle;"]


def cloth_strips(x, y, s):
    o = []
    for dx, c in [(-0.2, "teal"), (0.1, "gold"), (0.3, "peach")]:
        o += [line((x + dx * s, y), (x + dx * s, y + 0.65 * s), style="draw=olive, line width=0.9pt"),
              poly([(x + dx * s, y + 0.6 * s), (x + dx * s + 0.18 * s, y + 0.55 * s), (x + dx * s, y + 0.5 * s)], c, THIN)]
    return o


def moon_prop(x, y, s):
    return [f"\\filldraw[fill=cloud, {LINE}] {P(x, y + 0.25 * s)} circle ({f(0.2 * s)});", f"\\fill[skylight] {P(x + 0.09 * s, y + 0.3 * s)} circle ({f(0.17 * s)});"]


def platform(x, y, s):
    return [line((x - 0.3 * s, y), (x - 0.3 * s, y + 0.45 * s), style=LINE), line((x + 0.3 * s, y), (x + 0.3 * s, y + 0.45 * s), style=LINE),
            rect(x - 0.4 * s, y + 0.45 * s, 0.8 * s, 0.08 * s, "wood"), line((x - 0.1 * s, y + 0.53 * s), (x + 0.15 * s, y + 0.85 * s), style=LINE)]


PROPS = {k: v for k, v in globals().items() if callable(v) and k not in ("colors", "f", "P", "poly", "rect", "circ", "ell", "line", "PROPS")}


# --- the seal, shaped like the villager's thing -----------------------------

def seal(shape):
    x, y, r = CX, CY, R
    if shape == "kite":
        return [poly([(x, y + r * 1.15), (x + r * 0.85, y + r * 0.2), (x, y - r * 1.05), (x - r * 0.85, y + r * 0.2)], "cream", "draw=ink, line width=1.1pt"),
                line((x, y + r * 1.15), (x, y - r * 1.05)), line((x - r * 0.85, y + r * 0.2), (x + r * 0.85, y + r * 0.2)),
                f"\\draw[{THIN}] {P(x, y - r * 1.05)} .. controls {P(x - 0.25, y - r * 1.4)} and {P(x + 0.3, y - r * 1.5)} .. {P(x + 0.1, y - r * 1.85)};",
                *[poly([(x + dx, y - r * 1.3 - k * 0.18), (x + dx + 0.07, y - r * 1.36 - k * 0.18), (x + dx, y - r * 1.42 - k * 0.18)], c, THIN) for k, (dx, c) in enumerate([(-0.06, "teal"), (0.08, "gold")])]]
    if shape == "bell":
        return [f"\\filldraw[fill=cream, draw=ink, line width=1.1pt] {P(x - r, y - r * 0.75)} -- {P(x + r, y - r * 0.75)} .. controls {P(x + r * 0.6, y - r * 0.4)} and {P(x + r * 0.75, y + r * 0.85)} .. {P(x, y + r * 0.95)} .. controls {P(x - r * 0.75, y + r * 0.85)} and {P(x - r * 0.6, y - r * 0.4)} .. cycle;",
                circ(x, y - r * 0.85, 0.08, "gold", THIN), rect(x - 0.06, y + r * 0.95, 0.12, 0.12, "wood", THIN)]
    if shape == "wheel":
        o = [circ(x, y, r, "cream", "draw=ink, line width=1.1pt"), circ(x, y, r * 0.82, "none", THIN)]
        for a in range(0, 180, 45):
            ra = math.radians(a)
            o.append(line((x - math.cos(ra) * r * 0.82, y - math.sin(ra) * r * 0.82), (x + math.cos(ra) * r * 0.82, y + math.sin(ra) * r * 0.82)))
        return o + [circ(x, y, 0.1, "wood", THIN)]
    if shape == "coin":
        return [circ(x, y, r, "gold", "draw=ink, line width=1.1pt"), circ(x, y, r * 0.78, "none", THIN), rect(x - 0.1, y - 0.1, 0.2, 0.2, "cream", THIN)]
    if shape == "moon":
        return [circ(x, y, r, "cloud", "draw=ink, line width=1.1pt"), f"\\fill[skylight] {P(x + r * 0.42, y + r * 0.2)} circle ({f(r * 0.78)});",
                f"\\draw[ink, line width=1.1pt] {P(x, y)} circle ({f(r)});"]
    if shape == "sun":
        o = []
        for a in range(0, 360, 30):
            ra = math.radians(a)
            o.append(poly([(x + math.cos(ra - 0.12) * r * 0.9, y + math.sin(ra - 0.12) * r * 0.9), (x + math.cos(ra) * r * 1.3, y + math.sin(ra) * r * 1.3),
                           (x + math.cos(ra + 0.12) * r * 0.9, y + math.sin(ra + 0.12) * r * 0.9)], "gold", THIN))
        return o + [circ(x, y, r * 0.92, "gold", "draw=ink, line width=1.1pt"), circ(x, y, r * 0.6, "cream", THIN)]
    if shape == "ring":
        return [circ(x, y, r, "slate", "draw=ink, line width=1.1pt"), circ(x, y, r * 0.55, "cream", "draw=ink, line width=0.9pt")]
    if shape == "drum":
        return [rect(x - r * 0.9, y - r * 0.7, r * 1.8, r * 1.2, "peach", "draw=ink, line width=1.1pt"), ell(x, y + r * 0.5, r * 0.9, r * 0.3, "cream", "draw=ink, line width=1.1pt"),
                line((x - r * 0.9, y - r * 0.6), (x - r * 0.3, y + r * 0.25), (x + r * 0.3, y - r * 0.6), (x + r * 0.9, y + r * 0.25))]
    if shape == "hoop":
        return [circ(x, y, r, "cream", "draw=ink, line width=2.2pt"), line((x + r * 1.1, y + r * 0.9), (x + r * 0.55, y + r * 0.25), style="draw=wood, line width=1.2pt")]
    if shape == "lantern":
        return [ell(x, y, r * 0.75, r, "gold", "draw=ink, line width=1.1pt"), rect(x - r * 0.4, y + r * 0.9, r * 0.8, r * 0.18, "peachdark", THIN),
                rect(x - r * 0.4, y - r * 1.08, r * 0.8, r * 0.18, "peachdark", THIN), *[line((x + dx * r, y - r * 0.9), (x + dx * r, y + r * 0.9)) for dx in (-0.35, 0, 0.35)]]
    if shape == "fan":
        o = [f"\\filldraw[fill=cream, draw=ink, line width=1.1pt] {P(x, y - r * 0.8)} -- {P(x - r * 1.1, y + r * 0.3)} arc (135:45:{f(r * 1.556)}) -- cycle;"]
        for a in (60, 75, 90, 105, 120):
            ra = math.radians(a)
            o.append(line((x, y - r * 0.8), (x + math.cos(ra) * r * 1.5, y - r * 0.8 + math.sin(ra) * r * 1.5)))
        return o
    if shape == "barrel":
        return barrel(x, y - r * 0.85, r * 4.2, "cream")
    if shape == "shield":
        return [f"\\filldraw[fill=cream, draw=ink, line width=1.1pt] {P(x - r * 0.85, y + r * 0.8)} -- {P(x + r * 0.85, y + r * 0.8)} -- {P(x + r * 0.85, y)} .. controls {P(x + r * 0.7, y - r * 0.6)} .. {P(x, y - r * 1.0)} .. controls {P(x - r * 0.7, y - r * 0.6)} .. {P(x - r * 0.85, y)} -- cycle;"]
    if shape == "mask":  # the deities
        return [circ(x, y, r * 1.08, "gold", "draw=ink, line width=1.3pt"), circ(x, y, r * 0.86, "cream", THIN),
                *[poly([(x + dx - 0.09, y + 0.08), (x + dx + 0.09, y + 0.08), (x + dx, y + 0.18)], "ink", THIN) for dx in (-0.2, 0.2)],
                f"\\draw[{LINE}] {P(x - 0.18, y - 0.22)} .. controls {P(x, y - 0.32)} .. {P(x + 0.18, y - 0.22)};"]
    return [circ(x, y, r, "cream", "draw=ink, line width=1.1pt"), circ(x, y, r * 0.8, "none", THIN)]


# --- each fight: its seal's shape and what stands around it ------------------
# Props are (name, where, scale[, flip]): L and R stand either side of the
# seal, B stands behind it, F in front at its foot.

S = {
    "scarecrow": ("circle", [("rice", "L", 1.1), ("bird", "R", 1.6), ("flying", "B2", 1.2)]),
    "thresher": ("circle", [("sheaf", "L", 1.1), ("wheat", "R", 1.1)]),
    "courier": ("circle", [("crates", "L", 0.9), ("flag", "R", 0.9)]),
    "drover": ("circle", [("cow", "L", 1.1), ("cow", "R", 1.0, True), ("wall", "B", 1.0)]),
    "sampler": ("circle", [("cups", "L", 1.1), ("house", "R", 0.9)]),
    "salt_trader": ("circle", [("sacks", "L", 1.1), ("sacks", "R", 0.9)]),
    "lamplighter": ("lantern", [("lamp_post", "L", 1.2), ("lamp_post", "R", 1.0)]),
    "ferryman": ("circle", [("boat", "L", 1.0), ("water", "F", 1.0)]),
    "cooper": ("barrel", [("barrels", "L", 1.0), ("barrel", "R", 1.0)]),
    "windmill": ("circle", [("windmill", "B", 1.3)]),
    "ropewalker": ("circle", [("poles_rope", "B", 1.2), ("rope_coil", "R", 1.1)]),
    "smith": ("shield", [("anvil", "L", 1.1), ("fire", "R", 1.1)]),
    "bellringer": ("bell", [("tower", "B", 1.3)]),
    "gatekeeper": ("circle", [("gate", "B", 1.4), ("signboard", "R", 1.0)]),
    "reader": ("circle", [("gate", "B", 1.3), ("books", "L", 1.0)]),
    "dyer": ("circle", [("cloths", "B", 1.3), ("water", "F", 0.9)]),
    "boatwright": ("circle", [("boat", "B", 1.6, False, False), ("logs", "R", 0.9)]),
    "herbalist": ("circle", [("bush", "L", 1.0, "sage"), ("pot", "R", 1.0)]),
    "potter": ("circle", [("pots", "L", 1.0), ("kiln", "R", 1.0)]),
    "carpenter": ("circle", [("house_beams", "B", 1.3), ("logs", "R", 0.9)]),
    "weaver": ("circle", [("cloths", "B", 1.2), ("rope_coil", "L", 0.9)]),
    "falconer": ("circle", [("bird", "R", 1.8, "wood"), ("flying", "B2", 1.4), ("tree", "L", 1.0)]),
    "cartographer": ("circle", [("tripod", "L", 1.1), ("scroll", "R", 1.0)]),
    "mason": ("circle", [("blocks", "L", 1.0), ("blocks", "R", 0.9)]),
    "brewer": ("barrel", [("barrels", "L", 1.0), ("barrels", "R", 0.9)]),
    "archivist": ("circle", [("shelf", "B", 1.3), ("books", "R", 1.0)]),
    "magistrate": ("circle", [("bench", "B", 1.3), ("scales", "R", 1.0)]),
    "abbot": ("circle", [("steps", "F", 1.3), ("torii", "B", 1.4)]),
    "watchman": ("circle", [("tower", "L", 1.0), ("bridge", "R", 0.9)]),
    "warden": ("shield", [("bars_wall", "B", 1.3)]),
    "hermit": ("circle", [("cave_mouth", "B", 1.5), ("tree_pine", "R", 1.0)]),
    "tanner": ("circle", [("hide_frame", "L", 1.0), ("barrel", "R", 1.0)]),
    "vaulter": ("circle", [("poles_rope", "B", 1.1), ("blocks", "R", 0.6)]),
    "tinker": ("circle", [("cart_pot", "L", 1.1), ("pot", "R", 0.8)]),
    "pilgrim": ("circle", [("torii", "B", 1.3), ("crate", "R", 0.8)]),
    "knife_grinder": ("wheel", [("cart_wheel", "L", 1.0)]),
    "ox_herd": ("circle", [("ox", "L", 1.1), ("gate", "R", 0.8)]),
    "eel_fisher": ("circle", [("fish", "L", 1.0), ("water", "F", 1.1), ("reeds", "R", 1.0)]),
    "bargeman": ("circle", [("boat_low", "B", 1.7), ("water", "F", 1.2)]),
    "rice_broker": ("circle", [("signboard", "L", 1.2), ("sacks", "R", 1.0)]),
    "bell_founder": ("bell", [("fire", "L", 1.0), ("bell_prop", "R", 1.0)]),
    "porter": ("circle", [("crates", "L", 1.1), ("crates", "R", 0.9)]),
    "lacquerer": ("circle", [("trays", "L", 1.1), ("pot", "R", 0.8)]),
    "post_rider": ("circle", [("horse", "L", 1.1), ("flag", "R", 0.9)]),
    "paper_maker": ("circle", [("sheets", "B", 1.3), ("water", "F", 0.8)]),
    "gambler": ("coin", [("cards", "L", 1.1), ("coins", "R", 1.0)]),
    "monk": ("circle", [("gravel", "F", 1.3), ("torii", "B", 1.2)]),
    "general": ("shield", [("tent", "B", 1.3), ("flag", "R", 1.0)]),
    "duelist": ("circle", [("wall", "B", 1.6), ("tree", "R", 1.0)]),
    "tea_picker": ("circle", [("bush", "L", 1.0, "sagedark"), ("bush", "R", 0.9, "sage"), ("wall", "F", 1.2)]),
    "well_digger": ("circle", [("well", "L", 1.1), ("rope_coil", "R", 0.9)]),
    "kite_maker": ("kite", [("kite", "L", 0.9, "teal"), ("kite", "R", 0.8, "gold")]),
    "charcoal_burner": ("circle", [("mound", "L", 1.0), ("mound", "R", 0.9)]),
    "bridge_keeper": ("circle", [("bridge", "B", 1.5), ("water", "F", 1.2)]),
    "roofer": ("circle", [("roofs", "B", 1.3), ("ladder", "R", 1.0)]),
    "toll_collector": ("coin", [("gate", "L", 0.9), ("coins", "R", 0.9)]),
    "net_mender": ("circle", [("net", "B", 1.3), ("fish", "R", 0.8)]),
    "stone_cutter": ("circle", [("crane", "L", 1.1), ("blocks", "R", 0.9)]),
    "shrine_keeper": ("circle", [("torii", "B", 1.4), ("steps", "F", 1.1)]),
    "juggler": ("circle", [("balls", "B", 1.3), ("chalk_ring", "F", 1.3)]),
    "woodcutter": ("circle", [("tree_pine", "L", 1.2), ("logs", "R", 1.0)]),
    "harpooner": ("circle", [("boat_low", "L", 0.8), ("harpoon", "R", 1.0), ("water", "F", 1.0)]),
    "bird_scarer": ("circle", [("cloth_strips", "L", 1.1), ("flying", "B2", 1.2), ("boomerang", "R", 0.9)]),
    "hare_hunter": ("circle", [("hare", "L", 1.4), ("hare", "R", 1.1, True), ("boomerang", "B2", 0.9)]),
    "reed_cutter": ("circle", [("reeds", "L", 1.1), ("rack", "R", 1.0)]),
    "crow_watcher": ("circle", [("stilt_hut", "B", 1.3), ("bird", "R", 1.3), ("flying", "B2", 1.1)]),
    "shepherd": ("circle", [("sheep", "L", 1.1), ("sheep", "R", 1.0, True), ("wall", "B", 1.0)]),
    "drum_maker": ("drum", [("hide_frame", "L", 1.0), ("drum", "R", 0.8)]),
    "wind_reader": ("circle", [("flags", "B", 1.3), ("boomerang", "R", 0.9)]),
    "hoop_roller": ("hoop", [("hoop", "L", 0.9), ("house", "R", 0.8)]),
    "coin_minter": ("coin", [("coins", "L", 1.0), ("fire", "R", 1.0)]),
    "bangle_seller": ("ring", [("bangles", "L", 1.1), ("bangles", "R", 0.9)]),
    "wheelwright": ("wheel", [("wheel", "L", 1.0), ("wheel", "R", 0.9)]),
    "moon_watcher": ("moon", [("platform", "B", 1.3), ("moon_prop", "R", 1.2)]),
    "quoit_thrower": ("ring", [("quoits", "L", 1.1), ("quoits", "R", 1.0)]),
    "sun_priest": ("sun", [("gate", "B", 1.5)]),
    "ring_keeper": ("ring", [("cage_bars", "O", 1.5), ("lion", "L", 0.9), ("cage", "R", 0.8)]),
    "hay_mower": ("circle", [("scythe_prop", "L", 1.1), ("sheaf", "R", 0.9)]),
    "gleaner": ("circle", [("wheat", "L", 1.0), ("wheat", "R", 0.8)]),
    "thatcher": ("circle", [("house_straw", "B", 1.3), ("ladder", "R", 1.0)]),
    "orchard_keeper": ("circle", [("tree_fruit", "L", 1.2), ("ladder", "R", 1.0)]),
    "grave_digger": ("circle", [("grave", "L", 1.2), ("grave", "R", 1.0)]),
    "sheaf_binder": ("circle", [("sheaf", "L", 1.1), ("sheaf", "R", 1.0)]),
    "field_warden": ("circle", [("stilt_hut", "B", 1.3), ("rice", "R", 1.0)]),
    "harvest_keeper": ("circle", [("silo", "B", 1.4), ("jar", "L", 1.0), ("jar", "R", 0.9)]),
    "stilt_walker": ("lantern", [("lanterns", "B", 1.4)]),
    "acrobat": ("circle", [("poles_rope", "B", 1.4)]),
    "lion_dancer": ("circle", [("lion", "L", 1.1), ("torii", "B", 1.3)]),
    "punt_poler": ("circle", [("raft", "B", 1.4), ("water", "F", 1.3)]),
    "temple_guard": ("shield", [("torii", "B", 1.5), ("steps", "F", 1.2)]),
    "fan_dancer": ("fan", [("stage", "F", 1.3)]),
    "festival_master": ("lantern", [("lanterns", "B", 1.5), ("drum", "R", 0.8)]),
    "sword_dancer": ("circle", [("sword_rack", "B", 1.3)]),
    "local_deity": ("mask", [("torii", "B", 1.6), ("steps", "F", 1.3)]),
    "guardian_deity": ("mask", [("cave_mouth", "B", 1.7), ("crystals", "L", 1.2), ("crystals", "R", 1.0)]),
}

EXTRA = {
    "house_beams": lambda x, y, s: house(x, y, s, beams=True),
    "house_straw": lambda x, y, s: house(x, y, s, wall="peach", roof="gold"),
    "tree_pine": lambda x, y, s: tree(x, y, s, pine=True),
    "tree_fruit": lambda x, y, s: tree(x, y, s, fruit=True),
    "cart_pot": lambda x, y, s: cart(x, y, s, "pot"),
    "cart_wheel": lambda x, y, s: cart(x, y, s, "wheel"),
    "boat_low": lambda x, y, s: boat(x, y, s, sail=False),
}

# The ground of each row, as its region's art has it.
GROUND = ["sage", "stone", "teal", "stone", "sagedark", "slate", "olive", "stone", "sage", "slate"]


def levels():
    road = json.loads((ROOT / "data" / "road.json").read_text())
    stops = road["stops"] if isinstance(road, dict) else road
    by = {s["id"]: s for s in stops}
    memo = {}

    def lv(i):
        if i in memo:
            return memo[i]
        s = by[i]
        if "row" in s:
            memo[i] = s["row"]
        else:
            ids = [list(r.values())[0] for r in s["requires"]]
            ids = [x["stop"] if isinstance(x, dict) else x for x in ids]
            memo[i] = 0 if not ids else 1 + max(lv(x) for x in ids if x in by)
        return memo[i]
    return {i: lv(i) for i in by}


def place(name, where, scale, *extra):
    fn = EXTRA.get(name) or PROPS[name]
    gy = 0.62
    x, y = {"L": (0.72, gy), "R": (3.28, gy), "B": (CX, gy + 0.18), "O": (CX, gy + 0.1), "F": (CX, 0.32), "B2": (3.1, 2.55)}[where]
    if where == "B":  # behind the seal, and tall enough to show over it
        scale *= 1.45
    elif where in ("L", "R"):
        scale *= 1.2
    if extra and isinstance(extra[0], bool):
        return fn(x, y, scale * 0.85, extra[0]) if len(extra) == 1 else fn(x, y, scale * 0.85, *extra)
    if extra:
        return fn(x, y, scale * 0.85, *extra)
    return fn(x, y, scale * 0.85)


def scene(i, level):
    shape, props = S[i]
    ground = GROUND[min(level, len(GROUND) - 1)]
    o = [f"\\filldraw[fill=shadow, draw=none, opacity=0.35] {P(CX + 0.06, 0.52)} ellipse (1.88 and 0.42);",
         f"\\filldraw[fill={ground}, {LINE}] {P(CX, 0.62)} ellipse (1.85 and 0.42);"]
    behind = [p for p in props if p[1] in ("B", "B2")]
    rest = [p for p in props if p[1] not in ("B", "B2", "O")]
    over = [p for p in props if p[1] == "O"]
    for p in behind:
        o += place(*p)
    o += seal(shape)
    for p in rest + over:
        o += place(*p)
    return o


def render(i, level):
    tex = f"""\\documentclass[tikz,border=0pt]{{standalone}}
\\usepackage{{tikz}}
{colors()}
\\begin{{document}}
\\begin{{tikzpicture}}
\\useasboundingbox (0,0) rectangle ({W},{H});
{chr(10).join(scene(i, level))}
\\end{{tikzpicture}}
\\end{{document}}
"""
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as d:
        Path(d, "f.tex").write_text(tex)
        rr = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "f.tex"], cwd=d, capture_output=True, text=True)
        if rr.returncode:
            sys.exit(f"pdflatex failed for {i}:\n" + "\n".join(l for l in rr.stdout.splitlines() if l.startswith("!") or l.startswith("l.")))
        subprocess.run(["pdftocairo", "-png", "-transp", "-singlefile", "-scale-to-x", str(PX), "-scale-to-y", "-1", "f.pdf", str(OUT / i)], cwd=d, check=True)


if __name__ == "__main__":
    lv = levels()
    missing = sorted(set(lv) - set(S))
    if missing:
        sys.exit(f"no scene for {missing}")
    for i in (sys.argv[1:] or sorted(S)):
        render(i, lv[i])
        print("drew", i)
