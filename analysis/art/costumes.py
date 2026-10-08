#!/usr/bin/env python3
"""What the road's opponents wear (Sam, 2026-10-08: "the herbalist could wear a
little leaf outfit, the abbot an abbot outfit, the general a general outfit,
the village deity is glowing, the pilgrim is dressed like a pilgrim from elden
ring, the scarecrow has a pumpkin head, the bridge keeper is dreesed like a
knight with a big sombrero type steel helmet, the ox herd has horns").

Each costume in data/costumes.json names a piece for each slot (head, chest,
waist) and two colors. Each slot's picture is drawn here in TikZ, in the
fighter's own centimeters, in its rest pose facing +x, over the slot's box,
so the page can lay it on the part it rides with one transform. The style is
the region art's (regions.py, after Sam_Comm_final.png): flat fills and plum
ink lines. A piece that is also a plate of armor (the helmet, the sombrero,
the horns, the breastplate) is drawn on the plate's own points, so what is
seen is what stops a blade.

    python3 analysis/art/costumes.py            # writes web/art/costume/<id>-<slot>.png
    python3 analysis/art/costumes.py abbot      # just one costume
"""
import json, math, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PAL_FILE = json.loads((ROOT / "data" / "palette.json").read_text())
PAL = {**PAL_FILE["art"], **PAL_FILE["costume"]}
DATA = json.loads((ROOT / "data" / "costumes.json").read_text())
OUT = ROOT / "web" / "art" / "costume"
PX = 5  # pixels a centimeter of the body
LW = 0.55  # ink line, in body cm
LINE = f"draw=ink, line width={LW}cm, line join=round, line cap=round"
THIN = f"draw=ink, line width={LW * 0.6}cm, line cap=round, line join=round"


def name(c):
    return c.replace("_", "")


def colors():
    return "\n".join(f"\\definecolor{{{name(k)}}}{{HTML}}{{{v[1:]}}}" for k, v in PAL.items() if not k.startswith("_"))


def f(v):
    return f"{v:.2f}"


def P(x, y):
    return f"({f(x)},{f(y)})"


def poly(pts, fill, style=LINE):
    return f"\\filldraw[fill={name(fill)}, {style}] " + " -- ".join(P(x, y) for x, y in pts) + " -- cycle;"


def smooth(pts, fill, style=LINE, closed=True):
    body = " ".join(P(x, y) for x, y in pts)
    cyc = "cycle" if closed else ""
    return f"\\filldraw[fill={name(fill)}, {style}] plot[smooth{' cycle' if closed else ''}, tension=0.6] coordinates {{{body}}};"


def curve(pts, style=THIN, color=None):
    c = f", draw={name(color)}" if color else ""
    return f"\\draw[{style}{c}] plot[smooth, tension=0.6] coordinates {{{' '.join(P(x, y) for x, y in pts)}}};"


def line(*pts, style=THIN, color=None):
    c = f", draw={name(color)}" if color else ""
    return f"\\draw[{style}{c}] " + " -- ".join(P(x, y) for x, y in pts) + ";"


def circ(x, y, r, fill, style=LINE):
    return f"\\filldraw[fill={name(fill)}, {style}] {P(x, y)} circle ({f(r)});"


def ell(x, y, rx, ry, fill, style=LINE, rot=0):
    return f"\\filldraw[fill={name(fill)}, {style}, rotate around={{{rot}:{P(x, y)}}}] {P(x, y)} ellipse ({f(rx)} and {f(ry)});"


def leaf(x, y, l, ang, fill="leaf"):
    a = math.radians(ang)
    tip = (x + l * math.cos(a), y + l * math.sin(a))
    nx, ny = -math.sin(a) * l * 0.32, math.cos(a) * l * 0.32
    mid = (x + l * 0.5 * math.cos(a), y + l * 0.5 * math.sin(a))
    return [f"\\filldraw[fill={name(fill)}, {THIN}] {P(x, y)} .. controls {P(mid[0] + nx, mid[1] + ny)} .. {P(*tip)} .. controls {P(mid[0] - nx, mid[1] - ny)} .. cycle;",
            line((x, y), (mid[0] + (tip[0] - mid[0]) * 0.6, mid[1] + (tip[1] - mid[1]) * 0.6), style=f"line width={LW * 0.35}cm", color="leaf_dark")]


def plate_points(p):
    return [tuple(xy) for xy in DATA["plates"][p]["points"]]


def rivets(pts, color="steel_dark"):
    return [f"\\fill[{name(color)}] {P(x, y)} circle (0.7);" for x, y in pts]


# --- heads: over the head, which runs from (0,151) to (0,195), 11 cm about
# the line x = 0; the face is at +x ---------------------------------------------

def head_piece(piece, main, trim):
    o = []
    if piece == "kasa":
        o.append(poly([(-25, 185), (0, 204), (25, 185), (22, 183), (-22, 183)], "straw"))
        for k in range(-3, 4):
            o.append(line((k * 6.5, 184), (0, 202), color="straw_dark"))
        o.append(line((6, 184), (7, 166), color="ink"))
    elif piece in ("helmet", "general"):
        pts = plate_points("helmet")
        # The neck guard flares behind; the bowl is the plate's own line.
        o.append(poly([(-14, 178), (-24, 166), (-21, 160), (-12, 168)], main))
        o.append(line((-15, 172), (-23, 163), color="ink"))
        o.append(poly(pts + [(12, 176), (-12, 176)], "steel"))
        o.append(poly([(-15, 172), (-13, 197), (13, 197), (15, 172), (15, 176), (-15, 176)], "steel"))
        o.append(line((-14, 186), (14, 186), color="steel_dark"))
        o += rivets([(-9, 192), (0, 192), (9, 192), (-9, 181), (9, 181)])
        o.append(poly([(12, 176), (17, 172), (17, 170), (12, 172)], "steel_dark", THIN))
        if piece == "general":
            # A crescent crest and a plume: the general (Sam).
            o.append(f"\\filldraw[fill=gold, {LINE}] {P(4, 197)} .. controls {P(14, 214)} .. {P(27, 220)} .. controls {P(16, 209)} .. {P(8, 197)} -- cycle;")
            o.append(f"\\filldraw[fill=gold, {LINE}] {P(-4, 197)} .. controls {P(-8, 214)} .. {P(-2, 226)} .. controls {P(-2, 210)} .. {P(0, 197)} -- cycle;")
            o.append(smooth([(-10, 197), (-20, 206), (-30, 202), (-26, 194), (-14, 196)], trim, THIN))
        else:
            o.append(f"\\filldraw[fill=gold, {THIN}] {P(10, 197)} .. controls {P(14, 205)} .. {P(20, 207)} .. controls {P(15, 202)} .. {P(13, 197)} -- cycle;")
    elif piece == "skullcap":
        # A steel cap over the crown only, on the plate's points: the
        # gatekeeper's. A full helmet made the fight three times harder for
        # the yardstick (analysis/armor.md); the cap kept it as it was.
        pts = plate_points("skullcap")
        o.append(poly(pts + [(10, 188), (-10, 188)], "steel"))
        o.append(poly([(-12.5, 187), (12.5, 187), (12.5, 190.5), (-12.5, 190.5)], main, THIN))
        o += rivets([(-5, 194), (0, 195.5), (5, 194)])
        o.append(poly([(-1, 198), (1, 198), (0.5, 203), (-0.5, 203)], "gold", THIN))
    elif piece == "sombrero":
        # A broad steel hat on the plate's three points (Sam: "a knight with a
        # big sombrero type steel helmet").
        a, b, c = plate_points("sombrero")
        o.append(poly([a, (-14, 189.5), (-11, 199), (-6, 202.5), (6, 202.5), (11, 199), (14, 189.5), c, (26, 183), (-26, 183)], "steel"))
        o.append(f"\\filldraw[fill=steellight, {THIN}] {P(-11, 190)} .. controls {P(-10, 201)} .. {P(0, 203)} .. controls {P(10, 201)} .. {P(11, 190)} -- cycle;")
        o.append(line((-26, 184.5), (26, 184.5), color="steel_dark"))
        o += rivets([(-20, 185.6), (-10, 186.5), (10, 186.5), (20, 185.6), (0, 196)])
        o.append(poly([(7, 186), (14, 186), (14, 168), (9, 166)], "steel", THIN))
        for y in (182, 177, 172):
            o.append(line((9, y), (14, y), color="ink"))
    elif piece == "pumpkin":
        o.append(ell(0, 179, 16, 15, "pumpkin"))
        for dx, rx in [(-8, 7), (8, 7), (0, 6)]:
            o.append(ell(dx, 179, rx, 14.6, "pumpkin", THIN))
        o.append(poly([(-1.5, 193), (1.5, 193), (3, 200), (0.5, 201)], "leaf_dark", THIN))
        o.append(curve([(1, 198), (7, 202), (10, 199)], color="leaf_dark"))
        o.append(poly([(7, 183), (12, 183), (10, 188)], "ink", THIN))
        o.append(poly([(4, 173), (14, 173), (13, 169), (11, 171), (9, 168), (7, 171), (5, 169)], "ink", THIN))
    elif piece == "horns":
        o.append(f"\\filldraw[fill=bone, {LINE}] {P(1, 189)} .. controls {P(16, 192)} .. {P(27, 212)} .. controls {P(14, 205)} .. {P(8, 197)} -- cycle;")
        o.append(f"\\filldraw[fill=bonedark, {LINE}] {P(-2, 189)} .. controls {P(-15, 192)} .. {P(-23, 209)} .. controls {P(-13, 204)} .. {P(-8, 197)} -- cycle;")
        o.append(ell(-9, 187, 5, 2.6, main, THIN, 20))
        for t in (0.35, 0.55):
            o.append(line((4 + 23 * t, 190 + 22 * t - 3), (6 + 23 * t, 190 + 22 * t + 1), color="bone_dark"))
    elif piece == "leaves":
        for k in range(9):
            a = 200 - k * 25
            x, y = 10 * math.cos(math.radians(a)), 185 + 9 * math.sin(math.radians(a))
            o += leaf(x, y, 9, a + 70, "leaf" if k % 2 else "leaf_dark")
    elif piece == "headband":
        o.append(poly([(-11.5, 185), (11.5, 187), (11.5, 191), (-11.5, 189)], main, THIN))
        o.append(smooth([(-11, 188), (-19, 185), (-23, 179), (-17, 182)], main, THIN))
        o.append(smooth([(-11, 188), (-20, 193), (-24, 190), (-17, 189)], main, THIN))
        o.append(circ(-11.5, 188, 2, trim, THIN))
    elif piece == "topknot":
        o.append(f"\\filldraw[fill=black, {LINE}] {P(-11, 184)} .. controls {P(-11, 197)} .. {P(0, 196)} .. controls {P(9, 196)} .. {P(10, 189)} -- {P(-11, 184)};")
        o.append(ell(-3, 199, 3.5, 3, "black", THIN))
        o.append(poly([(-6, 196), (-6, 205), (-1, 204), (-1, 196)], "black", THIN))
        o.append(line((-7, 199), (0, 199), color=trim))
    elif piece == "cap":
        o.append(f"\\filldraw[fill={name(main)}, {LINE}] {P(-12, 186)} .. controls {P(-12, 201)} .. {P(2, 200)} .. controls {P(12, 199)} .. {P(12, 188)} -- cycle;")
        o.append(poly([(8, 187), (19, 186), (19, 184.5), (8, 185)], trim, THIN))
        o.append(circ(-2, 200, 1.6, trim, THIN))
    elif piece == "eboshi":
        o.append(f"\\filldraw[fill=black, {LINE}] {P(-10, 187)} .. controls {P(-12, 205)} .. {P(-6, 216)} .. controls {P(0, 220)} .. {P(4, 214)} .. controls {P(7, 204)} .. {P(9, 188)} -- cycle;")
        o.append(line((-10, 189), (9, 190), color=trim))
        o.append(line((8, 189), (9, 165), color=trim))
    elif piece == "scarf":
        o.append(f"\\filldraw[fill={name(main)}, {LINE}] {P(-12.5, 172)} .. controls {P(-13, 199)} .. {P(0, 198)} .. controls {P(11, 198)} .. {P(12, 187)} -- {P(4, 189)} .. controls {P(-4, 188)} .. {P(-6, 175)} -- cycle;")
        o.append(smooth([(-12, 176), (-20, 170), (-22, 162), (-16, 168)], main, THIN))
        for x, y in [(-6, 193), (2, 195), (-10, 184), (6, 192)]:
            o.append(f"\\fill[{name(trim)}] {P(x, y)} circle (1.1);")
    elif piece == "hood":
        o.append(f"\\filldraw[fill={name(main)}, {LINE}] {P(-15, 152)} .. controls {P(-17, 190)} .. {P(-6, 200)} .. controls {P(6, 203)} .. {P(13, 190)} .. controls {P(6, 186)} .. {P(4, 172)} .. controls {P(4, 162)} .. {P(10, 155)} -- cycle;")
        o.append(curve([(13, 190), (6, 186), (4, 172), (4, 162), (10, 155)], color=trim))
    elif piece == "mitre":
        # The abbot's tall cap over a hood (Sam: "the abbot an abbot outfit").
        o += head_piece("hood", main, trim)
        o.append(f"\\filldraw[fill={name(trim)}, {LINE}] {P(-9, 195)} .. controls {P(-10, 214)} .. {P(0, 222)} .. controls {P(9, 214)} .. {P(9, 196)} -- cycle;")
        o.append(line((0, 197), (0, 220), color="gold"))
        o.append(poly([(-9.5, 194), (9.5, 195.5), (9.5, 199), (-9.5, 197.5)], "gold", THIN))
    elif piece == "pilgrim":
        # A tall pointed hood leaning back, its veil over the face, a ragged
        # hem at the shoulders (Sam: "dressed like a pilgrim from elden ring").
        o.append(f"\\filldraw[fill={name(main)}, {LINE}] {P(-17, 150)} -- {P(-14, 154)} -- {P(-16, 158)} .. controls {P(-16, 195)} .. {P(-14, 232)} .. controls {P(4, 214)} .. {P(13, 196)} -- {P(14, 168)} -- {P(11, 160)} -- {P(13, 154)} -- {P(8, 150)} -- cycle;")
        o.append(curve([(-14, 230), (-4, 214), (6, 200)], color=trim))
        o.append(poly([(5, 192), (14, 194), (15, 162), (12, 158), (9, 162), (6, 158)], trim, THIN))
        o.append(line((8, 190), (9, 162), color="stone"))
        o.append(line((11, 191), (12, 162), color="stone"))
    elif piece == "feather":
        o += head_piece("cap", main, trim)
        o.append(f"\\filldraw[fill=bone, {THIN}] {P(-6, 199)} .. controls {P(-16, 214)} .. {P(-30, 218)} .. controls {P(-20, 208)} .. {P(-9, 197)} -- cycle;")
        o.append(curve([(-7, 198), (-17, 211), (-29, 217)], color="bone_dark"))
    elif piece == "halo":
        o.append(f"\\draw[draw=gold, line width=2.2cm] {P(-6, 190)} circle (19);")
        o.append(f"\\draw[draw=glowsoft, line width=0.8cm] {P(-6, 190)} circle (19);")
        for k in range(12):
            a = math.radians(k * 30)
            o.append(line((-6 + 22 * math.cos(a), 190 + 22 * math.sin(a)), (-6 + 27 * math.cos(a), 190 + 27 * math.sin(a)), color="gold"))
    elif piece == "sun":
        o.append(circ(-8, 188, 17, "gold"))
        for k in range(16):
            a = math.radians(k * 22.5)
            o.append(poly([(-8 + 17 * math.cos(a - 0.12), 188 + 17 * math.sin(a - 0.12)), (-8 + 24 * math.cos(a), 188 + 24 * math.sin(a)), (-8 + 17 * math.cos(a + 0.12), 188 + 17 * math.sin(a + 0.12))], "gold", THIN))
        o.append(circ(-8, 188, 11, "saffron", THIN))
    elif piece == "lion":
        # The lion dance's head: a mane round a painted face, jaw open.
        for k in range(14):
            a = math.radians(k * 360 / 14)
            o.append(circ(-2 + 15 * math.cos(a), 182 + 15 * math.sin(a), 6, trim, THIN))
        o.append(ell(-1, 182, 15, 14, main))
        o.append(poly([(8, 172), (20, 168), (19, 163), (9, 165)], main, LINE))
        o.append(poly([(9, 176), (21, 178), (20, 172), (10, 172)], "white", THIN))
        o.append(circ(6, 189, 3.6, "white", THIN))
        o.append(circ(7, 189, 1.6, "ink", THIN))
        o.append(poly([(-3, 195), (1, 205), (4, 195)], main, THIN))
        o.append(curve([(-6, 184), (0, 182), (6, 181)], color=trim))
    return o


# --- chests: over the chest, (0,100) to (0,166), 14 cm about x = 0 --------------

def torso(fill, style=LINE, r=15, top=167, bottom=100):
    return f"\\filldraw[fill={name(fill)}, {style}] {P(-r, bottom + r)} -- {P(-r, top - r)} arc (180:0:{r}) -- {P(r, bottom + r)} arc (0:-180:{r}) -- cycle;"


def chest_piece(piece, main, trim):
    o = []
    if piece == "robe":
        o.append(torso(main))
        o.append(poly([(2, 165), (9, 158), (12, 140), (8, 142)], trim, THIN))
        o.append(line((2, 165), (12, 122), color="ink"))
        o.append(poly([(-15, 112), (15, 112), (15, 122), (-15, 122)], trim, THIN))
    elif piece == "vest":
        o.append(torso(trim))
        o.append(f"\\filldraw[fill={name(main)}, {LINE}] {P(-15, 150)} arc (180:90:15) -- {P(5, 165)} -- {P(7, 108)} -- {P(-15, 108)} -- cycle;")
        o.append(poly([(10, 164), (14, 154), (14, 110), (11, 108)], main, THIN))
        for y in (150, 138, 126):
            o.append(f"\\fill[ink] {P(9, y)} circle (0.9);")
    elif piece == "apron":
        o.append(torso(trim))
        o.append(poly([(0, 150), (17, 148), (19, 100), (2, 100)], main))
        o.append(line((0, 150), (-3, 166), color="ink"))
        o.append(line((-15, 122), (2, 124), color="ink"))
        o.append(poly([(6, 128), (14, 128), (14, 118), (6, 118)], main, THIN))
    elif piece == "plate":
        # Lamellar rows laced in the trim color, and a shoulder guard.
        o.append(torso(main))
        for y in range(108, 160, 8):
            o.append(f"\\filldraw[fill=steel, {THIN}] {P(-14.5, y)} rectangle {P(14.5, y + 6)};")
            for x in range(-11, 14, 5):
                o.append(line((x, y + 1), (x, y + 5), color=trim))
        o.append(poly([(-12, 160), (4, 160), (6, 140), (-10, 138)], main, LINE))
        for y in (155, 149, 143):
            o.append(line((-11, y), (5, y), color=trim))
    elif piece == "monk":
        o.append(torso(main))
        o.append(poly([(-14, 156), (-6, 165), (14, 112), (6, 104)], trim))
        for k in range(8):
            t = k / 7
            o.append(circ(4 + 6 * t, 160 - 26 * t + 8 * math.sin(t * 3.14), 1.7, "wood", THIN))
        o.append(poly([(-15, 112), (15, 112), (15, 118), (-15, 118)], "brown", THIN))
    elif piece == "sash":
        o.append(torso(main))
        o.append(poly([(-14, 158), (-9, 164), (15, 116), (10, 108)], trim))
        o.append(poly([(-15, 112), (15, 112), (15, 118), (-15, 118)], trim, THIN))
    elif piece == "leaves":
        o.append(torso("leaf_dark", THIN))
        for y in range(104, 166, 9):
            for x in range(-12, 15, 8):
                o += leaf(x - 3, y, 11, -60 + (x * 3), "leaf" if (x + y) % 2 else "leaf_dark")
    elif piece == "mino":
        o.append(torso("straw_dark", THIN))
        o.append(poly([(-17, 166), (17, 166), (20, 98), (14, 102), (10, 96), (4, 101), (-2, 95), (-8, 101), (-14, 96), (-19, 100)], main))
        for x in range(-15, 19, 4):
            o.append(line((x * 0.85, 164), (x * 1.1, 101), color="straw_dark"))
    elif piece == "pilgrim":
        o.append(poly([(-17, 166), (14, 166), (18, 98), (12, 102), (8, 96), (2, 101), (-4, 96), (-10, 101), (-16, 95), (-20, 99)], main))
        for x in (-10, -3, 4, 10):
            o.append(line((x * 0.9, 162), (x * 1.15, 100), color="stone"))
        o.append(line((-15, 160), (12, 110), style=f"line width={LW * 1.4}cm", color="wood"))
        o.append(poly([(-24, 128), (-12, 130), (-12, 112), (-23, 110)], "wood"))
        o.append(line((-16, 120), (16, 118), style=f"line width={LW * 1.2}cm", color="straw_dark"))
    elif piece == "glow_robe":
        o.append(torso(main))
        o.append(poly([(2, 165), (9, 158), (12, 130), (8, 132)], trim, THIN))
        o.append(f"\\draw[draw=glow, line width={LW * 1.1}cm] {P(-10, 150)} .. controls {P(0, 158)} .. {P(6, 140)} .. controls {P(10, 128)} .. {P(-4, 120)};")
        o.append(poly([(-15, 112), (15, 112), (15, 120), (-15, 120)], "gold", THIN))
    return o


# --- waists: (0,81) to (0,127), and the skirt over the thighs below -------------

def waist_piece(piece, main, trim):
    o = []
    if piece == "skirt":
        o.append(poly([(-14, 118), (15, 118), (21, 72), (-19, 72)], main))
        for x in (-9, -2, 5, 12):
            o.append(line((x * 0.95, 116), (x * 1.35, 73), color=trim))
    elif piece == "plate_skirt":
        for k, x in enumerate((-16, -6, 4, 14)):
            o.append(poly([(x - 1, 116), (x + 9, 116), (x + 10, 82), (x - 2, 82)], main, THIN))
            for y in (106, 96, 86):
                o.append(f"\\filldraw[fill=steel, {THIN}] {P(x - 1.5, y)} rectangle {P(x + 9.5, y + 7)};")
        o.append(poly([(-15, 112), (15, 112), (15, 120), (-15, 120)], trim, THIN))
    elif piece == "mino":
        o.append(poly([(-15, 120), (16, 120), (21, 74), (15, 79), (10, 72), (4, 78), (-2, 71), (-8, 78), (-14, 72), (-20, 77)], main))
        for x in range(-14, 18, 4):
            o.append(line((x, 118), (x * 1.3, 77), color="straw_dark"))
    elif piece == "leaves":
        for k in range(8):
            x = -15 + k * 4.4
            o += leaf(x, 117, 30, -100 + k * 3, "leaf" if k % 2 else "leaf_dark")
    elif piece == "pilgrim":
        o.append(poly([(-16, 120), (15, 120), (22, 66), (16, 70), (10, 64), (4, 70), (-3, 64), (-9, 70), (-15, 64), (-21, 69)], main))
        for x in (-10, -3, 4, 11):
            o.append(line((x, 118), (x * 1.3, 68), color="stone"))
    return o


PIECES = {"head": head_piece, "chest": chest_piece, "waist": waist_piece}


def render(cid, slot):
    c = DATA["costumes"][cid]
    piece = c[slot]
    if piece == "none":
        return False
    x0, y0, x1, y1 = DATA["slots"][slot]["box"]
    body = PIECES[slot](piece, c["main"], c["trim"])
    if not body:
        sys.exit(f"no {slot} piece named {piece} ({cid})")
    tex = f"""\\documentclass[tikz,border=0pt]{{standalone}}
\\usepackage{{tikz}}
{colors()}
\\begin{{document}}
\\begin{{tikzpicture}}[x=1cm, y=1cm]
\\useasboundingbox ({x0},{y0}) rectangle ({x1},{y1});
{chr(10).join(body)}
\\end{{tikzpicture}}
\\end{{document}}
"""
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as d:
        Path(d, "f.tex").write_text(tex)
        rr = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "f.tex"], cwd=d, capture_output=True, text=True)
        if rr.returncode:
            sys.exit(f"pdflatex failed for {cid}-{slot}:\n" + "\n".join(l for l in rr.stdout.splitlines() if l.startswith("!") or l.startswith("l.")))
        subprocess.run(["pdftocairo", "-png", "-transp", "-singlefile", "-scale-to-x", str((x1 - x0) * PX), "-scale-to-y", "-1", "f.pdf", str(OUT / f"{cid}-{slot}")], cwd=d, check=True)
    return True


if __name__ == "__main__":
    ids = sys.argv[1:] or sorted(DATA["costumes"])
    for cid in ids:
        drawn = [s for s in PIECES if render(cid, s)]
        print("drew", cid, " ".join(drawn))
