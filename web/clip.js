// A vertical frame for short clips (Sam, 2026-10-07: "a shorts version of
// some of the craziest stuff that is happening on stream ready to post to
// tik tok"). It plays a stream-like exhibition (two road opponents, a seed)
// from a tick, and draws it on the command of analysis/video/make_short.py,
// one frame at a time: so many ticks forward (fewer for slow motion), with
// a camera that holds the fighters large and can punch in on the cut. It
// draws numbers core sends and decides nothing about the fight.
import init, { copy_json, palette_json, numbers as coreNumbers, tree_json, Lab, Game } from './pkg/vagrancy_wasm.js';
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
// Over the drawn frame: a vision's tint and its sand, blowing left to right
// and a little up, each grain's path fixed by its number and the clip's
// clock; and a flash of the paper.
function overlay(vision, flash) {
  const c = $('stage');
  const g = c.getContext('2d');
  g.save();
  g.setTransform(1, 0, 0, 1, 0, 0);
  if (vision) {
    g.globalAlpha = 0.24;
    g.fillStyle = PAL.costume.vision_tint;
    g.fillRect(0, 0, c.width, c.height);
    g.strokeStyle = PAL.costume.vision_sand;
    g.lineCap = 'round';
    for (let i = 0; i < 320; i += 1) {
      const speed = 0.25 + (i % 7) * 0.08;
      const x = ((i * 7919) % (c.width + 200) + time * speed) % (c.width + 200) - 100;
      const y = ((i * 104729) % c.height - time * speed * 0.18 % c.height + c.height) % c.height;
      const len = 10 + (i % 4) * 8;
      g.globalAlpha = 0.3 + (i % 5) * 0.12;
      g.lineWidth = 2 + (i % 3);
      g.beginPath();
      g.moveTo(x, y);
      g.lineTo(x + len, y - len * 0.18);
      g.stroke();
    }
  }
  if (flash > 0) {
    g.globalAlpha = Math.min(1, flash);
    g.fillStyle = PAL.paper;
    g.fillRect(0, 0, c.width, c.height);
  }
  g.restore();
}

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
  // Or a replay a person played (`replay`, the file's bytes): played back
  // as it was, no pilot in it (Sam, 2026-10-08: "I will play as paul").
  load({ left, right, seed, from, kill, duel = null, guests = null, replay = null }) {
    if (lab) lab.free();
    ids = [left, right];
    const start = () => replay
      ? Game.load_replay(new Uint8Array(replay))
      : duel
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
  // `head` (0 to 1) blends the camera onto the left fighter's head at
  // `headZoom`; `vision` lays blue sand blowing over a tinted scene, and
  // `flash` (0 to 1) washes it with the paper (Sam, 2026-10-08: Paul seeing
  // the fights where he dies, then snapping back to the real one).
  frame({ ticks = 2, dt = 1000 / 30, zoom = 2.3, punch = 0, y = 0.58, showTrees = false, treeScale = 1.5, head = 0, headZoom = 6, vision = false, flash = 0 }) {
    time += dt;
    acc += ticks;
    // What happened in the ticks this frame stepped: blades meeting, and
    // cuts (for the edit's beats on a long fight).
    let clashes = 0, cuts = 0, shields = 0;
    while (acc >= 1) {
      const was = cur.phase;
      // A played take runs on past its recording, so the fall is seen.
      if (lab.run_on) lab.run_on();
      else lab.step(0, 0);
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
    let at = focus && punch > 0 ? [m[0] + (focus[0] - m[0]) * punch, m[1] + (focus[1] - m[1]) * punch] : m;
    let zz = z * (1 + 0.6 * punch);
    if (head > 0) {
      const hp = cur.parts.find((p) => p.fighter === 0 && p.def === 2 && !p.stump);
      if (hp) {
        const q = cur.points[hp.b];
        at = [at[0] + (q[0] - at[0]) * head, at[1] + (q[1] - at[1]) * head];
        zz = zz + (headZoom - zz) * head;
      }
    }
    draw.view({ at, zoom: zz, y: y + (0.5 - y) * head });
    draw.treeScale(treeScale);
    // A replay has no pilots, so no trees to report.
    const report = lab.report ? JSON.parse(lab.report()) : [];
    draw.trees(showTrees ? report.filter((r) => (showTrees.seat === 'both' ? r.seat < 2 : r.seat === showTrees.seat) && r.id).map((r) => ({ seat: r.seat, tree: treeOf(r.id), trace: r, caption: (nd) => t('tree.now', { node: nd.text }) })) : []);
    draw(prev || cur, cur, Math.min(1, acc));
    if (vision || flash > 0) overlay(vision, flash);
    return { tick: lab.tick(), phase: cur.phase, ended: this.ended || null, clashes, cuts, shields };
  },
  caption(key, vars = {}, size = '') {
    $('clip-caption').className = size;
    $('clip-caption').replaceChildren(key ? el('span', { 'data-copy': key }, t(key, vars)) : '');
  },
  // The tick a played replay's fight ends on: where to cut a take.
  endOf(replay) {
    const g = Game.load_replay(new Uint8Array(replay));
    let f = JSON.parse(g.frame());
    while (f.phase === 'fight' && !g.done()) { g.step(0, 0); f = JSON.parse(g.frame()); }
    const tick = g.tick();
    g.free();
    return tick;
  },
  // Across the arena: the invitation to play, and the site, big (Sam,
  // 2026-10-08). Shown with no arguments it is taken away.
  banner(site = null) {
    const b = $('clip-banner');
    b.hidden = !site;
    b.replaceChildren(...(site ? [el('p', { class: 'play', 'data-copy': 'clip.play_free' }, t('clip.play_free')), el('p', { class: 'site', 'data-fill': '' }, site)] : []));
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
