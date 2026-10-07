#!/usr/bin/env python3
"""The region art behind arcade mode's chart (Sam, 2026-10-07: "some art made
for each layer, some basic stuff done with the tikz prompt, in the style of
the art sam comm final ... matching the name of the area ... each layer to
somehow meld and merge with the next one at the boundary").

One wide scene per row of the arcade tree, after Sam_Comm_final.png: flat
fills, plum ink lines, a mustard sky, peach and sage. Every color comes from
data/palette.json's `art` section. Detail is drawn from a fixed seed, so a
scene is the same each time it is drawn. Each scene's bottom edge is the
next scene's sky, and the page fades the two into each other.

    python3 analysis/art/regions.py      # writes web/art/region-<n>.png
"""
import json, math, random, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PAL = json.loads((ROOT / "data" / "palette.json").read_text())["art"]
OUT = ROOT / "web" / "art"
W, H = 32.0, 7.0  # cm

def colors():
    return "\n".join(f"\\definecolor{{{k.replace('_','')}}}{{HTML}}{{{v[1:]}}}" for k, v in PAL.items() if not k.startswith("_"))

LINE = "draw=ink, line width=0.9pt, line join=round"

def poly(pts, fill, extra=""):
    return f"\\filldraw[fill={fill}, {LINE}{extra}] " + " -- ".join(f"({x:.2f},{y:.2f})" for x, y in pts) + " -- cycle;"

def rect(x, y, w, h, fill, extra=""):
    return f"\\filldraw[fill={fill}, {LINE}{extra}] ({x:.2f},{y:.2f}) rectangle ({x+w:.2f},{y+h:.2f});"

def sky(r, base="sky"):
    out = [f"\\fill[{base}] (0,0) rectangle ({W},{H});",
           f"\\fill[skylight] (0,{H*0.55:.2f}) rectangle ({W},{H});"]
    for _ in range(5):
        cx, cy = r.uniform(1, W - 1), r.uniform(H * 0.72, H * 0.92)
        blobs = " ".join(f"({cx + dx:.2f},{cy + dy:.2f}) circle ({rad:.2f})" for dx, dy, rad in
                         [(0, 0, 0.55), (0.6, 0.15, 0.45), (-0.6, 0.1, 0.4), (1.1, -0.05, 0.3), (-1.0, -0.05, 0.3)])
        out.append(f"\\filldraw[fill=cloud, draw=ink, line width=0.6pt] {blobs};")
    return out

def hills(r, y0, amp, fill, n=9):
    xs = [i * W / n for i in range(n + 1)]
    pts = [(0, 0)] + [(x, y0 + amp * math.sin(i * 1.3 + r.random()) + r.uniform(-0.2, 0.2)) for i, x in enumerate(xs)] + [(W, 0)]
    return f"\\filldraw[fill={fill}, {LINE}] plot[smooth] coordinates {{" + " ".join(f"({x:.2f},{y:.2f})" for x, y in pts[1:-1]) + f"}} -- ({W},0) -- (0,0) -- cycle;"

def house(x, y, w, h, wall, roof, r):
    out = [rect(x, y, w, h, wall), poly([(x - 0.2, y + h), (x + w / 2, y + h + h * 0.55), (x + w + 0.2, y + h)], roof)]
    for i in range(int(w / 0.5)):
        out.append(rect(x + 0.15 + i * 0.5, y + h * 0.45, 0.28, 0.3, "skylight"))
    out.append(rect(x + w / 2 - 0.15, y, 0.3, h * 0.38, "wood"))
    return out

def scene(n, r):
    out = sky(r)
    if n == 0:  # the rice fields: terraced paddies
        out.append(hills(r, 3.6, 0.4, "sagedark"))
        for i, y in enumerate([2.8, 2.0, 1.2, 0.4]):
            out.append(f"\\filldraw[fill={'olive' if i % 2 else 'sage'}, {LINE}] (0,{y-0.9:.2f}) rectangle ({W},{y:.2f});")
            for k in range(70):
                x = r.uniform(0.2, W - 0.2); yy = y - r.uniform(0.15, 0.75)
                out.append(f"\\draw[ink, line width=0.5pt] ({x:.2f},{yy:.2f}) -- ++(-0.08,0.22) ({x:.2f},{yy:.2f}) -- ++(0.08,0.22) ({x:.2f},{yy:.2f}) -- ++(0,0.25);")
            out.append(f"\\draw[tealdark, line width=0.8pt] plot[smooth] coordinates {{" + " ".join(f"({x:.1f},{y-0.45+0.05*math.sin(x*3):.2f})" for x in [i*1.6 for i in range(21)]) + "};")
    elif n == 1:  # the village road
        out.append(hills(r, 3.0, 0.5, "sage"))
        out.append(poly([(W * 0.42, 3.0), (W * 0.58, 3.0), (W * 0.8, 0), (W * 0.2, 0)], "stone"))
        for x in [1.0, 4.2, 7.4, 21.0, 24.4, 27.8]:
            out += house(x, 2.0 + r.uniform(-0.3, 0.3), 2.6, 1.5, "peach", "peachdark", r)
        for k in range(18):
            x = 10.5 + k * 0.55
            out.append(f"\\draw[ink, line width=0.7pt] ({x:.2f},0.6) -- ++(0,0.7);")
        out.append(f"\\draw[ink, line width=0.7pt] (10.5,1.1) -- (20.4,1.1);")
    elif n == 2:  # the river crossing
        out.append(hills(r, 3.8, 0.4, "sagedark"))
        out.append(f"\\filldraw[fill=teal, {LINE}] (0,0.6) rectangle ({W},3.2);")
        for k in range(60):
            x, y = r.uniform(0.2, W), r.uniform(0.8, 3.0)
            out.append(f"\\draw[tealdark, line width=0.6pt] ({x:.2f},{y:.2f}) .. controls ++(0.2,0.12) and ++(-0.2,0.12) .. ++(0.6,0);")
        out.append(poly([(12, 3.4), (20, 3.4), (20.3, 3.0), (11.7, 3.0)], "wood"))
        for k in range(9):
            out.append(rect(12 + k * 0.95, 0.6, 0.18, 2.4, "wood"))
        out.append(poly([(4, 1.4), (8, 1.4), (7.4, 0.9), (4.6, 0.9)], "wood"))
        out.append(f"\\draw[ink, line width=1pt] (6,1.4) -- (6.3,3.0);")
    elif n == 3:  # the market towns
        out.append(hills(r, 3.6, 0.3, "olive"))
        for i in range(9):
            x = 0.6 + i * 3.5
            out.append(rect(x, 0.4, 2.8, 1.8, "cream"))
            stripes = " ".join(f"({x + j*0.4:.2f},2.2) rectangle ({x + j*0.4 + 0.2:.2f},2.75)" for j in range(7))
            out.append(f"\\fill[{'peach' if i % 2 else 'sage'}] ({x-0.1:.2f},2.2) rectangle ({x+2.9:.2f},2.75);")
            out.append(f"\\fill[{'peachdark' if i % 2 else 'sagedark'}] {stripes};")
            out.append(f"\\draw[ink, line width=0.9pt] ({x-0.1:.2f},2.2) rectangle ({x+2.9:.2f},2.75);")
            for j in range(4):
                out.append(f"\\filldraw[fill={r.choice(['gold','peach','sage','teal'])}, draw=ink, line width=0.5pt] ({x+0.4+j*0.6:.2f},0.85) circle (0.2);")
        out.append(f"\\draw[ink, line width=0.6pt] (0,3.4) .. controls (8,3.0) and (24,3.0) .. ({W},3.4);")
        for k in range(16):
            x = 1 + k * 2
            out.append(f"\\filldraw[fill=gold, draw=ink, line width=0.6pt] ({x:.2f},{3.16-0.12*math.sin(k):.2f}) ellipse (0.18 and 0.25);")
    elif n == 4:  # the hill country
        out.append(hills(r, 4.0, 0.7, "sagedark", 7))
        out.append(hills(r, 2.6, 0.6, "sage", 6))
        out.append(hills(r, 1.2, 0.4, "olive", 8))
        for k in range(14):
            x, y = r.uniform(0.5, W - 0.5), r.uniform(1.6, 3.6)
            out.append(rect(x - 0.07, y - 0.5, 0.14, 0.5, "wood"))
            out.append(f"\\filldraw[fill=sagedark, draw=ink, line width=0.7pt] ({x:.2f},{y+0.15:.2f}) circle (0.42);")
    elif n == 5:  # the mountain pass
        peaks = [(0, 0)]
        for i in range(12):
            peaks += [(i * 2.7 + 1.3, r.uniform(3.8, 5.6)), (i * 2.7 + 2.7, r.uniform(1.6, 2.6))]
        peaks += [(W, 0)]
        out.append(poly(peaks, "slate"))
        for x, y in peaks[1:-1:2]:
            out.append(poly([(x - 0.45, y - 0.7), (x, y), (x + 0.45, y - 0.7), (x + 0.1, y - 0.5)], "cream"))
        out.append(poly([(0, 0), (0, 1.0), (W * 0.45, 1.6), (W * 0.55, 1.6), (W, 1.0), (W, 0)], "stone"))
        for k in range(25):
            x = r.uniform(0.5, W - 0.5)
            out.append(f"\\filldraw[fill=stone, draw=ink, line width=0.6pt] ({x:.2f},{r.uniform(0.2,0.9):.2f}) circle ({r.uniform(0.1,0.28):.2f});")
    elif n == 6:  # the high plateau
        out.append(poly([(0, 0), (0, 2.6), (3, 3.1), (W - 3, 3.1), (W, 2.6), (W, 0)], "olive"))
        for k in range(120):
            x, y = r.uniform(0.2, W - 0.2), r.uniform(0.2, 2.8)
            out.append(f"\\draw[sagedark, line width=0.5pt] ({x:.2f},{y:.2f}) -- ++(0.12,0.3) ({x:.2f},{y:.2f}) -- ++(-0.08,0.25);")
        out.append(f"\\draw[ink, line width=0.8pt] (5,3.1) -- (5,5.0) (27,3.1) -- (27,5.0);")
        out.append(f"\\draw[ink, line width=0.6pt] (5,4.9) .. controls (14,4.2) and (18,4.2) .. (27,4.9);")
        for k in range(24):
            t = k / 23
            x = 5 + 22 * t
            y = 4.9 - 2.8 * t * (1 - t)
            out.append(poly([(x - 0.2, y), (x + 0.2, y), (x, y - 0.45)], r.choice(["gold", "teal", "cream", "peach", "sage"])))
    elif n == 7:  # the far gate
        out.append(hills(r, 3.0, 0.3, "sagedark"))
        out.append(rect(0, 0.4, W, 2.0, "stone"))
        for row in range(5):
            for k in range(42):
                out.append(f"\\draw[ink, line width=0.4pt] ({k*0.8 + (row % 2) * 0.4:.2f},{0.4+row*0.4:.2f}) rectangle ++(0.8,0.4);")
        out.append(rect(13.2, 0.4, 0.6, 4.6, "wood"))
        out.append(rect(18.2, 0.4, 0.6, 4.6, "wood"))
        out.append(poly([(12.2, 4.9), (19.8, 4.9), (20.4, 5.5), (11.6, 5.5)], "peachdark"))
        out.append(rect(12.6, 4.2, 6.8, 0.35, "wood"))
        out.append(rect(13.8, 0.4, 4.4, 3.8, "shadow"))
    elif n == 8:  # the hilltop shrine
        out.append(hills(r, 2.4, 0.8, "sage", 5))
        out.append(poly([(10, 0), (13.5, 3.6), (18.5, 3.6), (22, 0)], "sagedark"))
        for k in range(10):
            y = 0.3 + k * 0.33
            w = 2.4 - k * 0.12
            out.append(rect(16 - w / 2, y, w, 0.33, "stone"))
        out.append(rect(14.2, 3.6, 3.6, 1.5, "peach"))
        out.append(poly([(13.4, 5.1), (18.6, 5.1), (17.6, 6.1), (14.4, 6.1)], "peachdark"))
        out.append(rect(15.4, 3.6, 1.2, 1.0, "shadow"))
        out.append(f"\\filldraw[fill=gold, draw=ink, line width=0.8pt] (19.6,4.4) .. controls (19.4,3.6) and (20.6,3.6) .. (20.4,4.4) -- cycle;")
        out.append(f"\\draw[ink, line width=0.8pt] (19.2,4.4) -- (20.8,4.4) (20,4.4) -- (20,5.2) (19.85,4.0) -- (20.1,3.75);")
    elif n == 9:  # beneath the shrine: a cave
        out = [f"\\fill[shadow] (0,0) rectangle ({W},{H});"]
        out.append(poly([(0, H)] + [(x, H - r.uniform(0.6, 1.8)) for x in [i * 1.6 for i in range(1, 20)]] + [(W, H)], "slate"))
        for k in range(26):
            x = r.uniform(0.4, W - 0.4)
            out.append(poly([(x - 0.25, H - 0.6), (x + 0.25, H - 0.6), (x, H - 0.6 - r.uniform(0.5, 1.6))], "slate"))
        out.append(poly([(0, 0), (0, 1.2)] + [(x, 1.2 + r.uniform(-0.3, 0.4)) for x in [i * 2 for i in range(1, 16)]] + [(W, 1.2), (W, 0)], "slate"))
        for k in range(30):
            x, y = r.uniform(1, W - 1), r.uniform(1.6, 4.6)
            out.append(f"\\filldraw[fill=gold, draw=ink, line width=0.4pt] ({x:.2f},{y:.2f}) circle ({r.uniform(0.05,0.12):.2f});")
        out.append(f"\\filldraw[fill=gold, draw=ink, line width=1pt] (15.6,1.3) rectangle (16.4,1.6);")
        out.append(f"\\draw[ink, line width=1.2pt] (16,1.6) -- (16.4,4.2);")
    # A frame line along the bottom edge, as the reference's ground does.
    return out

def render(n):
    r = random.Random(4000 + n)
    body = "\n".join(scene(n, r))
    tex = f"""\\documentclass[tikz,border=0pt]{{standalone}}
\\usepackage{{tikz}}
{colors()}
\\begin{{document}}
\\begin{{tikzpicture}}
\\clip (0,0) rectangle ({W},{H});
{body}
\\end{{tikzpicture}}
\\end{{document}}
"""
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as d:
        Path(d, "f.tex").write_text(tex)
        rr = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "f.tex"], cwd=d, capture_output=True, text=True)
        if rr.returncode:
            sys.exit(f"pdflatex failed for region {n}:\n" + "\n".join(l for l in rr.stdout.splitlines() if l.startswith("!")))
        subprocess.run(["pdftocairo", "-png", "-singlefile", "-scale-to-x", "1800", "-scale-to-y", "-1", "f.pdf", str(OUT / f"region-{n}")], cwd=d, check=True)

if __name__ == "__main__":
    regions = json.loads((ROOT / "data" / "copy.en.json").read_text())["road"]["region"]
    for k in sorted(regions, key=int):
        render(int(k))
        print("drew region", k)
