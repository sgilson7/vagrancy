"""Each opponent's signature piece (Sam, 2026-10-08: "a lot of them are wearing
roughly the same japanese peasant type garb ... give enemy a signature
piece"): one thing from the opponent's trade or place (copy
`opponents.<id>.place`), worn so the opponent can be told by outline.

`SIG[name] = (slot, draw)`: the slot's picture it is drawn into (head, chest
or waist, in the fighter's rest-pose cm, facing +x, so the back is -x), and a
function of the costumes module's helpers and the costume's two colors.
data/costumes.json names each opponent's piece in `sig`.
"""
import math


def _bag(h, x, y, w, hgt, fill, strap=None):
    o = []
    if strap:
        o.append(h.line(*strap, style=f"line width={h.LW * 1.2}cm", color="brown_dark"))
    o.append(h.smooth([(x - w / 2, y), (x - w / 2 - 1, y + hgt * 0.6), (x - w / 3, y + hgt), (x + w / 3, y + hgt), (x + w / 2 + 1, y + hgt * 0.6), (x + w / 2, y)], fill))
    return o


def _pole(h, a, b, fill="wood", w=1.4):
    return [h.line(a, b, style=f"line width={h.LW * w * 1.8}cm", color="ink"), h.line(a, b, style=f"line width={h.LW * w}cm", color=fill)]


def _strap(h):
    return h.line((-12, 160), (12, 112), style=f"line width={h.LW * 1.3}cm", color="brown_dark")


# --- on the back and shoulders (chest) -------------------------------------------

def crow_on_shoulder(h, m, t):
    return [h.ell(-6, 172, 7, 4.5, "black", h.LINE, 15), h.circ(0, 176, 3.4, "black"),
            h.poly([(3, 177), (8, 176), (3, 174.5)], "gold", h.THIN), h.poly([(-12, 171), (-19, 167), (-12, 168)], "black", h.THIN),
            h.line((-6, 168), (-5, 164)), h.line((-3, 168), (-2, 164))]


def flail(h, m, t):
    return _pole(h, (-22, 108), (-8, 172)) + _pole(h, (-8, 172), (6, 180), "straw_dark") + [_strap(h)]


def rope_coil(h, m, t):
    o = []
    for k in range(4):
        o.append(f"\\draw[draw=ink, line width={h.LW * 2.2}cm] {h.P(-4, 150)} ellipse ({10 - k * 0.6} and {14 - k * 0.6});")
        o.append(f"\\draw[draw=straw, line width={h.LW * 1.3}cm] {h.P(-4, 150)} ellipse ({10 - k * 0.6} and {14 - k * 0.6});")
    return o


def vault_pole(h, m, t):
    return _pole(h, (-30, 96), (2, 178), "bone") + [_strap(h)]


def crook(h, m, t):
    return _pole(h, (-24, 100), (-16, 176)) + [f"\\draw[draw=ink, line width={h.LW * 2.5}cm] {h.P(-16, 176)} arc (180:0:5);",
                                                f"\\draw[draw=wood, line width={h.LW * 1.4}cm] {h.P(-16, 176)} arc (180:0:5);"]


def parcel(h, m, t):
    return [_strap(h), h.poly([(-30, 118), (-12, 118), (-12, 152), (-30, 152)], "stone"), h.line((-30, 135), (-12, 135), color="brown_dark"),
            h.line((-21, 118), (-21, 152), color="brown_dark"), h.circ(-21, 135, 1.6, "brown_dark", h.THIN)]


def salt_sack(h, m, t):
    return [_strap(h)] + _bag(h, -22, 116, 16, 34, "white") + [h.poly([(-25, 150), (-19, 150), (-18, 154), (-26, 154)], "white", h.THIN), h.line((-26, 151), (-18, 151), color="brown")]


def falcon(h, m, t):
    return [h.poly([(-14, 148), (-2, 150), (-2, 153), (-14, 152)], "brown_dark", h.THIN),
            h.ell(-9, 162, 5.5, 8, "stone", h.LINE, -10), h.circ(-7, 171, 3.6, "stone"), h.poly([(-4, 172), (0, 170), (-4, 169)], "gold", h.THIN),
            h.poly([(-12, 160), (-20, 150), (-11, 154)], "brown", h.THIN), h.line((-8, 166), (-6, 168), color="white")]


def oar(h, m, t):
    return _pole(h, (-26, 104), (-4, 178)) + [h.ell(-28, 100, 3.5, 9, "wood", h.LINE, 30), _strap(h)]


def map_quiver(h, m, t):
    o = [_strap(h), h.poly([(-28, 110), (-18, 106), (-10, 152), (-20, 156)], "brown")]
    for k, c in enumerate(("cream", "white", "cream")):
        o.append(h.ell(-17 + k * 3.2, 157 + k * 1.2, 2.4, 1.6, c, h.THIN, 70))
        o.append(h.poly([(-19 + k * 3.2, 152 + k * 1.2), (-15.5 + k * 3.2, 152.5 + k * 1.2), (-14 + k * 3.2, 160 + k * 1.2), (-17.5 + k * 3.2, 159 + k * 1.2)], c, h.THIN))
    return o


def seal_medallion(h, m, t):
    return [h.curve([(-2, 164), (6, 150), (10, 140)], style=f"line width={h.LW}cm", color="gold"), h.circ(10, 138, 4.5, "gold"),
            h.poly([(8, 136.5), (12, 136.5), (12, 139.5), (8, 139.5)], "plum", h.THIN)]


def crozier(h, m, t):
    return _pole(h, (-22, 98), (-14, 190), "gold", 1.2) + [f"\\draw[draw=ink, line width={h.LW * 2.3}cm] {h.P(-14, 190)} arc (180:-60:5);",
                                                         f"\\draw[draw=gold, line width={h.LW * 1.2}cm] {h.P(-14, 190)} arc (180:-60:5);"]


def pelt(h, m, t):
    return [h.smooth([(-16, 166), (-6, 172), (8, 168), (12, 156), (4, 160), (-6, 152), (-16, 140), (-22, 132), (-20, 150)], "bone_dark"),
            h.poly([(-22, 132), (-26, 124), (-19, 128)], "bone_dark", h.THIN), h.poly([(12, 156), (16, 148), (10, 152)], "bone_dark", h.THIN)]


def pots_and_pans(h, m, t):
    return [_strap(h), h.circ(-24, 140, 7, "steel"), h.line((-24, 147), (-24, 158), style=f"line width={h.LW * 1.6}cm", color="ink"),
            h.ell(-20, 122, 7, 5, "steel_dark"), h.circ(-29, 126, 4, "pumpkin_dark"), h.line((-18, 160), (-30, 112), color="ink")]


def walking_staff(h, m, t):
    return _pole(h, (-26, 92), (-10, 196)) + [h.circ(-9, 193, 3, "gold", h.THIN), h.line((-10, 196), (-6, 200), color="ink"),
                                              h.ell(-12, 186, 2, 3, "white", h.THIN)]


def whetstone(h, m, t):
    return [_strap(h), h.circ(-22, 136, 13, "stone"), h.circ(-22, 136, 9, "slate", h.THIN), h.circ(-22, 136, 2.4, "wood", h.THIN)]


def neck_bell(h, m, t):
    return [h.curve([(-6, 164), (2, 160), (9, 160)], style=f"line width={h.LW * 1.2}cm", color="brown_dark"),
            h.poly([(7, 158), (13, 158), (14.5, 150), (5.5, 150)], "gold"), h.circ(10, 149, 1.4, "ink", h.THIN)]


def eel_trap(h, m, t):
    o = [_strap(h), h.smooth([(-30, 112), (-32, 130), (-29, 154), (-17, 156), (-14, 132), (-17, 112)], "straw")]
    for y in (118, 126, 134, 142, 150):
        o.append(h.curve([(-30, y), (-23, y + 2), (-16, y)], color="straw_dark"))
    return o


def carrier(h, m, t):
    return _pole(h, (-16, 100), (-16, 176), "wood", 1.1) + _pole(h, (-30, 100), (-30, 176), "wood", 1.1) + [
        h.poly([(-31, 112), (-15, 112), (-15, 130), (-31, 130)], "wood"), h.poly([(-29, 131), (-17, 131), (-17, 147), (-29, 147)], "stone"),
        h.poly([(-30, 148), (-16, 148), (-16, 160), (-30, 160)], "peach_dark"), _strap(h)]


def paper_frame(h, m, t):
    return [_strap(h), h.poly([(-34, 112), (-14, 112), (-14, 160), (-34, 160)], "white"),
            h.poly([(-34, 112), (-14, 112), (-14, 160), (-34, 160)], "white", f"draw=wood, line width={h.LW * 1.6}cm"),
            h.line((-24, 114), (-24, 158), color="stone"), h.line((-33, 136), (-15, 136), color="stone")]


def war_banner(h, m, t):
    return _pole(h, (-16, 120), (-16, 222), "black", 1.1) + [h.poly([(-15.5, 220), (-15.5, 186), (-2, 186), (-2, 220)], t),
                                                             h.circ(-9, 203, 4.5, m, h.THIN)]


def roof_tiles(h, m, t):
    o = [_strap(h)]
    for k in range(5):
        o.append(f"\\filldraw[fill=slate, {h.LINE}] {h.P(-32, 112 + k * 8)} arc (180:0:8) -- {h.P(-16, 112 + k * 8 - 3)} arc (0:180:8) -- cycle;")
    return o


def charcoal_basket(h, m, t):
    o = [_strap(h), h.poly([(-33, 140), (-13, 140), (-15, 112), (-31, 112)], "straw")]
    for x, y in [(-29, 142), (-24, 144), (-19, 143), (-16, 141), (-27, 146), (-21, 147)]:
        o.append(h.circ(x, y, 2.8, "black", h.THIN))
    return o


def tea_basket(h, m, t):
    o = [_strap(h), h.poly([(-34, 152), (-12, 152), (-16, 112), (-30, 112)], "straw")]
    for k in range(5):
        o += h.leaf(-31 + k * 4.5, 151, 7, 80 - k * 8, "leaf")
    o.append(h.line((-32, 132), (-14, 132), color="straw_dark"))
    return o


def cape(h, m, t):
    return [h.poly([(-12, 164), (-30, 104), (-22, 100), (-14, 104), (-6, 158)], t)]


def kite(h, m, t):
    return [h.poly([(-22, 186), (-8, 152), (-22, 114), (-36, 152)], "sky"), h.line((-22, 186), (-22, 114), color="ink"),
            h.line((-36, 152), (-8, 152), color="ink"), h.poly([(-22, 186), (-8, 152), (-22, 152)], m, h.THIN),
            h.curve([(-22, 114), (-26, 106), (-20, 100), (-25, 94)], color="ink")]


def drying_net(h, m, t):
    o = [h.poly([(-16, 166), (14, 162), (16, 136), (-20, 128)], "cloud", h.THIN)]
    for k in range(6):
        o.append(h.line((-16 + k * 6, 166 - k * 0.8), (-20 + k * 7.2, 128 + k * 1.6), color="teal_dark"))
        o.append(h.line((-17 - k * 0.6, 160 - k * 6), (15, 160 - k * 4.4), color="teal_dark"))
    return o


def axe(h, m, t):
    return _pole(h, (-26, 104), (-8, 172)) + [h.poly([(-10, 166), (-2, 176), (2, 170), (-4, 162)], "steel"), _strap(h)]


def harpoon(h, m, t):
    return _pole(h, (-30, 96), (2, 190)) + [h.poly([(2, 190), (5, 200), (0, 196), (-3, 199)], "steel")]


def reed_bundle(h, m, t):
    o = [_strap(h)]
    for k in range(7):
        o.append(h.line((-30 + k * 2, 100), (-24 + k * 2.4, 186), style=f"line width={h.LW * 0.9}cm", color="sage_dark" if k % 2 else "olive"))
    o.append(h.poly([(-30, 140), (-14, 140), (-14, 145), (-30, 145)], "straw_dark", h.THIN))
    return o


def fleece(h, m, t):
    o = []
    for k in range(9):
        a = math.radians(200 - k * 22)
        o.append(h.circ(-2 + 15 * math.cos(a), 156 + 9 * math.sin(a), 5, "cloud", h.THIN))
    return o


def prayer_flags(h, m, t):
    o = _pole(h, (-24, 100), (-24, 200), "wood", 1.0)
    cols = ["sky", "white", "pumpkin", "leaf", "saffron"]
    for k, c in enumerate(cols):
        x0 = -24 + k * 0.3
        y0 = 198 - k * 9
        o.append(h.poly([(x0, y0), (x0 - 9, y0 - 1), (x0 - 9, y0 - 7), (x0, y0 - 7)], c, h.THIN))
    return o


def jewels(h, m, t):
    o = [h.curve([(-6, 164), (4, 150), (13, 150)], style=f"line width={h.LW}cm", color="gold")]
    for k in range(5):
        tt = k / 4
        o.append(h.circ(-6 + 18 * tt, 164 - 16 * tt + 4 * math.sin(tt * 3.1), 2.2, ["teal", "glow", "sky", "glow", "teal"][k], h.THIN))
    return o


def iron_hoop(h, m, t):
    return [_strap(h), f"\\draw[draw=ink, line width={h.LW * 3}cm] {h.P(-22, 136)} circle (15);",
            f"\\draw[draw=steel, line width={h.LW * 1.6}cm] {h.P(-22, 136)} circle (15);"]


def coin_medallion(h, m, t):
    return [h.curve([(-6, 164), (5, 150), (11, 142)], style=f"line width={h.LW}cm", color="brown_dark"), h.circ(11, 137, 6, "gold"),
            h.poly([(9.5, 135.5), (12.5, 135.5), (12.5, 138.5), (9.5, 138.5)], "white", h.THIN)]


def cart_wheel(h, m, t):
    o = [_strap(h), f"\\draw[draw=ink, line width={h.LW * 2.6}cm] {h.P(-22, 136)} circle (14);",
         f"\\draw[draw=wood, line width={h.LW * 1.5}cm] {h.P(-22, 136)} circle (14);"]
    for k in range(6):
        a = math.radians(k * 30)
        o.append(h.line((-22 + 13 * math.cos(a), 136 + 13 * math.sin(a)), (-22 - 13 * math.cos(a), 136 - 13 * math.sin(a)), color="brown"))
    o.append(h.circ(-22, 136, 3, "brown_dark", h.THIN))
    return o


def quoits(h, m, t):
    o = [h.line((-4, 164), (14, 112), style=f"line width={h.LW * 1.1}cm", color="straw_dark")]
    for k, c in enumerate(("teal", "pumpkin", "gold")):
        o.append(f"\\draw[draw={h.name(c)}, line width={h.LW * 1.6}cm] {h.P(2 + k * 4, 148 - k * 10)} circle (4.5);")
    return o


def ray_collar(h, m, t):
    o = []
    for k in range(9):
        a = math.radians(180 - k * 22.5)
        o.append(h.poly([(14 * math.cos(a) * 0.9, 160 + 6 * math.sin(a)), (22 * math.cos(a), 160 + 12 * math.sin(a) + 2), (14 * math.cos(a - 0.2) * 0.9, 160 + 6 * math.sin(a - 0.2))], "gold", h.THIN))
    return o


def wheat_bundle(h, m, t):
    o = [_strap(h)]
    for k in range(7):
        x = -30 + k * 2.4
        o.append(h.line((x, 104), (x + 3, 176), color="straw_dark"))
        o.append(h.ell(x + 3, 179, 1.6, 4, "gold", h.THIN))
    o.append(h.poly([(-31, 134), (-13, 134), (-13, 139), (-31, 139)], "brown", h.THIN))
    return o


def ladder(h, m, t):
    o = _pole(h, (-30, 92), (-24, 192), "wood", 1.1) + _pole(h, (-18, 92), (-12, 192), "wood", 1.1)
    for k in range(7):
        y = 100 + k * 13
        o.append(h.line((-30 + (y - 92) * 0.06, y), (-18 + (y - 92) * 0.06, y), style=f"line width={h.LW * 1.2}cm", color="brown"))
    return o


def spade(h, m, t):
    return _pole(h, (-24, 140), (-14, 196)) + [h.poly([(-28, 140), (-20, 140), (-19, 120), (-24, 114), (-29, 120)], "steel"), _strap(h)]


def mane_drape(h, m, t):
    o = []
    for k in range(7):
        o.append(h.poly([(-14 + k * 1, 166 - k * 7), (-26 - k * 0.5, 160 - k * 8), (-14 + k * 1, 160 - k * 7)], t if k % 2 else m, h.THIN))
    return o


def straw_cape(h, m, t):
    o = [h.poly([(-17, 168), (15, 168), (19, 140), (12, 143), (6, 138), (0, 143), (-6, 138), (-12, 143), (-19, 139)], "straw")]
    for x in range(-14, 16, 4):
        o.append(h.line((x, 166), (x * 1.15, 142), color="straw_dark"))
    return o


def silk_scarf(h, m, t):
    return [f"\\filldraw[fill={h.name(t)}, {h.THIN}] {h.P(-8, 166)} .. controls {h.P(-24, 168)} .. {h.P(-34, 156)} .. controls {h.P(-40, 150)} .. {h.P(-34, 140)} .. controls {h.P(-30, 152)} .. {h.P(-22, 158)} .. controls {h.P(-14, 160)} .. {h.P(-6, 160)} -- cycle;"]


def diamond_tunic(h, m, t):
    o = []
    for row in range(4):
        for col in range(3):
            x, y = -10 + col * 10 + (row % 2) * 5, 108 + row * 14
            if -14 <= x <= 14:
                o.append(h.poly([(x, y), (x + 4, y + 6), (x, y + 12), (x - 4, y + 6)], ["teal", "gold", "plum"][(row + col) % 3], h.THIN))
    return o


def stone_block(h, m, t):
    return [_strap(h), h.poly([(-34, 114), (-14, 114), (-14, 138), (-34, 138)], "stone"), h.line((-34, 126), (-14, 126), color="slate"),
            h.line((-24, 114), (-24, 126), color="slate"), h.line((-28, 126), (-28, 138), color="slate"), h.line((-34, 130), (-36, 110), color="brown_dark")]


def neck_towel(h, m, t):
    return [h.smooth([(-12, 166), (0, 170), (12, 166), (14, 160), (6, 162), (-6, 162), (-12, 160)], "white"),
            h.poly([(10, 162), (15, 162), (16, 144), (11, 144)], "white", h.THIN), h.line((11, 150), (16, 150), color="indigo"),
            h.line((11, 147), (16, 147), color="indigo")]


# --- at the belt (waist) -----------------------------------------------------------

def _belt_hang(h, x, items):
    return [h.line((x, 116), (x, 108), color="ink")] + items


def key_ring(h, m, t):
    o = [f"\\draw[draw=ink, line width={h.LW * 1.2}cm] {h.P(12, 110)} circle (3.4);"]
    for k, a in enumerate((-70, -95, -120)):
        r = math.radians(a)
        x, y = 12 + 3.4 * math.cos(r), 110 + 3.4 * math.sin(r)
        o += [h.line((x, y), (x + 7 * math.cos(r), y + 7 * math.sin(r)), style=f"line width={h.LW * 0.9}cm", color="gold"),
              h.circ(x, y, 1.2, "gold", h.THIN)]
    return o


def tea_set(h, m, t):
    return [h.line((10, 116), (12, 106), color="ink"), h.poly([(8, 106), (16, 106), (15, 99), (9, 99)], "white"),
            h.poly([(-16, 114), (-14, 114), (-13, 98), (-17, 98)], "straw_dark", h.THIN)] + [h.line((-17 + k, 98), (-18 + 2 * k, 92), color="straw_dark") for k in range(5)]


def lantern(h, m, t):
    return [h.line((12, 116), (16, 108), color="ink"), h.ell(17, 100, 5, 7, "glow"), h.poly([(13, 107), (21, 107), (21, 109), (13, 109)], "black", h.THIN),
            h.poly([(13, 93), (21, 93), (21, 91), (13, 91)], "black", h.THIN), h.line((14, 100), (20, 100), color="pumpkin_dark")]


def barrel_hoops(h, m, t):
    return [h.poly([(-15, 108), (15, 108), (16, 112), (-16, 112)], "steel_dark", h.THIN), h.poly([(-15, 118), (15, 118), (15, 122), (-15, 122)], "steel_dark", h.THIN),
            h.ell(16, 100, 3.5, 5.5, "wood", h.THIN), h.line((14, 108), (16, 105), color="ink")]


def smith_tools(h, m, t):
    return [h.line((12, 118), (10, 92), style=f"line width={h.LW * 1.4}cm", color="steel_dark"), h.line((15, 118), (14, 92), style=f"line width={h.LW * 1.4}cm", color="steel_dark"),
            h.poly([(-16, 115), (-13, 115), (-13, 96), (-16, 96)], "wood", h.THIN), h.poly([(-20, 96), (-9, 96), (-9, 90), (-20, 90)], "black", h.THIN)]


def signal_horn(h, m, t):
    return [f"\\filldraw[fill=bone, {h.LINE}] {h.P(10, 112)} .. controls {h.P(22, 108)} .. {h.P(24, 96)} -- {h.P(19, 96)} .. controls {h.P(18, 104)} .. {h.P(10, 108)} -- cycle;",
            h.line((10, 116), (12, 110), color="brown_dark")]


def skeins(h, m, t):
    return [h.ell(12 + k * 4, 102, 2, 8, c, h.THIN) for k, c in enumerate(("indigo", "teal", "indigo_light"))]


def carpenter_square(h, m, t):
    return [h.poly([(10, 118), (13, 118), (13, 90), (10, 90)], "steel"), h.poly([(10, 90), (24, 90), (24, 93), (10, 93)], "steel"),
            h.line((-14, 116), (-18, 104), style=f"line width={h.LW * 1.3}cm", color="gold")]


def spool(h, m, t):
    return [h.line((12, 116), (14, 108), color="ink"), h.poly([(10, 108), (20, 108), (20, 106), (10, 106)], "wood", h.THIN),
            h.poly([(11, 106), (19, 106), (19, 97), (11, 97)], t, h.THIN), h.poly([(10, 97), (20, 97), (20, 95), (10, 95)], "wood", h.THIN),
            h.curve([(19, 101), (24, 96), (22, 88)], color=t)]


def mug(h, m, t):
    return [h.line((12, 116), (14, 108), color="ink"), h.poly([(10, 108), (19, 108), (19, 96), (10, 96)], "wood"),
            f"\\draw[{h.THIN}] {h.P(19, 105)} arc (90:-90:3);", h.ell(14.5, 108.5, 4.5, 1.6, "cream", h.THIN)]


def herb_basket(h, m, t):
    o = [h.poly([(8, 110), (22, 110), (20, 96), (10, 96)], "straw")]
    for k in range(4):
        o += h.leaf(10 + k * 3.5, 109, 7, 70 + k * 12, "leaf")
    o.append(h.circ(17, 115, 2, "plum", h.THIN))
    o.append(f"\\draw[{h.THIN}] {h.P(9, 110)} .. controls {h.P(15, 122)} .. {h.P(21, 110)};")
    return o


def shackles(h, m, t):
    o = []
    for k in range(5):
        o.append(h.ell(12 + k * 1.2, 112 - k * 4, 1.6, 2.4, "steel_dark", h.THIN, 20 * (k % 2)))
    o.append(f"\\draw[draw=ink, line width={h.LW * 1.4}cm] {h.P(18, 88)} circle (3.5);")
    return o


def abacus(h, m, t):
    o = [h.poly([(8, 110), (24, 110), (24, 96), (8, 96)], "wood")]
    for y in (99, 103, 107):
        o.append(h.line((9, y), (23, y), color="ink"))
        for x in (11, 14, 19):
            o.append(h.circ(x, y, 1.2, "brown_dark", h.THIN))
    return o


def saddlebag(h, m, t):
    return [h.poly([(-28, 116), (-12, 116), (-12, 92), (-28, 94)], "brown"), h.poly([(-28, 116), (-12, 116), (-12, 108), (-28, 108)], "brown_dark", h.THIN),
            h.circ(-20, 108, 1.4, "gold", h.THIN)]


def dice(h, m, t):
    return [h.line((12, 116), (13, 104), color="ink"), h.poly([(9, 104), (16, 104), (16, 97), (9, 97)], "white"),
            h.poly([(15, 100), (21, 101), (20, 95), (14, 94)], "white"), h.circ(12.5, 100.5, 0.8, "ink", "draw=none"), h.circ(17.5, 98, 0.8, "ink", "draw=none")]


def alms_bowl(h, m, t):
    return [h.line((12, 116), (14, 106), color="ink"), f"\\filldraw[fill=black, {h.LINE}] {h.P(8, 106)} arc (180:360:7) -- cycle;"]


def second_scabbard(h, m, t):
    return [h.poly([(-14, 118), (-11, 120), (14, 92), (11, 90)], "black"), h.poly([(-17, 121), (-12, 123), (-10, 120), (-15, 118)], "gold", h.THIN)]


def bucket(h, m, t):
    return [h.curve([(12, 116), (16, 112), (16, 104)], color="straw_dark"), h.poly([(10, 104), (22, 104), (20, 90), (12, 90)], "wood"),
            h.line((11, 99), (21, 99), color="steel_dark"), f"\\draw[{h.THIN}] {h.P(10, 104)} .. controls {h.P(16, 112)} .. {h.P(22, 104)};"]


def paper_streamers(h, m, t):
    o = [h.poly([(-15, 114), (15, 114), (15, 119), (-15, 119)], "straw", h.THIN)]
    for x in (-10, 0, 10):
        o.append(h.poly([(x - 2, 114), (x + 2, 114), (x + 3, 108), (x - 1, 108), (x + 1, 102), (x - 3, 102), (x - 1, 96), (x - 4, 96)], "white", h.THIN))
    return o


def coin_string(h, m, t):
    o = [h.line((10, 114), (16, 92), color="brown_dark")]
    for k in range(6):
        o.append(f"\\filldraw[fill=gold, {h.THIN}] {h.P(10.5 + k, 112 - k * 3.6)} ellipse (2.6 and 1.2);")
    return o


def hare(h, m, t):
    return [h.line((12, 116), (14, 108), color="ink"), h.ell(15, 98, 3.5, 9, "stone", h.LINE, -8),
            h.ell(14, 108, 1.2, 4, "stone", h.THIN, 10), h.ell(17, 108, 1.2, 4, "stone", h.THIN, -10)]


def spyglass(h, m, t):
    return [h.poly([(8, 110), (24, 104), (25, 107), (9, 113)], "gold"), h.poly([(8, 110), (14, 108), (15, 111), (9, 113)], "brown", h.THIN)]


def hip_drum(h, m, t):
    return [h.line((-6, 120), (14, 108), color="brown_dark"), h.ell(16, 100, 7, 3, "bone"), h.poly([(9, 100), (23, 100), (23, 90), (9, 90)], "pumpkin_dark"),
            h.ell(16, 90, 7, 3, "bone", h.THIN), h.line((9, 100), (23, 90), color="white"), h.line((23, 100), (9, 90), color="white")]


def bangles(h, m, t):
    o = [h.line((10, 116), (12, 106), color="ink")]
    for k, c in enumerate(("gold", "teal", "gold", "sky", "gold")):
        o.append(f"\\draw[draw={h.name(c)}, line width={h.LW * 1.2}cm] {h.P(12, 103 - k * 3)} ellipse (4 and 1.6);")
    return o


def sickle(h, m, t):
    return [h.poly([(12, 118), (15, 118), (15, 104), (12, 104)], "wood", h.THIN),
            f"\\draw[draw=ink, line width={h.LW * 2.2}cm] {h.P(13, 104)} arc (180:330:7);", f"\\draw[draw=steel, line width={h.LW * 1.2}cm] {h.P(13, 104)} arc (180:330:7);"]


def pear_basket(h, m, t):
    o = [h.poly([(8, 108), (22, 108), (20, 96), (10, 96)], "straw")]
    for x in (11, 15, 19):
        o.append(h.ell(x, 110, 2.4, 3, "olive", h.THIN))
    return o


def clapper(h, m, t):
    return [h.line((12, 116), (14, 108), color="ink"), h.poly([(10, 108), (14, 108), (14, 92), (10, 92)], "wood", h.THIN),
            h.poly([(15, 108), (19, 108), (19, 92), (15, 92)], "wood", h.THIN)]


def grain_measure(h, m, t):
    return [h.line((12, 116), (14, 108), color="ink"), h.poly([(8, 108), (20, 108), (20, 96), (8, 96)], "wood"),
            h.ell(14, 108, 6, 1.8, "gold", h.THIN)]


def streamers(h, m, t):
    return [h.curve([(12, 114), (20, 104), (16, 92), (24, 82)], style=f"line width={h.LW * 1.6}cm", color=c) if k == 0 else
            h.curve([(-12, 114), (-20, 102), (-15, 90), (-24, 80)], style=f"line width={h.LW * 1.6}cm", color=c) for k, c in enumerate(("pumpkin", "sky"))]


def rope_belt(h, m, t):
    o = []
    for k in range(8):
        o.append(h.ell(-13 + k * 3.8, 116, 2.4, 3.2, "straw", h.THIN, 35))
    o.append(h.poly([(4, 113), (8, 113), (9, 100), (3, 100)], "white", h.THIN))
    return o


def fans(h, m, t):
    o = []
    for cx, ang in ((12, 70), (17, 95)):
        o.append(f"\\filldraw[fill={h.name(t)}, {h.THIN}] {h.P(cx, 104)} -- ++({ang - 30}:12) arc ({ang - 30}:{ang + 30}:12) -- cycle;")
    return o


def sheaf(h, m, t):
    o = [h.line((12, 116), (14, 106), color="ink")]
    for k in range(5):
        o.append(h.line((11 + k, 86), (12 + k * 0.6, 106), color="straw_dark"))
        o.append(h.ell(12 + k * 0.6, 108, 1.2, 3, "gold", h.THIN))
    o.append(h.poly([(10, 96), (17, 96), (17, 99), (10, 99)], "brown", h.THIN))
    return o


# --- on the head -------------------------------------------------------------------

def pinwheel(h, m, t):
    o = [h.line((-2, 196), (-2, 214), color="ink")]
    for k, c in enumerate(("sky", "pumpkin", "leaf", "gold")):
        a = k * 90
        o.append(f"\\filldraw[fill={h.name(c)}, {h.THIN}] {h.P(-2, 214)} -- ++({a}:6) -- ++({a + 135}:4.2) -- cycle;")
    o.append(h.circ(-2, 214, 1, "ink", "draw=none"))
    return o


def spectacles(h, m, t):
    return [f"\\draw[draw=ink, line width={h.LW * 0.8}cm] {h.P(9, 182)} circle (2.6);", h.line((1, 183), (6.6, 183), color="ink")]


def hood_bell(h, m, t):
    return [h.line((-4, 199), (-4, 203), color="ink"), h.poly([(-8, 203), (0, 203), (-1, 210), (-7, 210)], "gold"), h.circ(-4, 211, 1.6, "gold", h.THIN)]


def parasol_hat(h, m, t):
    return [h.line((0, 196), (0, 206), color="ink"), f"\\filldraw[fill={h.name(t)}, {h.LINE}] {h.P(-16, 205)} .. controls {h.P(-8, 216)} .. {h.P(0, 217)} .. controls {h.P(8, 216)} .. {h.P(16, 205)} -- cycle;",
            h.line((0, 217), (-8, 205.5), color="ink"), h.line((0, 217), (8, 205.5), color="ink")]


def quill(h, m, t):
    return [f"\\filldraw[fill=white, {h.THIN}] {h.P(-4, 182)} .. controls {h.P(-8, 196)} .. {h.P(-16, 206)} .. controls {h.P(-10, 194)} .. {h.P(-2, 182)} -- cycle;",
            h.line((-3, 182), (-14, 204), color="stone")]


def long_beard(h, m, t):
    return [h.smooth([(2, 172), (11, 172), (13, 160), (10, 146), (6, 140), (4, 152)], "white")]


def pipe(h, m, t):
    return [h.line((9, 172), (18, 170), style=f"line width={h.LW * 1.2}cm", color="brown_dark"), h.poly([(16, 170), (20, 170), (20, 176), (16, 176)], "brown_dark"),
            h.curve([(18, 178), (16, 184), (20, 190)], color="stone")]


def face_mask(h, m, t):
    return [h.poly([(2, 178), (12, 178), (12, 165), (4, 164)], "bone_dark"), h.line((-11, 175), (3, 176), color="brown_dark")]


def hair_brushes(h, m, t):
    return [h.line((-6, 192), (-14, 206), style=f"line width={h.LW * 1.2}cm", color="black"), h.poly([(-15, 206), (-13, 207), (-15, 211), (-17, 208)], "pumpkin_dark", h.THIN),
            h.line((-3, 194), (-6, 210), style=f"line width={h.LW * 1.2}cm", color="wood"), h.poly([(-7, 210), (-5, 210), (-6, 214)], "gold", h.THIN)]


def goggles(h, m, t):
    return [h.poly([(-11, 186), (11, 188), (11, 191), (-11, 189)], "brown_dark", h.THIN), h.circ(8, 186, 3.2, "sky"), h.circ(8, 186, 3.2, "sky", f"draw=steeldark, line width={h.LW}cm")]


def cloth_strips(h, m, t):
    return [h.curve([(-6, 198), (-14, 204), (-22, 200), (-28, 206)], style=f"line width={h.LW * 1.6}cm", color="sky"),
            h.curve([(-4, 199), (-10, 210), (-18, 212), (-22, 220)], style=f"line width={h.LW * 1.6}cm", color="white"),
            h.curve([(-8, 197), (-18, 196), (-26, 190)], style=f"line width={h.LW * 1.6}cm", color="pumpkin")]


def third_eye(h, m, t):
    return [h.ell(9.5, 189, 1.6, 3, "glow", h.THIN), h.circ(9.8, 189, 0.9, "plum", "draw=none")]


def crescent(h, m, t):
    return [f"\\filldraw[fill=glowsoft, {h.THIN}] {h.P(-4, 208)} arc (90:270:7) arc (270:90:4.6 and 7) -- cycle;"]


def hoop_earring(h, m, t):
    return [f"\\draw[draw=ink, line width={h.LW * 1.6}cm] {h.P(1, 172)} circle (3.6);", f"\\draw[draw=gold, line width={h.LW * 0.9}cm] {h.P(1, 172)} circle (3.6);"]


def tall_stripes(h, m, t):
    o = [h.poly([(-8, 196), (8, 196), (6, 226), (-6, 226)], "white")]
    for k in range(4):
        o.append(h.poly([(-8 + k * 0.5, 199 + k * 7), (8 - k * 0.5, 199 + k * 7), (8 - k * 0.5, 202 + k * 7), (-8 + k * 0.5, 202 + k * 7)], m, h.THIN))
    return o


def lantern_crown(h, m, t):
    o = [h.poly([(-12, 196), (12, 196), (12, 199), (-12, 199)], "black", h.THIN)]
    for x, c in ((-14, "glow"), (0, "pumpkin"), (14, "glow")):
        o += [h.line((x * 0.8, 199), (x, 206), color="ink"), h.ell(x, 211, 4, 5, c), h.line((x - 3, 211), (x + 3, 211), color="pumpkin_dark")]
    return o


def _grown(fn, k, px, py):
    """`fn` drawn `k` times larger about (px, py): a thing at the belt is
    drawn into the chest's picture, over the clothes, where the waist's
    picture is covered by the chest's."""
    def draw(h, m, t):
        return [f"\\begin{{scope}}[shift={{({px},{py})}}, scale={k}, shift={{({-px},{-py})}}]"] + fn(h, m, t) + ["\\end{scope}"]
    return draw


def _moved(fn, dx, dy):
    """`fn` drawn moved by (dx, dy) cm."""
    def draw(h, m, t):
        return [f"\\begin{{scope}}[shift={{({dx},{dy})}}]"] + fn(h, m, t) + ["\\end{scope}"]
    return draw


# Each signature piece: (slot, draw). Every one is worn by one opponent.
SIG = {
    "crow": ("head", _moved(crow_on_shoulder, 2, 30)), "flail": ("chest", flail), "rope_coil": ("chest", rope_coil), "vault_pole": ("chest", vault_pole),
    "crook": ("chest", crook), "parcel": ("chest", parcel), "salt_sack": ("chest", salt_sack), "falcon": ("chest", falcon), "oar": ("chest", oar),
    "map_quiver": ("chest", map_quiver), "seal": ("head", seal_medallion), "crozier": ("chest", crozier), "pelt": ("chest", pelt),
    "pots": ("chest", pots_and_pans), "walking_staff": ("chest", walking_staff), "whetstone": ("chest", whetstone), "cowbell": ("head", neck_bell),
    "eel_trap": ("chest", eel_trap), "carrier": ("chest", carrier), "paper_frame": ("chest", paper_frame), "banner": ("chest", war_banner),
    "tiles": ("chest", roof_tiles), "charcoal": ("chest", charcoal_basket), "tea_basket": ("chest", tea_basket), "cape": ("chest", cape),
    "kite": ("chest", kite), "net": ("chest", drying_net), "axe": ("chest", axe), "harpoon": ("chest", harpoon), "reeds": ("chest", reed_bundle),
    "fleece": ("head", fleece), "prayer_flags": ("chest", prayer_flags), "jewels": ("head", jewels), "hoop": ("chest", iron_hoop),
    "medallion": ("head", coin_medallion), "wheel": ("chest", cart_wheel), "quoits": ("head", quoits), "rays": ("head", ray_collar),
    "wheat": ("chest", wheat_bundle), "ladder": ("chest", ladder), "spade": ("chest", spade), "mane": ("chest", mane_drape),
    "straw_cape": ("chest", straw_cape), "silk": ("chest", silk_scarf), "stone_block": ("chest", stone_block), "towel": ("head", neck_towel), "diamonds": ("chest", diamond_tunic),
    "keys": ("chest", _grown(key_ring, 1.35, 2, 116)), "tea_set": ("chest", _grown(tea_set, 1.35, 2, 116)), "lantern": ("chest", _grown(lantern, 1.35, 2, 116)), "hoops": ("chest", _grown(barrel_hoops, 1.35, 2, 116)),
    "tongs": ("chest", _grown(smith_tools, 1.35, 2, 116)), "horn": ("chest", _grown(signal_horn, 1.35, 2, 116)), "skeins": ("chest", _grown(skeins, 1.35, 2, 116)), "square": ("chest", _grown(carpenter_square, 1.35, 2, 116)),
    "spool": ("chest", _grown(spool, 1.35, 2, 116)), "mug": ("chest", _grown(mug, 1.35, 2, 116)), "herb_basket": ("chest", _grown(herb_basket, 1.35, 2, 116)), "shackles": ("chest", _grown(shackles, 1.35, 2, 116)),
    "abacus": ("chest", _grown(abacus, 1.35, 2, 116)), "saddlebag": ("chest", _grown(saddlebag, 1.35, 2, 116)), "dice": ("chest", _grown(dice, 1.35, 2, 116)), "alms_bowl": ("chest", _grown(alms_bowl, 1.35, 2, 116)),
    "scabbard": ("chest", _grown(second_scabbard, 1.35, 2, 116)), "bucket": ("chest", _grown(bucket, 1.35, 2, 116)), "streamers_paper": ("chest", _grown(paper_streamers, 1.35, 2, 116)),
    "coins": ("chest", _grown(coin_string, 1.35, 2, 116)), "hare": ("chest", _grown(hare, 1.35, 2, 116)), "spyglass": ("chest", _grown(spyglass, 1.35, 2, 116)), "drum": ("chest", _grown(hip_drum, 1.35, 2, 116)),
    "bangles": ("chest", _grown(bangles, 1.35, 2, 116)), "sickle": ("chest", _grown(sickle, 1.35, 2, 116)), "pears": ("chest", _grown(pear_basket, 1.35, 2, 116)), "clapper": ("chest", _grown(clapper, 1.35, 2, 116)),
    "grain_measure": ("chest", _grown(grain_measure, 1.35, 2, 116)), "ribbons": ("chest", _grown(streamers, 1.35, 2, 116)), "rope_belt": ("chest", _grown(rope_belt, 1.35, 2, 116)), "fans": ("chest", _grown(fans, 1.35, 2, 116)),
    "sheaf": ("chest", _grown(sheaf, 1.35, 2, 116)),
    "pinwheel": ("head", pinwheel), "spectacles": ("head", spectacles), "hood_bell": ("head", hood_bell), "parasol": ("head", parasol_hat),
    "quill": ("head", quill), "beard": ("head", long_beard), "pipe": ("head", pipe), "mask": ("head", face_mask), "brushes": ("head", hair_brushes),
    "goggles": ("head", goggles), "cloth_strips": ("head", cloth_strips), "third_eye": ("head", third_eye), "crescent": ("head", crescent),
    "earring": ("head", hoop_earring), "tall_hat": ("head", tall_stripes), "lantern_crown": ("head", lantern_crown),
}
