// A vertical frame for short clips (Sam, 2026-10-07: "a shorts version of
// some of the craziest stuff that is happening on stream ready to post to
// tik tok"). It plays a stream-like exhibition (two road opponents, a seed)
// from a tick, and draws it on the command of analysis/video/make_short.py,
// one frame at a time: so many ticks forward (fewer for slow motion), with
// a camera that holds the fighters large and can punch in on the cut. It
// draws numbers core sends and decides nothing about the fight.
import init, { copy_json, palette_json, numbers as coreNumbers, tree_json, Lab } from './pkg/vagrancy_wasm.js';
import { renderer } from './draw.js';

let COPY, N, PAL, draw;
const $ = (id) => document.getElementById(id);
function t(key, vars = {}) {
  const s = key.split('.').reduce((o, k) => (o == null ? o : o[k]), COPY);
  if (typeof s !== 'string') throw new Error(`no copy string at ${key}`);
  const all = { game: COPY.game.name, ...vars };
  return s.replace(/\{([a-z_.]+)\}/g, (m, p) => String(all[p]));
}
function el(tag, attrs = {}, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  for (const k of kids) if (k != null) e.append(k);
  return e;
}

let lab = null, ids = [], prev = null, cur = null, acc = 0, mid = null, time = 0, focus = null, trees = new Map();
const one = () => 1 << N.frac_bits;

function frameNow() {
  prev = cur;
  cur = JSON.parse(lab.frame());
  draw.events(cur);
}
// The fighters' middle, from the frame's attached parts, smoothed so the
// camera drifts and does not jitter.
let spread = 0;
function middle() {
  let sx = 0, sy = 0, n = 0, lo = Infinity, hi = -Infinity;
  for (const p of cur.parts) {
    if (!p.attached) continue;
    const q = cur.points[p.a];
    sx += q[0]; sy += q[1]; n += 1;
    lo = Math.min(lo, q[0]); hi = Math.max(hi, q[0]);
  }
  if (!n) return mid;
  // How far apart the fighters stand, in world units, smoothed like the
  // middle: the camera pulls back to keep both in the frame.
  const s = (hi - lo) / one();
  spread = spread ? spread + (s - spread) * 0.15 : s;
  const m = [sx / n, sy / n];
  mid = mid ? [mid[0] + (m[0] - mid[0]) * 0.15, mid[1] + (m[1] - mid[1]) * 0.15] : m;
  return mid;
}
function treeOf(id) {
  if (!trees.has(id)) {
    const fill = (n) => { n.text = t(n.label.key, n.label.vars); n.children.forEach(fill); return n; };
    trees.set(id, fill(JSON.parse(tree_json(id))));
  }
  return trees.get(id);
}

window.clip = {
  // A match from its start, played on without drawing to tick `from`; the
  // next round starts at once, as on the stream before its round ends.
  // A clip may bring its own duel (core's Lab.duel: each side's weapon, a
  // shield, the rounds to win) and its own guests: costumes, a scene, blade
  // colors and names that are not the road's, for a clip made outside the
  // game (analysis/video/make_short.py).
  load({ left, right, seed, from, kill, duel = null, guests = null }) {
    if (lab) lab.free();
    ids = [left, right];
    const start = () => duel
      ? Lab.duel(seed, N.default_tuning, left, right, duel.weapons[0], duel.weapons[1], !!duel.shield, duel.rounds || N.rounds_to_win, duel.gap || 0)
      : Lab.watch(seed, N.default_tuning, left, right, 'flat');
    draw.guests(guests);
    // The round the cut ends began somewhere before it: a first run to the
    // cut finds where, so the clip never opens in the round before.
    if (kill) {
      const probe = start();
      let f = JSON.parse(probe.frame()), began = 0;
      while (probe.tick() < kill - 1) {
        const was = f.phase;
        probe.step(f.phase !== 'fight' ? N.ready_bit : 0, 0);
        f = JSON.parse(probe.frame());
        if (was !== 'fight' && f.phase === 'fight') began = probe.tick();
      }
      probe.free();
      from = Math.max(from, began + 6);
    }
    lab = start();
    cur = JSON.parse(lab.frame());
    while (lab.tick() < from) {
      lab.step(cur.phase !== 'fight' ? N.ready_bit : 0, 0);
      cur = JSON.parse(lab.frame());
    }
    prev = null; acc = 0; mid = null; focus = null; spread = 0;
    draw.reset();
    const named = (k, id) => (guests && guests.names ? guests.names[k] : t(`opponents.${id}.name`));
    $('clip-names').replaceChildren(
      el('span', { 'data-fill': '', class: 'side-0' }, named(0, left)), ' ',
      el('span', { 'data-copy': 'arena.versus' }, t('arena.versus')), ' ',
      el('span', { 'data-fill': '', class: 'side-1' }, named(1, right)));
    return { tick: lab.tick(), phase: cur.phase };
  },
  // `ticks` forward (a fraction is carried), and the frame drawn at `dt` ms
  // of clip time. The next round is not asked for, so a round's end plays
  // out. Returns what the page knows.
  frame({ ticks = 2, dt = 1000 / 30, zoom = 2.3, punch = 0, y = 0.58, showTrees = false, treeScale = 1.5 }) {
    time += dt;
    acc += ticks;
    // What happened in the ticks this frame stepped: blades meeting, and
    // cuts (for the edit's beats on a long fight).
    let clashes = 0, cuts = 0, shields = 0;
    while (acc >= 1) {
      const was = cur.phase;
      lab.step(0, 0);
      frameNow();
      for (const e of cur.events) {
        if (e.Clash) clashes += 1;
        if (e.Cut && e.Cut.spilled) cuts += 1;
        if (e.Shield) shields += 1;
      }
      acc -= 1;
      if (was === 'fight' && cur.phase !== 'fight') {
        const said = JSON.parse(lab.phase_text(''));
        focus = said && said.focus;
        this.ended = { tick: lab.tick(), said };
      }
    }
    const m = middle();
    // A punch blends the camera's point toward the cut and closes in. The
    // zoom is held back far enough to keep both fighters in the frame.
    const half = N.arena_half / one();
    const fit = (0.8 * 2 * half) / Math.max(1, spread + 80);
    const z = Math.min(zoom, Math.max(1, fit));
    const at = focus && punch > 0 ? [m[0] + (focus[0] - m[0]) * punch, m[1] + (focus[1] - m[1]) * punch] : m;
    draw.view({ at, zoom: z * (1 + 0.6 * punch), y });
    draw.treeScale(treeScale);
    const report = JSON.parse(lab.report());
    draw.trees(showTrees ? report.filter((r) => (showTrees.seat === 'both' ? r.seat < 2 : r.seat === showTrees.seat) && r.id).map((r) => ({ seat: r.seat, tree: treeOf(r.id), trace: r, caption: (nd) => t('tree.now', { node: nd.text }) })) : []);
    draw(prev || cur, cur, Math.min(1, acc));
    return { tick: lab.tick(), phase: cur.phase, ended: this.ended || null, clashes, cuts, shields };
  },
  caption(key, vars = {}, size = '') {
    $('clip-caption').className = size;
    $('clip-caption').replaceChildren(key ? el('span', { 'data-copy': key }, t(key, vars)) : '');
  },
  // A guest clip's own words, from its file rather than the game's copy.
  say(text, size = '') {
    $('clip-caption').className = size;
    $('clip-caption').replaceChildren(text ? el('span', { 'data-fill': '' }, text) : '');
  },
  // Every costume and scene picture the last frame asked for has loaded.
  picturesReady() { return draw.picturesReady(); },
  ended: null,
};

async function main() {
  await init();
  COPY = JSON.parse(copy_json());
  N = JSON.parse(coreNumbers());
  PAL = JSON.parse(palette_json());
  draw = renderer($('stage'), PAL, N);
  draw.clock(() => time);
  document.body.dataset.ready = '1';
}
main();
