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
  load({ left, right, seed, from }) {
    if (lab) lab.free();
    ids = [left, right];
    lab = Lab.watch(seed, N.default_tuning, left, right, 'flat');
    cur = JSON.parse(lab.frame());
    while (lab.tick() < from) {
      lab.step(cur.phase !== 'fight' ? N.ready_bit : 0, 0);
      cur = JSON.parse(lab.frame());
    }
    prev = null; acc = 0; mid = null; focus = null; spread = 0;
    draw.reset();
    $('clip-names').replaceChildren(
      el('span', { 'data-fill': '', class: 'side-0' }, t(`opponents.${left}.name`)), ' ',
      el('span', { 'data-copy': 'arena.versus' }, t('arena.versus')), ' ',
      el('span', { 'data-fill': '', class: 'side-1' }, t(`opponents.${right}.name`)));
    return { tick: lab.tick(), phase: cur.phase };
  },
  // `ticks` forward (a fraction is carried), and the frame drawn at `dt` ms
  // of clip time. The next round is not asked for, so a round's end plays
  // out. Returns what the page knows.
  frame({ ticks = 2, dt = 1000 / 30, zoom = 2.3, punch = 0, y = 0.58, showTrees = false, treeScale = 1.5 }) {
    time += dt;
    acc += ticks;
    while (acc >= 1) {
      const was = cur.phase;
      lab.step(0, 0);
      frameNow();
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
    draw.trees(showTrees ? report.filter((r) => r.seat === showTrees.seat && r.id).map((r) => ({ seat: r.seat, tree: treeOf(r.id), trace: r, caption: (nd) => t('tree.now', { node: nd.text }) })) : []);
    draw(prev || cur, cur, Math.min(1, acc));
    return { tick: lab.tick(), phase: cur.phase, ended: this.ended || null };
  },
  caption(key, vars = {}, size = '') {
    $('clip-caption').className = size;
    $('clip-caption').replaceChildren(key ? el('span', { 'data-copy': key }, t(key, vars)) : '');
  },
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
