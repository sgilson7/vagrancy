#!/usr/bin/env python3
"""Paul and Feyd-Rautha's costumes and the
Arrakeen Residency hall for a fan clip, after Sam's reference pack
(~/Downloads/Dune_Paul_Feyd_Duel_Reference_Pack): Paul's dusty stillsuit,
scarf wraps and curls; Feyd's pale bald head and sculpted black Harkonnen
leathers; the hall in amber dawn light, sandstone pillars, the court along
the walls, the throne on its dais. Drawn the way analysis/art/costumes.py
and scenes.py draw the game's, into web/art (Sam, 2026-10-08: the duel is
story mode's prize for its first chapter, data/duels.json).

    python3 analysis/art/duel_arrakeen.py
"""
import json, math, random, subprocess, sys, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
DATA = json.loads((ROOT / "data" / "costumes.json").read_text())
OUT = ROOT / "web" / "art"
PX = 5

C = {
    "ink": "#2A2420",
    "skin": "#D9B99B", "skin_dark": "#B8977A", "hair": "#3A2C24",
    "suit": "#7D7163", "suit_dark": "#5E554B", "suit_light": "#9C8F7E", "tube": "#4B433B",
    "wrap": "#A8957B", "wrap_dark": "#8B795F", "cloak": "#8E7D66",
    "pale": "#ECE3D8", "pale_dark": "#C9BDB0",
    "black": "#1C1A1E", "black_sheen": "#3E3A44", "black_edge": "#0E0D10",
    "hall": "#D3B48C", "hall_dark": "#BF9C72", "pillar": "#B08B62", "pillar_dark": "#9A774F", "shaft": "#EAD5B0",
    "court": "#8E7559", "court_dark": "#7A634A", "gold": "#C9A24E", "dais": "#C4A57E", "dais_line": "#A8875F",
}
LW = 0.5


def P(x, y):
    return f"({x:.2f},{y:.2f})"


def colors():
    return "\n".join(f"\\definecolor{{{k.replace('_', '')}}}{{HTML}}{{{v[1:]}}}" for k, v in C.items())


def n(c):
    return c.replace("_", "")


def poly(pts, fill, lw=LW, line="ink"):
    return f"\\filldraw[fill={n(fill)}, draw={n(line)}, line width={lw}cm, line join=round] " + " -- ".join(P(x, y) for x, y in pts) + " -- cycle;"


def smooth(pts, fill, lw=LW, line="ink"):
    return f"\\filldraw[fill={n(fill)}, draw={n(line)}, line width={lw}cm] plot[smooth cycle, tension=0.6] coordinates {{{' '.join(P(x, y) for x, y in pts)}}};"


def curve(pts, color, lw=LW * 0.7):
    return f"\\draw[draw={n(color)}, line width={lw}cm, line cap=round] plot[smooth, tension=0.6] coordinates {{{' '.join(P(x, y) for x, y in pts)}}};"


def circ(x, y, r, fill, lw=LW * 0.6, line="ink"):
    return f"\\filldraw[fill={n(fill)}, draw={n(line)}, line width={lw}cm] {P(x, y)} circle ({r:.2f});"


def torso(fill, r=15, top=167, bottom=100, lw=LW):
    return f"\\filldraw[fill={n(fill)}, draw=ink, line width={lw}cm] {P(-r, bottom + r)} -- {P(-r, top - r)} arc (180:0:{r}) -- {P(r, bottom + r)} arc (0:-180:{r}) -- cycle;"


# --- Paul ---------------------------------------------------------------------------

def paul_head():
    o = [smooth([(-11, 158), (-12, 180), (-8, 194), (2, 197), (10, 192), (13, 182), (12, 170), (9, 160), (2, 156)], "skin")]
    rnd = random.Random(5)
    for k in range(16):
        a = math.radians(70 + k * 13)
        o.append(circ(-1 + 11 * math.cos(a), 186 + 10 * math.sin(a), rnd.uniform(3.2, 4.4), "hair", LW * 0.4))
    o.append(circ(-9, 178, 4, "hair", LW * 0.4))
    o.append(curve([(7, 184), (11, 183)], "hair", LW * 0.6))
    o.append(circ(9.3, 181, 0.9, "ink", 0.01))
    # The scarf wrapped high round the neck.
    o.append(smooth([(-13, 160), (-4, 168), (10, 165), (14, 158), (6, 150), (-8, 150)], "wrap"))
    o.append(curve([(-11, 158), (0, 162), (12, 158)], "wrap_dark"))
    o.append(curve([(-12, 154), (0, 157), (12, 153)], "wrap_dark"))
    return o


def paul_chest():
    o = []
    # The cloak and the scarf's long ends behind.
    o.append(poly([(-12, 166), (-30, 92), (-22, 86), (-14, 96), (-6, 150)], "cloak"))
    o.append(curve([(-14, 160), (-24, 120), (-27, 94)], "wrap_dark"))
    o.append(torso("suit"))
    for y in range(108, 160, 7):
        o.append(curve([(-13, y), (0, y + 1.5), (13, y)], "suit_dark", LW * 0.45))
    o.append(poly([(-6, 150), (10, 150), (12, 128), (-4, 126)], "suit_light", LW * 0.6))
    # The stillsuit's tubes, chest to collar.
    o.append(curve([(4, 128), (9, 140), (6, 156), (2, 163)], "tube", LW * 1.4))
    o.append(curve([(-2, 126), (-8, 142), (-6, 160)], "tube", LW * 1.4))
    o.append(poly([(-14, 112), (14, 112), (14, 118), (-14, 118)], "suit_dark", LW * 0.6))
    o.append(smooth([(-14, 166), (-2, 172), (12, 168), (16, 160), (6, 156), (-10, 156)], "wrap"))
    return o


def paul_waist():
    o = [poly([(-14, 120), (14, 120), (16, 80), (4, 80), (1, 92), (-2, 80), (-15, 80)], "suit")]
    for y in (108, 98, 88):
        o.append(curve([(-14, y), (0, y + 1), (15, y)], "suit_dark", LW * 0.45))
    o.append(poly([(-15, 112), (15, 112), (15, 119), (-15, 119)], "suit_dark", LW * 0.6))
    for x in (-10, 6):
        o.append(poly([(x, 112), (x + 6, 112), (x + 6, 104), (x, 104)], "wrap_dark", LW * 0.5))
    # A wrap hanging at the hip.
    o.append(poly([(-15, 118), (-9, 118), (-12, 84), (-18, 88)], "wrap", LW * 0.6))
    return o


# --- Feyd-Rautha ------------------------------------------------------------------------

def feyd_head():
    o = [smooth([(-11, 158), (-12, 182), (-7, 195), (3, 197), (11, 191), (13, 180), (12, 168), (9, 159), (2, 156)], "pale")]
    o.append(curve([(-6, 194), (2, 196.5), (9, 192)], "pale_dark", LW * 0.6))
    o.append(curve([(6, 185), (12, 184.5)], "pale_dark", LW * 0.7))
    o.append(poly([(8.5, 182), (11.5, 182), (11, 180.5), (9, 180.5)], "ink", 0.01))
    o.append(curve([(9, 170), (12, 171)], "pale_dark", LW * 0.6))
    # The high black collar.
    o.append(poly([(-13, 150), (-12, 164), (-4, 168), (11, 166), (13, 152)], "black"))
    o.append(curve([(-11, 162), (0, 166), (11, 164)], "black_sheen", LW * 0.5))
    return o


def feyd_chest():
    o = [torso("black")]
    # Sculpted plates: a breastplate, ribs of black, a pauldron.
    o.append(poly([(-2, 160), (14, 154), (15, 128), (4, 120), (-6, 132)], "black_sheen", LW * 0.6))
    for k in range(4):
        y = 146 - k * 9
        o.append(curve([(-13, y), (-2, y - 3), (13, y - 1)], "black_sheen", LW * 0.45))
    o.append(poly([(-15, 166), (6, 168), (10, 156), (2, 148), (-14, 146)], "black"))
    o.append(curve([(-12, 162), (4, 164), (8, 156)], "black_sheen", LW * 0.5))
    o.append(poly([(-15, 112), (15, 112), (15, 118), (-15, 118)], "black_edge", LW * 0.6))
    return o


def feyd_waist():
    o = [poly([(-14, 120), (14, 120), (15, 80), (4, 80), (1, 90), (-2, 80), (-15, 80)], "black")]
    o.append(curve([(-12, 116), (-2, 100), (-6, 82)], "black_sheen", LW * 0.45))
    o.append(curve([(12, 116), (6, 100), (10, 82)], "black_sheen", LW * 0.45))
    o.append(poly([(-15, 112), (15, 112), (15, 119), (-15, 119)], "black_edge", LW * 0.6))
    o.append(poly([(-2, 113), (4, 113), (4, 118), (-2, 118)], "black_sheen", LW * 0.4))
    return o


PIECES = {("paul", "head"): paul_head, ("paul", "chest"): paul_chest, ("paul", "waist"): paul_waist,
          ("feyd", "head"): feyd_head, ("feyd", "chest"): feyd_chest, ("feyd", "waist"): feyd_waist}


# --- the hall -------------------------------------------------------------------------

def hall_back():
    o = [poly([(-1100, -200), (1100, -200), (1100, 2000), (-1100, 2000)], "hall", 0.01, "hall")]
    # Shafts of dawn light, from high on the left.
    for x in (-700, -260, 220, 640):
        o.append(f"\\fill[{n('shaft')}, opacity=0.55] {P(x - 400, 2000)} -- {P(x - 240, 2000)} -- {P(x + 330, -50)} -- {P(x + 150, -50)} -- cycle;")
    # Monolithic pillars.
    for x in (-900, -520, 420, 820):
        o.append(poly([(x, -200), (x + 150, -200), (x + 140, 2000), (x + 10, 2000)], "pillar", 0.12, "pillar_dark"))
        o.append(poly([(x + 110, -200), (x + 150, -200), (x + 140, 2000), (x + 104, 2000)], "pillar_dark", 0.01, "pillar_dark"))
    return o


def figure(x, y, h, fill):
    w = h * 0.22
    return [smooth([(x - w / 2, y), (x - w / 2 * 1.1, y + h * 0.6), (x - w / 3, y + h * 0.82), (x + w / 3, y + h * 0.82), (x + w / 2 * 1.1, y + h * 0.6), (x + w / 2, y)], fill, 0.01, fill),
            circ(x, y + h * 0.9, h * 0.09, fill, 0.01, fill)]


def hall_court():
    o = []
    # The dais and the throne, far back in the middle.
    for k in range(4):
        w = 300 - k * 40
        o.append(poly([(-w / 2, k * 12), (w / 2, k * 12), (w / 2, k * 12 + 12), (-w / 2, k * 12 + 12)], "dais", 0.1, "dais_line"))
    o.append(poly([(-22, 48), (22, 48), (20, 130), (12, 150), (-12, 150), (-20, 130)], "gold", 0.12, "court_dark"))
    o.append(poly([(-16, 60), (16, 60), (14, 120), (-14, 120)], "court_dark", 0.01, "court_dark"))
    o += figure(0, 60, 70, "court_dark")
    # The court standing along the walls, in rows.
    rnd = random.Random(9)
    for row, (y, h, fill) in enumerate([(14, 72, "court"), (6, 84, "court_dark")]):
        for side in (-1, 1):
            x = 190 * side
            while abs(x) < 950:
                o += figure(x + rnd.uniform(-6, 6), y, h * rnd.uniform(0.92, 1.05), fill)
                x += side * rnd.uniform(36, 48)
    return o


SCENE = {"hall": (hall_back, [-1100, -200, 1100, 2000], 0.06), "court": (hall_court, [-1100, -200, 1100, 300], 0.3)}


def tex_doc(box, body, unit="1cm"):
    x0, y0, x1, y1 = box
    return f"""\\documentclass[tikz,border=0pt]{{standalone}}
\\usepackage{{tikz}}
{colors()}
\\begin{{document}}
\\begin{{tikzpicture}}[x={unit}, y={unit}]
\\useasboundingbox ({x0},{y0}) rectangle ({x1},{y1});
\\begin{{scope}}
\\clip ({x0},{y0}) rectangle ({x1},{y1});
{chr(10).join(body)}
\\end{{scope}}
\\end{{tikzpicture}}
\\end{{document}}
"""


def render(tex, out, width):
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as d:
        Path(d, "f.tex").write_text(tex)
        rr = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "f.tex"], cwd=d, capture_output=True, text=True)
        if rr.returncode:
            sys.exit(f"pdflatex failed for {out.name}:\n" + "\n".join(l for l in rr.stdout.splitlines() if l.startswith("!") or l.startswith("l.")))
        subprocess.run(["pdftocairo", "-png", "-transp", "-singlefile", "-scale-to-x", str(width), "-scale-to-y", "-1", "f.pdf", str(out.with_suffix(""))], cwd=d, check=True)


def main():
    for (who, slot), fn in PIECES.items():
        box = DATA["slots"][slot]["box"]
        render(tex_doc(box, fn()), OUT / "costume" / f"{who}-{slot}.png", (box[2] - box[0]) * PX)
        print("drew", who, slot)
    for name, (fn, box, _) in SCENE.items():
        # A tenth of a centimeter a centimeter, so the picture stays inside
        # TeX's largest size; line widths above are in that unit too.
        render(tex_doc(box, fn(), "0.1cm"), OUT / "scene" / f"arrakeen-{name}.png", (box[2] - box[0]) * 2)
        print("drew arrakeen", name)


if __name__ == "__main__":
    main()
