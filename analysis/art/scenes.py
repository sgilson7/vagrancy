#!/usr/bin/env python3
"""Backgrounds behind a fight (Sam, 2026-10-08: "some backgrounds during the
fights that are pretty lightweight, built the same way as the costumes, that
do not pollute the background too much ... a village far in the distance on
the left, and then a camp filled with soldiers far in the distance down the
road in the forest to the right, and the woodcutter and warden are fighting
in a clearing ... play with 3d perspective / parallax").

Each scene in data/backgrounds.json is layers; each layer is one picture,
drawn here in TikZ in the arena's own centimeters (the ground at y = 0, the
fighters about 200 cm tall), over the layer's box. The page moves each layer
by its depth as the camera moves, so the far ones barely move. Colors are
palette.json's `scenery` set: the region art's, faded toward the paper, the
far layers palest, so nothing behind a fighter competes with it.

    python3 analysis/art/scenes.py            # writes web/art/scene/<scene>-<layer>.png
"""
import json, math, random, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PAL = {**json.loads((ROOT / "data" / "palette.json").read_text())["scenery"], "paper": json.loads((ROOT / "data" / "palette.json").read_text())["paper"]}
DATA = json.loads((ROOT / "data" / "backgrounds.json").read_text())
OUT = ROOT / "web" / "art" / "scene"
PX = 2  # pixels a centimeter


def name(c):
    return c.replace("_", "")


def colors():
    return "\n".join(f"\\definecolor{{{name(k)}}}{{HTML}}{{{v[1:]}}}" for k, v in PAL.items() if not k.startswith("_"))


def P(x, y):
    return f"({x:.1f},{y:.1f})"


def poly(pts, fill, line, lw=1.4):
    return f"\\filldraw[fill={name(fill)}, draw={name(line)}, line width={lw * 0.1:.3f}cm, line join=round] " + " -- ".join(P(x, y) for x, y in pts) + " -- cycle;"


def line(pts, color, lw=1.2):
    return f"\\draw[draw={name(color)}, line width={lw * 0.1:.3f}cm, line cap=round, line join=round] " + " -- ".join(P(x, y) for x, y in pts) + ";"


def smooth(pts, fill, line_c, lw=1.4):
    return f"\\filldraw[fill={name(fill)}, draw={name(line_c)}, line width={lw * 0.1:.3f}cm] plot[smooth, tension=0.7] coordinates {{{' '.join(P(x, y) for x, y in pts)}}} -- cycle;"


# --- the clearing ------------------------------------------------------------------

def house(x, y, w, h, roof="far_roof"):
    return [poly([(x - w / 2, y), (x + w / 2, y), (x + w / 2, y + h), (x - w / 2, y + h)], "far_wall", "far_line", 0.9),
            poly([(x - w / 2 - 4, y + h), (x + w / 2 + 4, y + h), (x + w / 4, y + h + h * 0.7), (x - w / 4, y + h + h * 0.7)], roof, "far_line", 0.9),
            poly([(x - 3, y), (x + 3, y), (x + 3, y + h * 0.5), (x - 3, y + h * 0.5)], "far_line", "far_line", 0.4)]


def pagoda(x, y):
    o = []
    for k in range(3):
        w = 34 - k * 8
        yy = y + k * 26
        o += [poly([(x - w / 2 + 4, yy), (x + w / 2 - 4, yy), (x + w / 2 - 4, yy + 16), (x - w / 2 + 4, yy + 16)], "far_wall", "far_line", 0.9),
              poly([(x - w / 2 - 6, yy + 16), (x + w / 2 + 6, yy + 16), (x + w / 4, yy + 26), (x - w / 4, yy + 26)], "far_roof", "far_line", 0.9)]
    o.append(line([(x, y + 78), (x, y + 92)], "far_line", 0.9))
    return o


def tent(x, y, w):
    return [poly([(x - w / 2, y), (x + w / 2, y), (x, y + w * 0.7)], "far_tent", "far_line", 0.9), line([(x, y), (x, y + w * 0.35)], "far_line", 0.6)]


def banner(x, y, h):
    return [line([(x, y), (x, y + h)], "far_line", 0.6), poly([(x, y + h), (x + 9, y + h - 3), (x, y + h - 7)], "far_roof", "far_line", 0.5)]


def soldier(x, y):
    return [line([(x, y), (x, y + 9)], "far_line", 1.6), f"\\fill[{name('far_line')}] {P(x, y + 11)} circle (1.6);", line([(x + 2, y + 2), (x + 3, y + 18)], "far_line", 0.4)]


def smoke(x, y, h):
    pts = [(x + 6 * math.sin(k * 0.9), y + k * h / 8) for k in range(9)]
    return [line(pts, "far_smoke", 2.4)]


def pine(x, y, h, fill="mid_tree", lw=1.4, trunk="mid_trunk"):
    w = h * 0.42
    o = [poly([(x - 3, y), (x + 3, y), (x + 3, y + h * 0.18), (x - 3, y + h * 0.18)], trunk, "mid_line", lw * 0.7)]
    for k in range(3):
        b = y + h * 0.15 + k * h * 0.24
        ww = w * (1 - k * 0.25)
        o.append(poly([(x - ww / 2, b), (x + ww / 2, b), (x, b + h * 0.42)], fill, "mid_line", lw))
    return o


def clearing_hills():
    o = []
    # Rolling hills along the horizon, and the road from the village to the camp.
    o.append(smooth([(-1100, -200), (-1100, 70), (-900, 95), (-650, 75), (-420, 105), (-150, 80), (100, 98), (380, 72), (650, 110), (900, 85), (1100, 100), (1100, -200)], "far_hill", "far_line", 0.9))
    o.append(poly([(-1100, -200), (1100, -200), (1100, 40), (-1100, 40)], "far_hill", "far_hill", 0.1))
    o.append(line([(-420, 40), (-220, 30), (0, 24), (220, 30), (420, 40)], "road", 6))
    return o


def clearing_far():
    rnd = random.Random(3)
    o = []
    # The village, far down the road to the left.
    for x, w, h in [(-560, 46, 26), (-500, 36, 22), (-440, 50, 28), (-385, 34, 20), (-620, 30, 18), (-330, 40, 22)]:
        o += house(x, 40, w, h)
    o += pagoda(-470, 70)
    o += smoke(-500, 70, 70) + smoke(-390, 66, 60)
    # The camp in the trees to the right: tents, banners, soldiers.
    for x in range(470, 900, 30):
        o += pine(x + rnd.uniform(-8, 8), 40, rnd.uniform(90, 130), "far_tree", 0.9, "far_tent")
    for x, w in [(300, 40), (352, 34), (405, 44), (460, 36), (512, 40)]:
        o += tent(x, 40, w)
    for x in (320, 385, 440, 495):
        o += banner(x, 40, 62)
    for x in range(285, 540, 14):
        o += soldier(x + rnd.uniform(-3, 3), 40)
    o += smoke(420, 60, 80)
    return o


def clearing_mid():
    rnd = random.Random(7)
    o = []
    # The forest round the clearing: thick at the sides, open in the middle,
    # so nothing stands behind the fighters.
    for side in (-1, 1):
        x = 1080 * side
        while abs(x) > 430:
            h = rnd.uniform(220, 380)
            o += pine(x, 0, h, "mid_tree" if rnd.random() < 0.6 else "mid_tree_dark")
            x -= side * rnd.uniform(40, 75)
    o.append(poly([(-1100, -200), (1100, -200), (1100, 6), (-1100, 6)], "mid_field", "mid_field", 0.1))
    return o


def clearing_near():
    rnd = random.Random(11)
    o = []
    for x in range(-1080, 1100, 26):
        if abs(x) < 380 and rnd.random() < 0.75:
            continue
        h = rnd.uniform(8, 22)
        o.append(line([(x, 0), (x - 4, h)], "near_grass", 1.6))
        o.append(line([(x + 4, 0), (x + 7, h * 0.8)], "near_grass", 1.6))
    # A felled tree's stumps, the woodcutter's.
    for x in (-520, 470, 840):
        o.append(poly([(x - 12, 0), (x + 12, 0), (x + 10, 22), (x - 10, 22)], "near_stump", "near_line", 1.2))
        o.append(f"\\filldraw[fill={name('road')}, draw={name('near_line')}, line width=0.1cm] {P(x, 22)} ellipse (10 and 3);")
    return o


SCENES = {"clearing": {"hills": clearing_hills, "far": clearing_far, "mid": clearing_mid, "near": clearing_near}}


def render(scene, layer):
    x0, y0, x1, y1 = layer["box"]
    body = SCENES[scene][layer["name"]]()
    tex = f"""\\documentclass[tikz,border=0pt]{{standalone}}
\\usepackage{{tikz}}
{colors()}
\\begin{{document}}
\\begin{{tikzpicture}}[x=0.1cm, y=0.1cm]
\\useasboundingbox ({x0},{y0}) rectangle ({x1},{y1});
\\begin{{scope}}
\\clip ({x0},{y0}) rectangle ({x1},{y1});
{chr(10).join(body)}
\\end{{scope}}
\\end{{tikzpicture}}
\\end{{document}}
"""
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as d:
        Path(d, "f.tex").write_text(tex)
        rr = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "f.tex"], cwd=d, capture_output=True, text=True)
        if rr.returncode:
            sys.exit(f"pdflatex failed for {scene}-{layer['name']}:\n" + "\n".join(l for l in rr.stdout.splitlines() if l.startswith("!") or l.startswith("l.")))
        subprocess.run(["pdftocairo", "-png", "-transp", "-singlefile", "-scale-to-x", str((x1 - x0) * PX), "-scale-to-y", "-1", "f.pdf", str(OUT / f"{scene}-{layer['name']}")], cwd=d, check=True)


if __name__ == "__main__":
    for scene, s in DATA["scenes"].items():
        for layer in s["layers"]:
            render(scene, layer)
            print("drew", scene, layer["name"])
