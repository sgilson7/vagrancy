#!/usr/bin/env bash
# Build the browser app into dist/web/.
#
# No node, no npm, no bundler: the page is a hand-written ES module and
# wasm-bindgen emits the only generated file. Ported from gear-master-2d's
# packaging/package-web.sh (the stamping, the lockfile pin), with Floodline's
# vendor sha256 check and three steps of this game's own: the audio guard,
# the copy fill and the palette variables.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEB="${VAGRANCY_WEB:-$ROOT/dist/web}"
CRATE=vagrancy-wasm
WASM=vagrancy_wasm
TARGET=wasm32-unknown-unknown

say() { printf '\033[1m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }
# `shasum` is BSD, `sha256sum` is what the Linux CI runner has (Floodline).
sha256() { if command -v shasum >/dev/null; then shasum -a 256; else sha256sum; fi }

rustup target list --installed 2>/dev/null | grep -q "^$TARGET$" \
  || die "missing target. Run: rustup target add $TARGET"

# wasm-bindgen-cli and the wasm-bindgen crate must be the same version or the
# generated glue will not match the module. Read the version the lockfile
# actually pinned rather than trusting whatever happens to be installed.
NEED=$(awk '/^name = "wasm-bindgen"$/{f=1} f&&/^version = /{gsub(/"/,"");print $3;exit}' "$ROOT/Cargo.lock")
command -v wasm-bindgen >/dev/null \
  || die "wasm-bindgen not found. Run: cargo install wasm-bindgen-cli --version $NEED"
HAVE=$(wasm-bindgen --version | awk '{print $2}')
[ "$HAVE" = "$NEED" ] \
  || die "wasm-bindgen $HAVE installed but the lockfile pins $NEED.
  Run: cargo install wasm-bindgen-cli --version $NEED --force"

# --- no audio without a license row (PLANNING-BRIEF 0.5) --------------------
# web/assets/music/ is gitignored, so Sam's own copy of a track can sit there
# on this machine; this is the second guard, and it refuses to package it.
say "Checking audio against LICENSES.md"
while IFS= read -r f; do
  rel="${f#"$ROOT"/}"
  grep -qF "\`$rel\`" "$ROOT/LICENSES.md" \
    || die "$rel is audio with no row in LICENSES.md, so it does not ship.
  Adding audio to the build is Sam's decision (PLANNING-BRIEF 0.4)."
done < <(find "$ROOT/web" "$ROOT/data" -type f \( -iname '*.mp3' -o -iname '*.ogg' \
          -o -iname '*.oga' -o -iname '*.wav' -o -iname '*.flac' -o -iname '*.m4a' \
          -o -iname '*.aac' -o -iname '*.opus' -o -iname '*.weba' \))

# --- vendored bundles are the bytes that were pinned (Floodline) ------------
if [ -f "$ROOT/web/vendor/SHA256SUMS" ]; then
  say "Checking vendored bundles"
  (cd "$ROOT/web/vendor" && while read -r want name; do
     got=$(sha256 < "$name" | cut -d' ' -f1)
     [ "$got" = "$want" ] || die "web/vendor/$name is $got, but SHA256SUMS pins $want"
   done < SHA256SUMS)
fi

say "Building $CRATE for $TARGET"
cargo build --release --target "$TARGET" -p "$CRATE"

say "Assembling $WEB"
rm -rf "$WEB"; mkdir -p "$WEB"
cp -R "$ROOT/web/." "$WEB/"
rm -rf "$WEB/pkg" "$WEB/assets/music"
mkdir -p "$WEB/data"
cp "$ROOT/data/copy.en.json" "$WEB/data/"

say "Generating JS bindings"
wasm-bindgen --target web --no-typescript --out-dir "$WEB/pkg" \
  "$ROOT/target/$TARGET/release/$WASM.wasm"

if command -v wasm-opt >/dev/null; then
  say "Optimising wasm"
  wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
    -o "$WEB/pkg/${WASM}_bg.wasm" "$WEB/pkg/${WASM}_bg.wasm"
fi

# --- the words and the colors, from their one home --------------------------
# index.html holds {{key}} tokens and no text of its own; they are filled here
# because the title, the fallback and the loading line must be on the page
# before any script runs. styles.css gets its color variables from
# data/palette.json for the same reason.
say "Filling copy and palette"
python3 - "$ROOT" "$WEB" <<'PY'
import html, json, re, sys
root, web = sys.argv[1], sys.argv[2]
copy = json.load(open(f"{root}/data/copy.en.json"))
pal = json.load(open(f"{root}/data/palette.json"))
def look(key):
    v = copy
    for k in key.split("."):
        v = v[k]
    if not isinstance(v, str):
        sys.exit(f"{key} is not a string in the copy file")
    return v
def fill(m):
    s = look(m.group(1)).replace("{game}", copy["game"]["name"])
    if "{" in s.replace("{hash}", ""):
        sys.exit(f"{m.group(1)} has a placeholder packaging cannot fill: {s}")
    return html.escape(s, quote=True)
# The game's page and the BT Lab's, each holding {{key}} tokens.
for p in (f"{web}/index.html", f"{web}/bt-lab.html", f"{web}/arena.html", f"{web}/clip.html"):
    page = re.sub(r"<!--.*?-->\s*", "", open(p).read(), flags=re.S)
    page = re.sub(r"\{\{([a-z_.]+)\}\}", fill, page)
    open(p, "w").write(page)
flat = {}
def walk(v, path):
    if isinstance(v, dict):
        for k, c in v.items():
            if not k.startswith("_"):
                walk(c, f"{path}-{k}" if path else k)
    elif isinstance(v, str) and v.startswith("#"):
        flat[path.replace("_", "-")] = v
walk(pal, "")
css = ":root {\n" + "".join(f"  --{k}: {v};\n" for k, v in sorted(flat.items())) + "}\n"
p = f"{web}/styles.css"
# Read before opening for write: `open(p, "w")` truncates first, and the
# first draft of this line shipped a stylesheet that was only its colors.
rest = open(p).read()
open(p, "w").write(css + rest)
PY

# --- cache busting (gear-master-2d, verbatim in intent) ---------------------
# GitHub Pages serves everything with `Cache-Control: max-age=600`, so for ten
# minutes after a deploy a reload keeps using the previous app.js and .wasm
# from disk cache. Stamping the content hash into every internal asset URL
# means a changed build is simply a different URL. A deployed fix that a
# returning player cannot receive is not a delivered fix.
say "Stamping build version"
bust() { S="$2" R="$3" perl -0777 -pi -e 's/\Q$ENV{S}\E/$ENV{R}/g' "$1"; }

# Everything the browser caches, whatever it is called — hashed before
# stamping, which is what makes it stable.
BUILD=$(cat "$WEB"/*.js "$WEB"/*.html "$WEB/styles.css" "$WEB/data/copy.en.json" \
            "$WEB/pkg/$WASM.js" "$WEB/pkg/${WASM}_bg.wasm" | sha256 | cut -c1-8)

# Every relative import in every shipped module, rather than a list of the
# ones somebody remembered (GM2D found two that were missed by name).
for js in "$WEB"/*.js; do
  perl -0777 -pi -e "s{from '\./([A-Za-z0-9_-]+\.js)'}{from './\$1?v=$BUILD'}g" "$js"
  perl -0777 -pi -e "s{from '\./pkg/$WASM\.js'}{from './pkg/$WASM.js?v=$BUILD'}g" "$js"
  perl -0777 -pi -e "s{import\('\./([A-Za-z0-9_/-]+\.js)'\)}{import('./\$1?v=$BUILD')}g" "$js"
  bust "$js" '__BUILD__' "$BUILD"
done
bust "$WEB/pkg/$WASM.js" "new URL('${WASM}_bg.wasm', import.meta.url)" \
                         "new URL('${WASM}_bg.wasm?v=$BUILD', import.meta.url)"
bust "$WEB/index.html"   'src="app.js"'      "src=\"app.js?v=$BUILD\""
bust "$WEB/bt-lab.html"  'src="btlab.js"'    "src=\"btlab.js?v=$BUILD\""
bust "$WEB/arena.html"   'src="arena.js"'    "src=\"arena.js?v=$BUILD\""
bust "$WEB/arena.html"   'href="styles.css"' "href=\"styles.css?v=$BUILD\""
bust "$WEB/clip.html"    'src="clip.js"'     "src=\"clip.js?v=$BUILD\""
bust "$WEB/clip.html"    'href="styles.css"' "href=\"styles.css?v=$BUILD\""
bust "$WEB/bt-lab.html"  'href="styles.css"' "href=\"styles.css?v=$BUILD\""
perl -0777 -pi -e "s/\{hash\}/$BUILD/g" "$WEB/bt-lab.html"
bust "$WEB/index.html"   'href="styles.css"' "href=\"styles.css?v=$BUILD\""
bust "$WEB/index.html"   '__BUILD__'         "$BUILD"
perl -0777 -pi -e "s/\Q__BUILD__\E/$BUILD/g" "$WEB/index.html"
# The build line is a copy string with a {hash} placeholder; fill it too.
perl -0777 -pi -e "s/\{hash\}/$BUILD/g" "$WEB/index.html"

grep -q "app.js?v=$BUILD" "$WEB/index.html" || die "cache-busting did not apply"
grep -q "btlab.js?v=$BUILD" "$WEB/bt-lab.html" || die "the BT Lab's cache-busting did not apply"
grep -q "box-sizing" "$WEB/styles.css"      || die "styles.css lost its rules in packaging"
grep -q "?v=$BUILD" "$WEB/pkg/$WASM.js"     || die "wasm URL not stamped"
if grep -RnoE "from '\./[A-Za-z0-9_/-]+\.js'" "$WEB"/*.js; then
  die "an import above is unstamped, and will be served stale after a deploy"
fi
grep -q "const BUILD = '$BUILD'" "$WEB/app.js" || die "app.js build stamp not applied"

# What the gate compares the page against: the content hash, and the commit it
# was built from. Against the live page, drive.py reads this file back.
COMMIT="${GITHUB_SHA:-$(git -C "$ROOT" rev-parse --verify -q HEAD || echo none)}"
printf '%s %s\n' "$BUILD" "$COMMIT" > "$WEB/build.txt"

# Jekyll would otherwise skip the pkg/ directory and mangle assets.
touch "$WEB/.nojekyll"

say "Done: $WEB (build $BUILD)"
ls -la "$WEB/pkg/${WASM}_bg.wasm" | awk '{printf "    wasm: %.0f KB\n", $5/1024}'
