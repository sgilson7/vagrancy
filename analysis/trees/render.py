#!/usr/bin/env python3
"""Draw every opponent's behavior tree and the node icons, as TikZ, after
Sam's TikZ figure prompt (2026-10-06): one standalone .tex per figure, every
size and color set once at the top, repeated parts generated, compiled with
pdflatex and turned into PNGs with pdftocairo.

Input: analysis/trees/trees.json, written by `cargo run -p lab -- trees`
(each pilot as `pilot::view::describe` builds it, labels filled from the
copy file). Output: web/icons/<icon>.png and <icon>-lit.png, web/trees/<id>.png,
and web/trees/manifest.json, whose fingerprint the test
`the_drawn_trees_are_current` checks. Run as `make trees`.
"""
import json, os, re, subprocess, sys, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
WEB = ROOT / "web"
PALETTE = json.loads((ROOT / "data" / "palette.json").read_text())

# --- Sizes, in cm ---------------------------------------------------------------
BOX_W = 3.9      # a node's box
BOX_H = 1.75
HGAP = 0.45      # between branches side by side
VGAP = 0.55      # between a node and what hangs below it
INDENT = 0.75    # how far a sequence's steps sit right of the sequence
ICON = 0.95      # an icon's side inside a box
DPI = 110        # tree images
ICON_PX = 96     # icon images

def hexcol(h):
    return h.lstrip("#").upper()

COLORS = {
    "paper": hexcol(PALETTE["paper"]),
    "ink": hexcol(PALETTE["line"]),
    "ground": hexcol(PALETTE["ground"]),
    "muted": hexcol(PALETTE["muted"]),
    "cond": hexcol(PALETTE["focus"]),                    # conditions: indigo
    "act": hexcol(PALETTE["fighters"]["right"]["stripe"]),  # moves: dark ochre
}

def fnv1a(data: bytes) -> str:
    h = 0xcbf29ce484222325
    for b in data:
        h ^= b
        h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"

def macro(icon):
    return "Icon" + "".join(p.capitalize() for p in icon.split("_"))

def preamble():
    cols = "\n".join(f"\\definecolor{{{k}}}{{HTML}}{{{v}}}" for k, v in COLORS.items())
    return ("\\documentclass[tikz,border=2pt]{standalone}\n"
            "\\usetikzlibrary{arrows.meta,positioning,calc}\n"
            f"{cols}\n\\input{{{HERE / 'icons.tex'}}}\n")

def compile_png(tex: str, out: Path, args):
    with tempfile.TemporaryDirectory() as d:
        src = Path(d) / "f.tex"
        src.write_text(tex)
        r = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "f.tex"], cwd=d, capture_output=True, text=True)
        if r.returncode != 0:
            sys.exit(f"pdflatex failed for {out.name}:\n" + "\n".join(l for l in r.stdout.splitlines() if l.startswith("!")))
        subprocess.run(["pdftocairo", "-png", "-singlefile", *args, "f.pdf", str(out.with_suffix(""))], cwd=d, check=True)

# --- Icons ------------------------------------------------------------------------------
def icon_tex(icon, color):
    return (preamble() + "\\begin{document}\n\\begin{tikzpicture}[x=0.1cm,y=0.1cm]\n"
            f"\\def\\IC{{{color}}}\\def\\IW{{0.9pt}}\n"
            "\\path (0,0) rectangle (10,10);  % the same box for every icon\n"
            f"\\{macro(icon)}\n\\end{{tikzpicture}}\n\\end{{document}}\n")

# --- Trees -----------------------------------------------------------------------------------
def across(n):
    return n["kind"] in ("selector", "parallel", "repeat")

def measure(n):
    if not n["children"]:
        return BOX_W, BOX_H
    sizes = [measure(c) for c in n["children"]]
    if across(n):
        w = max(BOX_W, sum(s[0] for s in sizes) + HGAP * (len(sizes) - 1))
        return w, BOX_H + VGAP + max(s[1] for s in sizes)
    return max(BOX_W, INDENT + max(s[0] for s in sizes)), BOX_H + sum(VGAP + s[1] for s in sizes)

STYLE = {"selector": "comp", "sequence": "comp", "parallel": "comp", "repeat": "comp",
         "condition": "cond", "action": "act", "search": "srch"}

def esc(s):
    return s.replace("\\", "\\textbackslash{}").replace("%", "\\%").replace("&", "\\&").replace("#", "\\#").replace("_", "\\_")

def place(n, x, y, out, edges):
    w, _ = measure(n)
    bx = x + (w - BOX_W) / 2 if across(n) and n["children"] else x
    style = STYLE[n["kind"]] + (", dashed" if n["interrupt"] else "")
    out.append(f"\\node[box, {style}] (n{n['id']}) at ({bx + BOX_W / 2:.3f},{y - BOX_H / 2:.3f}) {{}};\n"
               f"\\begin{{scope}}[shift={{({bx + 0.25:.3f},{y - BOX_H / 2 - ICON / 2:.3f})}}, x={ICON / 10:.4f}cm, y={ICON / 10:.4f}cm]"
               f"\\def\\IC{{ink}}\\def\\IW{{0.7pt}}\\{macro(n['icon'])}\\end{{scope}}\n"
               f"\\node[label] at ({bx + 0.25 + ICON + 0.15:.3f},{y - BOX_H / 2:.3f}) {{{esc(n['text'])}}};\n")
    if not n["children"]:
        return
    if across(n):
        total = sum(measure(c)[0] for c in n["children"]) + HGAP * (len(n["children"]) - 1)
        cx = x + (w - total) / 2
        for c in n["children"]:
            cw, _ = measure(c)
            place(c, cx, y - BOX_H - VGAP, out, edges)
            edges.append(f"\\draw[edge] (n{n['id']}.south) -- (n{c['id']}.north);\n")
            cx += cw + HGAP
    else:
        cy = y - BOX_H - VGAP
        spine = bx + 0.35
        for c in n["children"]:
            place(c, x + INDENT, cy, out, edges)
            edges.append(f"\\draw[edge] ({spine:.3f},{y - BOX_H:.3f}) |- (n{c['id']}.west);\n")
            cy -= measure(c)[1] + VGAP

def tree_tex(tree):
    out, edges = [], []
    place(tree, 0, 0, out, edges)
    return (preamble() + "\\begin{document}\n\\begin{tikzpicture}[font=\\sffamily,\n"
            f"  box/.style={{draw, line width=1.1pt, rounded corners=3pt, minimum width={BOX_W}cm, minimum height={BOX_H}cm}},\n"
            "  comp/.style={draw=ink, fill=ground}, cond/.style={draw=cond, fill=paper}, act/.style={draw=act, fill=paper},\n"
            "  srch/.style={draw=cond, fill=paper, line width=2pt},\n"
            f"  label/.style={{anchor=west, text=ink, text width={BOX_W - ICON - 0.65:.2f}cm, align=left, font=\\sffamily\\footnotesize, inner sep=0pt}},\n"
            "  edge/.style={draw=muted, line width=1pt}]\n"
            "% --- Nodes ---\n" + "".join(out)
            + "% --- Edges (after the nodes they join; they end at the borders) ---\n" + "".join(edges)
            + "\\end{tikzpicture}\n\\end{document}\n")

def main():
    src = (HERE / "trees.json").read_bytes()
    trees = json.loads(src)
    icons = sorted(set(re.findall(r"\\newcommand\{\\Icon(\w+)\}", (HERE / "icons.tex").read_text())))
    names = sorted({re.sub(r"(?<!^)([A-Z])", r"_\1", m).lower() for m in icons})
    (WEB / "icons").mkdir(exist_ok=True)
    (WEB / "trees").mkdir(exist_ok=True)
    for name in names:
        compile_png(icon_tex(name, "ink"), WEB / "icons" / f"{name}.png", ["-transp", "-scale-to", str(ICON_PX)])
        compile_png(icon_tex(name, "paper"), WEB / "icons" / f"{name}-lit.png", ["-transp", "-scale-to", str(ICON_PX)])
    for tid, tree in trees.items():
        (HERE / "tex").mkdir(exist_ok=True)
        tex = tree_tex(tree)
        (HERE / "tex" / f"{tid}.tex").write_text(tex)
        compile_png(tex, WEB / "trees" / f"{tid}.png", ["-transp", "-r", str(DPI)])
    fp = fnv1a(src + (HERE / "icons.tex").read_bytes() + (HERE / "render.py").read_bytes())
    (WEB / "trees" / "manifest.json").write_text(json.dumps({"fingerprint": fp, "trees": sorted(trees), "icons": names}, indent=2) + "\n")
    print(f"{len(names)} icons, {len(trees)} trees, fingerprint {fp}")

if __name__ == "__main__":
    main()
