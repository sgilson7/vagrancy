// The BT Lab (Sam, 2026-10-06): a classroom demo of how a behavior tree
// becomes key presses, built on the game's own pilots. Like the game's page
// it draws what core sends and decides nothing: the tree, what ran, and the
// keys and the reasons for them come from core (crates/pilot/src/moves.rs,
// crates/wasm Lab); every word is a copy string.
import init, {
  copy_json, palette_json, numbers as coreNumbers, controls_json, tree_json, maps_json,
  Lab, lab_roster_json, lab_moves_json, bubbles_step, lab_check, lab_describe_json, lab_conditions_json,
} from './pkg/vagrancy_wasm.js';
import { renderer } from './draw.js';
import { listen, bits, keyName } from './keys.js';

const BUILD = '__BUILD__';
let COPY, N, PAL, CONTROLS, ROSTER, MOVES, EDITOR, ACTION_BITS, draw, WASM;

const $ = (id) => document.getElementById(id);
function t(key, vars = {}) {
  const s = key.split('.').reduce((o, k) => (o == null ? o : o[k]), COPY);
  if (typeof s !== 'string') throw new Error(`no copy string at ${key}`);
  const all = { game: COPY.game.name, hash: BUILD, ...vars };
  return s.replace(/\{([a-z_.]+)\}/g, (m, p) => {
    if (!(p in all)) throw new Error(`no value for {${p}} in ${key}`);
    return String(all[p]);
  });
}
function el(tag, attrs = {}, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === 'on') for (const [ev, fn] of Object.entries(v)) e.addEventListener(ev, fn);
    else if (v !== undefined && v !== null && v !== false) e.setAttribute(k, v === true ? '' : v);
  }
  for (const k of kids) if (k != null && k !== false) e.append(k);
  return e;
}
const say = (key, vars, attrs = {}) => el('p', { 'data-copy': key, ...attrs }, t(key, vars));
const button = (key, onclick, vars, attrs = {}) => el('button', { type: 'button', 'data-copy': key, on: { click: onclick }, ...attrs }, t(key, vars));
// A value filled from data, not a sentence: a key's name, a number.
const fill = (text, attrs = {}) => el('span', { 'data-fill': '', ...attrs }, String(text));
const name = (id) => t(`opponents.${id}.name`);

// --- what is running ---------------------------------------------------------------

// The lab starts at an eighth of full speed (Sam, 2026-10-07), slow enough
// to follow a tree; a fight of your own is played at full speed.
const SLOW_DEFAULT = 0.125;
const S = {
  mode: 'watch', left: 'lamplighter', right: 'drover', opp: 'lamplighter', map: 'flat',
  inspect: 1, speed: SLOW_DEFAULT, paused: false, floating: false,
};
let lab = null;
let prev = null, cur = null, report = [], acc = 0, last = 0, stepOnce = false;
// The last few seconds of each seat's keys and running move, newest last.
let history = [];
const HISTORY_TICKS = 180;

function start(custom = null) {
  const seed = (Math.random() * 0xffffffff) >>> 0;
  const tuning = N.default_tuning;
  S.custom = custom;
  TREES.delete('custom');
  if (custom === 'watch') { lab = Lab.watch_custom(seed, tuning, JSON.stringify(EDIT), S.right, S.map); S.mode = 'watch'; S.inspect = 0; }
  else if (custom === 'play') { lab = Lab.play_custom(seed, tuning, JSON.stringify(EDIT), S.map); S.mode = 'play'; }
  else lab = S.mode === 'watch' ? Lab.watch(seed, tuning, S.left, S.right, S.map) : Lab.play(seed, tuning, S.opp);
  // A fight of your own plays at full speed; watching starts slow.
  if (S.mode === 'play') { S.inspect = 1; S.speed = 1; }
  cur = JSON.parse(lab.frame());
  prev = null;
  history = [];
  // What each seat would report before its first tick, so the panels have
  // a tree and keys to show while the lab waits off screen.
  report = JSON.parse(lab.report());
  acc = 0;
  draw.reset();
  buildTree();
  renderTransport();
  renderPanels();
}

function tickOnce() {
  // In a fight of your own, the next round starts when the last one is
  // read; the pilots press "ready" themselves.
  // The lab moves on to the next round by itself, in either mode.
  const ready = cur && cur.phase !== 'fight' ? N.ready_bit : 0;
  const mine = S.mode === 'play' ? bits(CONTROLS.solo, ACTION_BITS) | ready : ready;
  lab.step(mine, 0);
  prev = cur;
  cur = JSON.parse(lab.frame());
  draw.events(cur);
  report = JSON.parse(lab.report());
  history.push({ tick: lab.tick(), seats: report.map((r) => ({ seat: r.seat, keys: r.keys, mv: (r.explain[0] || {}).mv || null, t: (r.explain[0] || {}).t || 0 })) });
  if (history.length > HISTORY_TICKS) history.shift();
}

function loop(now) {
  // The next frame is asked for first, so a frame that throws shows its
  // error and the next one still runs: an uncaught error here once froze
  // each fight on the page.
  requestAnimationFrame(loop);
  const tickMs = 1000 / N.ticks_per_second;
  if (lab) {
    let n = 0;
    if (!S.paused && labVisible) {
      acc += Math.min(now - (last || now), 250) * S.speed;
      while (acc >= tickMs && n < 8) { tickOnce(); acc -= tickMs; n += 1; }
    } else if (stepOnce) {
      tickOnce(); n = 1; stepOnce = false; acc = 0;
    }
    if (n) renderPanels();
    draw.trees(S.floating ? report.filter((r) => r.id).map((r) => ({ seat: r.seat, tree: treeOf(r.id), trace: r, caption: (nd) => t('tree.now', { node: nd.text }) })) : []);
    draw(prev || cur, cur, S.paused ? 1 : Math.min(1, acc / tickMs));
  }
  const dt = now - (last || now);
  last = now;
  demoTick(now, dt);
}

// --- the trees ---------------------------------------------------------------------

const TREES = new Map();
function treeOf(id) {
  if (id === 'custom' && !TREES.has(id)) {
    const fillText = (n) => { n.text = t(n.label.key, n.label.vars); n.children.forEach(fillText); return n; };
    TREES.set(id, fillText(JSON.parse(lab_describe_json(JSON.stringify(EDIT)))));
  }
  if (!TREES.has(id)) {
    const fillText = (n) => { n.text = t(n.label.key, n.label.vars); n.children.forEach(fillText); return n; };
    TREES.set(id, fillText(JSON.parse(tree_json(id))));
  }
  return TREES.get(id);
}

// Columns and rows, as the arena's floating trees lay them out (draw.js):
// a selector, parallel or repeat spreads its children across; a sequence
// stacks its steps under itself.
const across = (n) => n.kind === 'selector' || n.kind === 'parallel' || n.kind === 'repeat';
function lay(n, col, row, out) {
  if (!n.children.length) { out.set(n.id, { col, row, n }); return 1; }
  if (across(n)) {
    let c = col;
    for (const k of n.children) c += lay(k, c, row + 1, out);
    out.set(n.id, { col: col + (c - col - 1) / 2, row, n });
    return c - col;
  }
  out.set(n.id, { col, row, n });
  let r = row + 1, w = 1;
  for (const k of n.children) {
    const sub = new Map();
    w = Math.max(w, lay(k, col, r, sub));
    for (const [id, v] of sub) out.set(id, v);
    r = Math.max(...[...sub.values()].map((v) => v.row)) + 1;
  }
  return w;
}

const SVG = 'http://www.w3.org/2000/svg';
const svg = (tag, attrs = {}) => {
  const e = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  return e;
};
const BOX_W = 168, BOX_H = 40, COL = 180, ROW = 56;
let nodeEls = new Map();

function inspectedId() {
  if (S.custom === 'watch') return S.inspect === 0 ? 'custom' : S.right;
  if (S.custom === 'play') return 'custom';
  return S.mode === 'watch' ? (S.inspect === 0 ? S.left : S.right) : S.opp;
}
// A road opponent's name, or the written tree's own, which is the writer's
// words and so drawn as data.
const fighterName = (id) => (id === 'custom' ? (EDIT.name || t('btlab.editor.unnamed')) : name(id));

function buildTree() {
  const panel = $('tree-panel');
  const id = inspectedId();
  const tree = treeOf(id);
  const pos = new Map();
  const cols = lay(tree, 0, 0, pos);
  const rows = Math.max(...[...pos.values()].map((v) => v.row)) + 1;
  const W = cols * COL + 20, H = rows * ROW + 20;
  const root = svg('svg', { viewBox: `0 0 ${W} ${H}`, width: W, height: H, class: 'bt-svg', role: 'img', 'aria-label': t('btlab.tree.heading', { opponent: fighterName(id) }) });
  const at = (nid) => { const v = pos.get(nid); return [10 + v.col * COL, 10 + v.row * ROW]; };
  const edges = svg('g', { class: 'edges' });
  for (const v of pos.values()) {
    for (const k of v.n.children) {
      const [x1, y1] = at(v.n.id), [x2, y2] = at(k.id);
      const d = across(v.n)
        ? `M${x1 + BOX_W / 2},${y1 + BOX_H} L${x2 + BOX_W / 2},${y2}`
        : `M${x1 + 10},${y1 + BOX_H} L${x1 + 10},${y2 + BOX_H / 2} L${x2},${y2 + BOX_H / 2}`;
      edges.append(svg('path', { d, 'data-from': v.n.id, 'data-to': k.id }));
    }
  }
  root.append(edges);
  nodeEls = new Map();
  for (const v of pos.values()) {
    const [x, y] = at(v.n.id);
    const g = svg('g', { class: `node kind-${v.n.kind}${v.n.interrupt ? ' interrupt' : ''}`, 'data-id': v.n.id, transform: `translate(${x},${y})` });
    g.append(svg('rect', { width: BOX_W, height: BOX_H, rx: v.n.kind === 'condition' ? 18 : 4 }));
    const im = svg('image', { href: `icons/${v.n.icon}.png`, x: 4, y: 6, width: 28, height: 28 });
    g.append(im);
    const fo = svg('foreignObject', { x: 36, y: 2, width: BOX_W - 40, height: BOX_H - 4 });
    const div = document.createElement('div');
    div.className = 'node-text';
    div.textContent = v.n.text;
    div.setAttribute('data-fill', '');
    fo.append(div);
    g.append(fo);
    nodeEls.set(v.n.id, g);
    root.append(g);
  }
  const legend = el('ul', { class: 'bt-legend' },
    ...['active', 'held', 'failed', 'interrupt'].map((k) => el('li', { class: `lg-${k}` }, el('span', { class: 'swatch' }), t(`btlab.legend.${k}`))));
  panel.replaceChildren(
    el('h2', { 'data-copy': 'btlab.tree.heading' }, t('btlab.tree.heading', { opponent: fighterName(id) })),
    say('btlab.tree.desc', {}, { class: 'desc' }),
    legend,
    el('div', { class: 'bt-scroll' }, root));
}

function lightTree(r) {
  if (!r) return;
  const active = new Set(r.active), held = new Set(r.held), failed = new Set(r.failed);
  for (const [nid, g] of nodeEls) {
    g.classList.toggle('active', active.has(nid));
    g.classList.toggle('held', held.has(nid) && !active.has(nid));
    g.classList.toggle('failed', failed.has(nid) && !active.has(nid));
  }
  for (const p of $('tree-panel').querySelectorAll('path')) {
    p.classList.toggle('active', active.has(+p.dataset.from) && active.has(+p.dataset.to));
  }
}

// --- from the action to the keys ----------------------------------------------------

// Each input bit, named as Settings names it, with the key it is bound to
// for one player; the second pair of arms has no key.
function keyRows(many) {
  const rows = N.actions.map(([action, bit]) => ({ bit, label: t(`settings.keys.actions.${action}`), key: CONTROLS.solo[action] ? keyName(CONTROLS.solo[action]) : '' }));
  if (many) {
    ['shoulder2_up', 'shoulder2_down', 'elbow2_in', 'elbow2_out'].forEach((k, i) => rows.push({ bit: MOVES.arms2[i], label: t(`btlab.keys.arms2.${k}`), key: '' }));
  }
  return rows;
}
const keysOf = (bitsSet, many) => keyRows(many).filter((r) => bitsSet & r.bit);
const caps = (bitsSet, many) => {
  const ks = keysOf(bitsSet, many);
  return ks.length ? el('span', { class: 'caps' }, ...ks.map((r) => el('kbd', {}, fill(r.label)))) : say('btlab.key.none', {}, { class: 'none' });
};
const mrad = (n) => fill(t('btlab.mrad', { n }));

function recipeOf(mv) {
  return (MOVES.moves.find((m) => m.id === mv) || {}).recipe;
}

function explainBlock(e, many) {
  const kids = [];
  if (e.part) kids.push(el('h4', { 'data-copy': e.part }, t(e.part)));
  if (!e.mv) {
    kids.push(say('btlab.idle'));
    return el('div', { class: 'explain' }, ...kids);
  }
  kids.push(say('btlab.now', { move: t(`tree.act.${e.mv}`), t: e.t }, { class: 'now' }));
  const r = recipeOf(e.mv);
  if (r && r.kind === 'script') {
    kids.push(say('btlab.script.intro', {}, { class: 'desc' }));
    const rows = r.beats.map((b, i) => el('tr', { class: i === e.beat ? 'on' : '' },
      el('td', {}, fill(t('btlab.script.span', { from: b.from, to: b.to }))),
      el('td', {}, caps(b.keys, many)),
      el('td', {}, fill(t(`btlab.step.${b.step}`)))));
    kids.push(el('table', { class: 'recipe' },
      el('thead', {}, el('tr', {}, ...['ticks', 'keys', 'step'].map((h) => el('th', { 'data-copy': `btlab.script.${h}` }, t(`btlab.script.${h}`))))),
      el('tbody', {}, ...rows)));
  } else if (r && r.kind === 'pose' && e.pose) {
    const p = e.pose;
    kids.push(say('btlab.pose.intro', { shoulder: p.shoulder_deg, elbow: p.elbow_deg }, { class: 'desc' }));
    kids.push(say('btlab.pose.rule', { share: MOVES.pose.share, max: MOVES.pose.max, band: MOVES.pose.band }, { class: 'desc' }));
    const row = (k, j) => el('tr', {},
      el('th', { 'data-copy': `btlab.pose.${k}` }, t(`btlab.pose.${k}`)),
      el('td', {}, mrad(j.gap)), el('td', {}, mrad(j.aim)), el('td', {}, mrad(j.speed)),
      el('td', {}, caps(j.key, many)));
    kids.push(el('table', { class: 'recipe' },
      el('thead', {}, el('tr', {}, ...['joint', 'gap', 'aim', 'turn', 'key'].map((h) => el('th', { 'data-copy': `btlab.pose.${h}` }, t(`btlab.pose.${h}`))))),
      el('tbody', {}, row('shoulder', p.shoulder), row('elbow', p.elbow))));
  } else if (r && r.kind === 'search' && e.search) {
    const s = e.search;
    kids.push(say('btlab.search.intro', { horizon_ms: Math.round(s.horizon * 1000 / N.ticks_per_second) }, { class: 'desc' }));
    if (s.tried.length) {
      kids.push(el('table', { class: 'recipe' },
        el('thead', {}, el('tr', {}, ...['keys', 'score'].map((h) => el('th', { 'data-copy': `btlab.search.${h}` }, t(`btlab.search.${h}`))))),
        el('tbody', {}, ...s.tried.map(([k, sc]) => el('tr', { class: k === s.chosen ? 'on' : '' }, el('td', {}, caps(k, many)), el('td', {}, fill(sc)))))));
    }
    kids.push(say('btlab.search.keeps', { n: s.keeps }, { class: 'desc' }));
  } else if (r && r.kind === 'throw') {
    kids.push(say('btlab.throw.intro', {}, { class: 'desc' }));
    const steps = ['throw_settle', 'throw_wind', 'throw_watch', 'throw_release'];
    kids.push(el('ol', { class: 'throw-steps' }, ...steps.map((s, i) => el('li', { class: i === e.throw_step ? 'on' : '', 'data-copy': `tree.act.${s}` }, t(`tree.act.${s}`)))));
    if (e.throw_step === 2) kids.push(say(e.window ? 'btlab.throw.open' : 'btlab.throw.shut', { cm: MOVES.throw_window_cm }));
  }
  if (e.step_keys) kids.push(say('btlab.step_added', {}, { class: 'desc' }));
  kids.push(el('h4', { 'data-copy': 'btlab.keys.from_this' }, t('btlab.keys.from_this')), caps(e.keys, many));
  return el('div', { class: 'explain' }, ...kids);
}

function renderPanels() {
  const r = report.find((x) => x.seat === S.inspect) || report.find((x) => x.seat === 1);
  lightTree(r);
  const panel = $('keys-panel');
  const many = !!(r && r.explain.length > 1);
  const kids = [el('h2', { 'data-copy': 'btlab.keys.heading' }, t('btlab.keys.heading'))];
  if (r) {
    for (const e of r.explain) kids.push(explainBlock(e, many));
    kids.push(el('h3', { 'data-copy': 'btlab.keys.pressed' }, t('btlab.keys.pressed')));
    kids.push(el('div', { class: 'keyboard' }, ...keyRows(many).map((k) => el('div', { class: `cap${r.keys & k.bit ? ' down' : ''}`, 'data-bit': k.bit },
      el('span', { class: 'cap-key', 'data-fill': '' }, k.key || '·'), el('span', { class: 'cap-label', 'data-fill': '' }, k.label)))));
  }
  panel.replaceChildren(...kids);
  renderTimeline(many);
  renderHud();
}

function renderHud() {
  if (!cur) return;
  const names = S.custom === 'watch' ? [fighterName('custom'), name(S.right)]
    : S.custom === 'play' ? [t('fighters.left.name'), fighterName('custom')]
    : S.mode === 'watch' ? [name(S.left), name(S.right)] : [t('fighters.left.name'), name(S.opp)];
  $('hud').replaceChildren(
    el('span', { 'data-copy': 'btlab.tick' }, t('btlab.tick', { tick: lab.tick() })), ' ',
    el('span', { 'data-copy': 'hud.round' }, t('hud.round', { round: cur.round })), ' ',
    el('span', { 'data-copy': 'hud.score' }, t('hud.score', { left_name: names[0], left_wins: cur.wins[0], right_name: names[1], right_wins: cur.wins[1] })));
}

// --- the timeline: each key, the last few seconds ----------------------------------------

function renderTimeline(many) {
  const panel = $('timeline-panel');
  let cv = panel.querySelector('canvas');
  if (!cv) {
    cv = el('canvas', { width: 900, height: 300, 'aria-label': t('btlab.timeline.heading', { seconds: HISTORY_TICKS / N.ticks_per_second }) });
    panel.replaceChildren(
      el('h2', { 'data-copy': 'btlab.timeline.heading' }, t('btlab.timeline.heading', { seconds: HISTORY_TICKS / N.ticks_per_second })),
      say('btlab.timeline.desc', {}, { class: 'desc' }), cv);
  }
  const rows = keyRows(many);
  const ctx = cv.getContext('2d');
  const LABEL = 230, top = 22, rh = Math.min(24, (cv.height - top) / rows.length), cw = (cv.width - LABEL) / HISTORY_TICKS;
  ctx.fillStyle = PAL.paper; ctx.fillRect(0, 0, cv.width, cv.height);
  ctx.font = '12px Georgia, serif';
  rows.forEach((row, i) => {
    ctx.fillStyle = PAL.line;
    ctx.fillText(row.label, 4, top + i * rh + rh * 0.7);
    ctx.strokeStyle = PAL.meter_back;
    ctx.beginPath(); ctx.moveTo(LABEL, top + (i + 1) * rh); ctx.lineTo(cv.width, top + (i + 1) * rh); ctx.stroke();
  });
  // A thin line where a move starts (its tick 0), named when it is a
  // different move from the one named last.
  let lastLabel = null;
  history.forEach((h, x) => {
    const s = h.seats.find((q) => q.seat === S.inspect) || h.seats.find((q) => q.seat === 1);
    if (!s) return;
    const px = LABEL + x * cw;
    if (s.mv && s.t === 0) {
      ctx.strokeStyle = PAL.fighters.right.body;
      ctx.beginPath(); ctx.moveTo(px, top - 4); ctx.lineTo(px, cv.height); ctx.stroke();
      if (s.mv !== lastLabel) { ctx.fillStyle = PAL.fighters.right.stripe; ctx.fillText(t(`tree.act.${s.mv}`), px + 3, 14); lastLabel = s.mv; }
    }
    rows.forEach((row, i) => {
      if (s.keys & row.bit) { ctx.fillStyle = PAL.focus; ctx.fillRect(px, top + i * rh + 3, Math.max(1, cw), rh - 6); }
    });
  });
}

// --- the controls -------------------------------------------------------------------------

const SPEEDS = [[1, 'watch.speed.full.label'], [0.5, 'watch.speed.half.label'], [0.25, 'watch.speed.quarter.label'], [0.125, 'btlab.speed.eighth.label']];

function renderTransport() {
  const box = $('transport');
  const pause = button(S.paused ? 'btlab.resume.label' : 'btlab.pause.label', () => { S.paused = !S.paused; renderTransport(); }, {}, { id: 'lab-pause' });
  const step = button('btlab.step.label', () => { stepOnce = true; }, {}, { id: 'lab-step', disabled: !S.paused });
  const speeds = SPEEDS.map(([v, key]) => {
    const b = button(key, () => { S.speed = v; renderTransport(); });
    b.classList.toggle('picked', S.speed === v);
    b.setAttribute('aria-pressed', String(S.speed === v));
    return b;
  });
  const insp = S.mode === 'watch'
    ? el('p', {}, el('label', { 'data-copy': 'btlab.inspect.label' }, t('btlab.inspect.label')), ' ',
      ...[0, 1].map((seat) => {
        const who = S.custom === 'watch' ? (seat === 0 ? 'custom' : S.right) : (seat === 0 ? S.left : S.right);
        const b = button('btlab.inspect.seat', () => { S.inspect = seat; buildTree(); renderTransport(); renderPanels(); }, { opponent: fighterName(who) });
        b.classList.toggle('picked', S.inspect === seat);
        return b;
      }))
    : null;
  const floating = el('label', {}, el('input', { type: 'checkbox', id: 'lab-floating', checked: S.floating, on: { change: (e) => { S.floating = e.target.checked; } } }), ' ', t('btlab.floating'));
  box.replaceChildren(
    el('div', { class: 'actions' }, pause, step, ...speeds),
    insp, el('p', {}, floating),
    say('btlab.shortcuts', {}, { class: 'desc' }));
}

function roster() {
  // Grouped by row of the road, as arcade mode lays them out.
  const byRow = new Map();
  for (const r of ROSTER) {
    if (!byRow.has(r.level)) byRow.set(r.level, []);
    byRow.get(r.level).push(r);
  }
  return [...byRow.entries()].sort((a, b) => a[0] - b[0]);
}
function picker(id, labelKey, value, onChange) {
  const sel = el('select', { id, on: { change: () => onChange(sel.value) } },
    ...roster().map(([row, rs]) => el('optgroup', { label: t('btlab.row', { row }) },
      ...rs.map((r) => el('option', { value: r.id }, name(r.id))))));
  sel.value = value;
  return el('p', {}, el('label', { for: id, 'data-copy': labelKey }, t(labelKey)), ' ', sel);
}

function renderSetup() {
  const box = $('setup');
  const modes = ['watch', 'play'].map((m) => {
    const b = button(`btlab.mode.${m}`, () => { S.mode = m; renderSetup(); });
    b.classList.toggle('picked', S.mode === m);
    return b;
  });
  const maps = JSON.parse(maps_json()).maps.map((m) => m.id);
  const map = el('select', { id: 'lab-map', on: { change: () => { S.map = map.value; } } }, ...maps.map((m) => el('option', { value: m }, t(`maps.${m}.name`))));
  map.value = S.map;
  const kids = [el('h2', { 'data-copy': 'btlab.setup.heading' }, t('btlab.setup.heading')), el('div', { class: 'actions' }, ...modes)];
  if (S.mode === 'watch') {
    kids.push(picker('lab-left', 'btlab.left', S.left, (v) => { S.left = v; }), picker('lab-right', 'btlab.right', S.right, (v) => { S.right = v; }),
      el('p', {}, el('label', { for: 'lab-map', 'data-copy': 'local.map' }, t('local.map')), ' ', map));
  } else {
    kids.push(picker('lab-opp', 'btlab.opponent', S.opp, (v) => { S.opp = v; }), say('btlab.play_keys', keyVars(), { class: 'desc' }));
  }
  kids.push(el('div', { class: 'actions' }, button('btlab.start.label', start, {}, { id: 'lab-start' })));
  box.replaceChildren(...kids);
}
function keyVars() {
  const v = {};
  for (const [action, code] of Object.entries(CONTROLS.solo)) v[`key.${action}`] = keyName(code);
  return v;
}

// --- the lesson: pages to scroll through (Sam, 2026-10-07) -----------------------------

// Each page puts the lecture's idea (copy btlab.lesson.<id>) beside a fight
// that shows it, drawn with the trees over the fighters. `demo` is what the
// page plays; `lab` is what "Open this in the lab below" sets up there.
// Lecture figures use the lecture's drawing: ovals for composites, boxes for
// conditions and actions, diamonds for decorators.
const leaf = (label) => ({ label });
const node = (kind, ...kids) => ({ kind, kids });
const FIGURES = {
  tasks: [node('seq', leaf('opp_near'), leaf('swing'))],
  sequence: [node('seq', leaf('move_to_door'), leaf('unlock'), leaf('open'), leaf('enter'))],
  selector: [node('sel', leaf('cover'), leaf('flee'), leaf('fight'))],
  factor: [
    node('sel', node('seq', leaf('door_open'), leaf('enter')), node('seq', leaf('move_to_door'), leaf('unlock'), leaf('open'), leaf('enter'))),
    node('seq', node('sel', leaf('door_open'), node('seq', leaf('move_to_door'), leaf('open'))), leaf('enter')),
  ],
  random: [{ ...node('rsel', leaf('smoke'), leaf('patrol'), leaf('chat')), weights: ['rare', 'common', 'sometimes'] }],
  decorator: [node('until_fail', node('seq', leaf('visible'), leaf('shoot')))],
  parallel: [node('par', leaf('reload'), leaf('take_cover'))],
  script: [node('seq', leaf('pathfind'), leaf('follow'))],
};
// The trees the lesson writes in the editor's own shape (content::custom).
const FACTOR_TREE = { reaction_ticks: 10, rules: [
  { if: ['me_down'], do: 'stand', interrupt: true },
  { if: [{ gap_below: 200 }, { chance: 40 }], do: 'overhead' },
  { if: [{ gap_below: 200 }], do: 'thrust' },
  { if: [{ gap_above: 240 }], do: 'approach' },
  { do: 'guard' },
] };
const RANDOM_PCT = [25, 50];
const RANDOM_TREE = { reaction_ticks: 10, rules: [
  { if: ['me_down'], do: 'stand', interrupt: true },
  { if: [{ gap_below: 220 }, { chance: RANDOM_PCT[0] }], do: 'overhead' },
  { if: [{ gap_below: 220 }, { chance: RANDOM_PCT[1] }], do: 'low_sweep' },
  { if: [{ gap_below: 220 }], do: 'thrust' },
  { if: [{ gap_above: 260 }], do: 'approach' },
  { do: 'guard' },
] };
// Each page rings, in the tree over its left fighter, the nodes it is about
// (Sam, 2026-10-07: "each of the slides mentions a type of node ... which
// should be present in the simulation to the right"); the gate checks each
// page's fighter has one. Every page plays at an eighth of full speed.
const SLOW = 0.125;
const kindIs = (k) => (n) => n.kind === k;
const labelIs = (k) => (n) => n.label.key === k;
const LESSON = [
  { id: 'tasks', ring: kindIs('condition'), demo: { left: 'drover', right: 'scarecrow' } },
  { id: 'sequence', ring: kindIs('sequence'), demo: { left: 'cooper', right: 'drover' } },
  { id: 'selector', ring: kindIs('selector'), demo: { left: 'lamplighter', right: 'drover' } },
  { id: 'running', ring: (n) => !!n.interrupt, demo: { left: 'dyer', right: 'lamplighter' } },
  { id: 'factor', ring: labelIs('tree.cond.gap_below'), demo: { tree: FACTOR_TREE, right: 'drover' } },
  { id: 'random', ring: labelIs('tree.cond.chance'), demo: { tree: RANDOM_TREE, right: 'drover' }, vars: { first_pct: RANDOM_PCT[0], second_pct: RANDOM_PCT[1] } },
  { id: 'decorator', ring: kindIs('repeat'), demo: { left: 'thresher', right: 'scarecrow' } },
  { id: 'parallel', ring: kindIs('parallel'), demo: { left: 'local_deity', right: 'drover' } },
  { id: 'chance', ring: labelIs('tree.cond.chance'), demo: { left: 'smith', right: 'courier', replay: true } },
  { id: 'script', ring: labelIs('tree.act.overhead'), demo: { left: 'hay_mower', right: 'scarecrow' }, lab: { until: 'overhead' } },
  { id: 'pose', beyond: true, ring: labelIs('tree.act.high_guard'), demo: { left: 'lamplighter', right: 'scarecrow' }, lab: { until: 'high_guard' } },
  { id: 'search', beyond: true, ring: kindIs('search'), demo: { left: 'archivist', right: 'drover' }, lab: { until: 'search' } },
  { id: 'throw', beyond: true, ring: labelIs('tree.act.throw'), demo: { left: 'harpooner', right: 'drover' }, lab: { until: 'throw' } },
  { id: 'yours', opp: 'lamplighter' },
];
for (const pg of LESSON) if (pg.demo) pg.demo.speed = SLOW;
// The page's left tree, and the ids of the nodes it rings in it.
function pageTree(pg) {
  return pg.demo.tree ? describedTree(pg.id, pg.demo.tree) : treeOf(pg.demo.left);
}
function ringedIds(pg) {
  const out = new Set();
  const walk = (n) => { if (pg.ring(n)) out.add(n.id); n.children.forEach(walk); };
  walk(pageTree(pg));
  return out;
}
// The lab's set-up for a page: the page's two fighters, the left one inspected.
const labStep = (pg) => ({ mode: 'watch', left: pg.demo.left, right: pg.demo.right, inspect: 0, speed: pg.demo.speed, ...(pg.lab || {}) });

const FIG_COL = 150, FIG_ROW = 84, LEAF_W = 136, LEAF_H = 44;
const SYMBOL = { seq: '→', sel: '?', rsel: '~?', par: '⇉' };
function figure(spec, ariaKey) {
  const pos = new Map();
  let next = 0, depth = 0;
  const place = (n, row) => {
    depth = Math.max(depth, row);
    if (!n.kids) { pos.set(n, { x: next++, y: row }); return; }
    n.kids.forEach((k) => place(k, row + 1));
    const xs = n.kids.map((k) => pos.get(k).x);
    pos.set(n, { x: (Math.min(...xs) + Math.max(...xs)) / 2, y: row });
  };
  place(spec, 0);
  const W = next * FIG_COL, H = (depth + 1) * FIG_ROW;
  const root = svg('svg', { viewBox: `0 0 ${W} ${H}`, class: 'lecture-fig', role: 'img', 'aria-label': t(ariaKey) });
  const at = (n) => { const p = pos.get(n); return [p.x * FIG_COL + FIG_COL / 2, p.y * FIG_ROW + LEAF_H / 2 + 6]; };
  const draw1 = (n) => {
    const [x, y] = at(n);
    (n.kids || []).forEach((k, i) => {
      const [x2, y2] = at(k);
      root.append(svg('line', { x1: x, y1: y + 18, x2, y2: y2 - LEAF_H / 2, class: 'fig-edge' }));
      if (spec.weights && n === spec) {
        // Near the child's end of the edge, on the outside of it.
        const wx = x + (x2 - x) * 0.6, wy = y + 18 + (y2 - LEAF_H / 2 - y - 18) * 0.6;
        const side = x2 < x - 1 ? 'end' : x2 > x + 1 ? 'start' : 'start';
        const tx = svg('text', { x: wx + (side === 'end' ? -8 : 8), y: wy - 4, 'text-anchor': side, class: 'fig-weight', 'data-copy': `btlab.lesson.label.${spec.weights[i]}` });
        tx.textContent = t(`btlab.lesson.label.${spec.weights[i]}`);
        root.append(tx);
      }
      draw1(k);
    });
    const g = svg('g', { class: `fig-node fig-${n.kids ? n.kind : 'leaf'}` });
    if (!n.kids) {
      g.append(svg('rect', { x: x - LEAF_W / 2, y: y - LEAF_H / 2, width: LEAF_W, height: LEAF_H, rx: 3 }));
      const fo = svg('foreignObject', { x: x - LEAF_W / 2 + 4, y: y - LEAF_H / 2 + 2, width: LEAF_W - 8, height: LEAF_H - 4 });
      fo.append(el('div', { class: 'fig-text', 'data-copy': `btlab.lesson.label.${n.label}` }, t(`btlab.lesson.label.${n.label}`)));
      g.append(fo);
    } else if (SYMBOL[n.kind]) {
      g.append(svg('ellipse', { cx: x, cy: y, rx: 30, ry: 18 }));
      const tx = svg('text', { x, y: y + 6, 'text-anchor': 'middle', class: 'fig-symbol', 'data-fill': '' });
      tx.textContent = SYMBOL[n.kind];
      g.append(tx);
    } else {
      g.append(svg('path', { d: `M${x},${y - 26} L${x + 46},${y} L${x},${y + 26} L${x - 46},${y} Z` }));
      const tx = svg('text', { x, y: y + 5, 'text-anchor': 'middle', class: 'fig-decor', 'data-copy': `btlab.lesson.label.${n.kind}` });
      tx.textContent = t(`btlab.lesson.label.${n.kind}`);
      g.append(tx);
    }
    root.append(g);
  };
  draw1(spec);
  return root;
}

// One page's fight. A single one plays at a time, the page most in view;
// the others are freed so a long lesson costs one fight's work.
let demo = null;
const DESCRIBED = new Map();
function describedTree(key, tree) {
  if (!DESCRIBED.has(key)) {
    const fillText = (n) => { n.text = t(n.label.key, n.label.vars); n.children.forEach(fillText); return n; };
    DESCRIBED.set(key, fillText(JSON.parse(lab_describe_json(JSON.stringify(tree)))));
  }
  return DESCRIBED.get(key);
}
function demoNames(pg) {
  return [pg.demo.tree ? t('btlab.lesson.page_tree') : name(pg.demo.left), name(pg.demo.right)];
}
function startDemo(pg, seed) {
  stopDemo();
  const box = document.querySelector(`.lesson-page[data-page="${pg.id}"]`);
  const canvas = box.querySelector('canvas');
  const d = { pg, box, seed: seed ?? ((Math.random() * 0xffffffff) >>> 0), acc: 0, prev: null, report: [], over: 0, paused: false, shown: -1 };
  d.draw = canvas.renderer ||= renderer(canvas, PAL, N);
  // The lab is about the trees: the fighters are drawn plain.
  d.draw.costumes(false);
  d.draw.backgrounds(false);
  // A third larger, unless the tree is big enough to cover its fighter.
  const count = (n) => 1 + n.children.reduce((a, k) => a + count(k), 0);
  d.draw.treeScale(count(pageTree(pg)) <= 20 ? 1.3 : 1);
  d.draw.bubbleLayout((list, w, h) => JSON.parse(bubbles_step(JSON.stringify(list), w, h)));
  d.lab = pg.demo.tree
    ? Lab.watch_custom(d.seed, N.default_tuning, JSON.stringify(pg.demo.tree), pg.demo.right, 'flat')
    : Lab.watch(d.seed, N.default_tuning, pg.demo.left, pg.demo.right, 'flat');
  d.cur = JSON.parse(d.lab.frame());
  d.ring = ringedIds(pg);
  d.draw.reset();
  demo = d;
}
function stopDemo() {
  if (demo) { demo.lab.free(); demo = null; }
}
function demoTick(now, dt) {
  const d = demo;
  if (!d) return;
  const tickMs = 1000 / N.ticks_per_second;
  if (!d.paused) {
    d.acc += Math.min(dt, 250) * d.pg.demo.speed;
    let n = 0;
    while (d.acc >= tickMs && n < 8) {
      d.lab.step(d.cur && d.cur.phase !== 'fight' ? N.ready_bit : 0, 0);
      d.prev = d.cur;
      d.cur = JSON.parse(d.lab.frame());
      d.draw.events(d.cur);
      d.report = JSON.parse(d.lab.report());
      d.acc -= tickMs;
      n += 1;
    }
  }
  // A finished match starts over after a moment: from the same start on
  // the page about chance, from a new one elsewhere.
  if (d.cur.phase === 'match_over') {
    d.over ||= now;
    if (now - d.over > 2500) { startDemo(d.pg, d.pg.demo.replay ? d.seed : undefined); return; }
  }
  const treeFor = (r) => (r.id === 'custom' ? describedTree(d.pg.id, d.pg.demo.tree) : treeOf(r.id));
  d.draw.trees(d.report.filter((r) => r.id).map((r) => ({ seat: r.seat, tree: treeFor(r), trace: r, ring: r.seat === 0 ? d.ring : null, caption: (nd) => t('tree.now', { node: nd.text }) })));
  d.draw(d.prev || d.cur, d.cur, d.paused ? 1 : Math.min(1, d.acc / tickMs));
  // What each fighter runs and presses, a few times a second.
  const tick = d.lab.tick();
  if (tick - d.shown >= 3 || tick < d.shown) {
    d.shown = tick;
    const names = demoNames(d.pg);
    d.box.querySelector('.demo-readout').replaceChildren(...[0, 1].map((seat) => {
      const r = d.report.find((x) => x.seat === seat);
      const e = r && r.explain[0];
      const line = e && e.mv
        ? say('btlab.lesson.now', { opponent: names[seat], move: t(`tree.act.${e.mv}`) })
        : say('btlab.lesson.idle', { opponent: names[seat] });
      return el('div', { class: `readout side-${seat}` }, line,
        el('span', { class: 'desc', 'data-copy': 'btlab.lesson.keys' }, t('btlab.lesson.keys')), ' ', caps(r ? r.keys : 0, !!(r && r.explain.length > 1)));
    }));
  }
}

function goToLab(step) {
  setUp(step);
  $('lab-section').scrollIntoView({ behavior: 'smooth' });
}

function renderLesson() {
  const k = (pg, f) => `btlab.lesson.${pg.id}.${f}`;
  const pages = LESSON.map((pg, i) => {
    const lect = [];
    for (let n = 1; COPY.btlab.lesson[pg.id][`lecture_${n}`]; n += 1) lect.push(say(k(pg, `lecture_${n}`)));
    const figs = (FIGURES[pg.id] || []).map((f) => figure(f, k(pg, 'figure')));
    const actions = [];
    if (pg.id === 'yours') {
      actions.push(button('btlab.lesson.yours.fight.label', () => goToLab({ mode: 'play', opp: pg.opp, inspect: 1, speed: 1 }), { opponent_mid: t(`opponents.${pg.opp}.name_mid`) }, { id: 'lesson-fight' }),
        button('btlab.lesson.yours.write.label', () => $('editor').scrollIntoView({ behavior: 'smooth' })));
    } else {
      actions.push(button(demo && demo.paused ? 'btlab.resume.label' : 'btlab.pause.label', (e) => {
        if (!demo || demo.pg !== pg) return;
        demo.paused = !demo.paused;
        e.target.textContent = t(demo.paused ? 'btlab.resume.label' : 'btlab.pause.label');
        e.target.dataset.copy = demo.paused ? 'btlab.resume.label' : 'btlab.pause.label';
      }));
      if (pg.demo.replay) {
        actions.push(button('btlab.lesson.replay.label', () => startDemo(pg, demo && demo.pg === pg ? demo.seed : undefined), {}, { class: 'lesson-replay' }),
          button('btlab.lesson.reseed.label', () => startDemo(pg)));
      }
      actions.push(pg.demo.tree
        ? button('btlab.lesson.copy_tree.label', () => { EDIT = JSON.parse(JSON.stringify(pg.demo.tree)); keep(); renderEditor(); $('editor').scrollIntoView({ behavior: 'smooth' }); })
        : button('btlab.lesson.open_lab.label', () => goToLab(labStep(pg)), {}, { class: 'lesson-open' }));
    }
    const side = pg.id === 'yours'
      ? [say(k(pg, 'demo'), pg.vars)]
      : [el('canvas', { width: 1200, height: 640, 'aria-label': t('game.canvas_name') }), say(k(pg, 'ringed'), {}, { class: 'ringed' }),
        el('div', { class: 'demo-readout', role: 'status' }), say(k(pg, 'demo'), pg.vars)];
    return el('section', { class: 'lesson-page', id: `lesson-${pg.id}`, 'data-page': pg.id },
      el('div', { class: 'lecture' },
        say('btlab.lesson.page', { n: i + 1, count: LESSON.length }, { class: 'desc' }),
        el('h2', { 'data-copy': k(pg, 'title') }, t(k(pg, 'title'))),
        ...lect, ...figs),
      el('div', { class: 'in-game' },
        el('h3', { 'data-copy': 'btlab.lesson.in_game' }, t('btlab.lesson.in_game')),
        ...side,
        el('p', { class: 'question' }, el('strong', { 'data-copy': 'btlab.lesson.discuss' }, t('btlab.lesson.discuss')), ' ', el('span', { 'data-copy': k(pg, 'question') }, t(k(pg, 'question')))),
        el('div', { class: 'actions' }, ...actions)));
  });
  const rail = el('nav', { id: 'lesson-rail', 'aria-label': t('btlab.lesson.contents') },
    ...LESSON.map((pg) => el('a', { href: `#lesson-${pg.id}`, 'data-page': pg.id, 'aria-label': t(`btlab.lesson.${pg.id}.title`), title: t(`btlab.lesson.${pg.id}.title`) })),
    el('a', { href: '#lab-section', class: 'rail-lab', 'aria-label': t('btlab.lesson.to_lab'), title: t('btlab.lesson.to_lab') }),
    el('a', { href: '#editor', class: 'rail-editor', 'aria-label': t('btlab.lesson.to_editor'), title: t('btlab.lesson.to_editor') }));
  $('lesson').replaceChildren(
    el('header', { class: 'lesson-head' },
      el('h2', { 'data-copy': 'btlab.lesson.heading' }, t('btlab.lesson.heading')),
      say('btlab.lesson.intro', {}, { class: 'desc' })),
    ...pages, rail);
  // The page most in view plays; the rail marks it.
  const seen = new Map();
  const io = new IntersectionObserver((entries) => {
    for (const e of entries) seen.set(e.target.dataset.page, e.intersectionRatio);
    const [best, ratio] = [...seen.entries()].sort((a, b) => b[1] - a[1])[0] || [];
    for (const a of rail.querySelectorAll('a[data-page]')) a.classList.toggle('on', a.dataset.page === best && ratio > 0.25);
    const pg = ratio > 0.25 ? LESSON.find((p) => p.id === best) : null;
    if (!pg || !pg.demo) { stopDemo(); return; }
    if (!demo || demo.pg !== pg) startDemo(pg);
  }, { threshold: [0, 0.25, 0.5, 0.75, 1] });
  for (const p of $('lesson').querySelectorAll('.lesson-page')) io.observe(p);
  // The lab below runs only while it is on screen, or a tick at a time.
  new IntersectionObserver((entries) => { for (const e of entries) labVisible = e.isIntersecting; }).observe($('lab-section'));
}
let labVisible = false;

function setUp(step) {
  S.mode = step.mode;
  if (step.left) S.left = step.left;
  if (step.right) S.right = step.right;
  if (step.opp) S.opp = step.opp;
  S.inspect = step.inspect;
  S.speed = step.speed;
  S.paused = !!step.paused;
  renderSetup();
  start();
  // A step about one move runs the match on until that move starts, then
  // pauses there, so the class can step through it a tick at a time.
  if (step.until) {
    const running = () => {
      const r = report.find((x) => x.seat === S.inspect);
      return r && r.explain.some((e) => e.mv === step.until);
    };
    for (let i = 0; i < 60 * 60 && !running(); i += 1) tickOnce();
    S.paused = true;
    renderTransport();
    renderPanels();
  }
}

// --- the editor: a tree of your own (Sam, 2026-10-07) -------------------------------------

// The tree being written, in data/pilots.json's shape: it is checked and run
// by core (content::custom), kept in this browser, and shared as a code.
const STARTER = { name: '', reaction_ticks: 14, rules: [
  { if: ['me_down'], do: 'stand', interrupt: true },
  { if: [{ gap_above: 240 }], do: 'approach' },
  { if: [{ gap_below: 200 }, { chance: 50 }], do: 'overhead' },
  { do: 'guard' },
] };
let EDIT = load();
function load() {
  try { const t0 = localStorage.getItem('vagrancy.btlab.tree'); if (t0) return JSON.parse(t0); } catch { /* storage off */ }
  return JSON.parse(JSON.stringify(STARTER));
}
function keep() {
  try { localStorage.setItem('vagrancy.btlab.tree', JSON.stringify(EDIT)); } catch { /* storage off */ }
}
const condId = (c) => (typeof c === 'string' ? c : Object.keys(c)[0]);
const condVal = (c) => (typeof c === 'string' ? null : Object.values(c)[0]);
const unitOf = (id) => (EDITOR.conditions.find((x) => x.id === id) || {}).unit;
function condLabel(id) {
  const unit = unitOf(id);
  return t(`tree.cond.${id}`, unit === 'cm' ? { cm: '…' } : unit === 'pct' ? { pct: '…' } : {});
}
function moveLabel(m) {
  return m === 'search' ? t('btlab.editor.search_move') : t(`tree.act.${m}`);
}
// A share code: the tree as JSON, in base64 so it survives a chat message.
const toCode = (tree) => btoa(unescape(encodeURIComponent(JSON.stringify(tree))));
const fromCode = (code) => JSON.parse(decodeURIComponent(escape(atob(code.trim()))));

function renderEditor() {
  const box = $('editor');
  const check = JSON.parse(lab_check(JSON.stringify(EDIT)));
  const redo = () => { keep(); renderEditor(); };
  const rules = EDIT.rules.map((r, k) => {
    const conds = (r.if || []).map((c, j) => {
      const id = condId(c);
      const unit = unitOf(id);
      const sel = el('select', { 'aria-label': t('btlab.editor.condition'), on: { change: () => {
        const u = unitOf(sel.value);
        r.if[j] = u ? { [sel.value]: u === 'pct' ? 50 : 200 } : sel.value;
        redo();
      } } }, ...EDITOR.conditions.map((x) => el('option', { value: x.id }, condLabel(x.id))));
      sel.value = id;
      const num = unit ? el('input', { type: 'number', value: condVal(c), 'aria-label': t(`btlab.editor.unit.${unit}`),
        on: { change: (e) => { r.if[j] = { [id]: Number(e.target.value) }; redo(); } } }) : null;
      return el('li', {}, sel, ' ', num, ' ', unit ? el('span', { 'data-copy': `btlab.editor.unit.${unit}` }, t(`btlab.editor.unit.${unit}`)) : null, ' ',
        button('btlab.editor.remove_condition.label', () => { r.if.splice(j, 1); redo(); }));
    });
    const mv = el('select', { 'aria-label': t('btlab.editor.move'), on: { change: () => { r.do = mv.value; redo(); } } },
      ...EDITOR.moves.map((m) => el('option', { value: m }, moveLabel(m))));
    mv.value = r.do;
    const intr = el('label', {}, el('input', { type: 'checkbox', checked: r.interrupt ? true : null,
      on: { change: (e) => { r.interrupt = e.target.checked; redo(); } } }), ' ', t('btlab.editor.interrupt'));
    return el('li', { class: `rule${check.rule === k ? ' refused' : ''}`, 'data-rule': String(k) },
      el('h4', { 'data-copy': 'btlab.editor.rule' }, t('btlab.editor.rule', { n: k + 1 })),
      say('btlab.editor.if', {}, { class: 'desc' }),
      el('ul', { class: 'conds' }, ...conds),
      button('btlab.editor.add_condition.label', () => { (r.if ||= []).push('me_grounded'); redo(); }),
      el('p', {}, el('label', { 'data-copy': 'btlab.editor.then' }, t('btlab.editor.then')), ' ', mv),
      el('p', {}, intr),
      el('div', { class: 'actions' },
        button('btlab.editor.up.label', () => { if (k > 0) { [EDIT.rules[k - 1], EDIT.rules[k]] = [EDIT.rules[k], EDIT.rules[k - 1]]; redo(); } }),
        button('btlab.editor.down.label', () => { if (k < EDIT.rules.length - 1) { [EDIT.rules[k + 1], EDIT.rules[k]] = [EDIT.rules[k], EDIT.rules[k + 1]]; redo(); } }),
        button('btlab.editor.remove_rule.label', () => { EDIT.rules.splice(k, 1); redo(); })));
  });
  const nameIn = el('input', { type: 'text', id: 'editor-name', maxlength: 40, value: EDIT.name || '', on: { change: (e) => { EDIT.name = e.target.value; redo(); } } });
  const react = el('input', { type: 'number', id: 'editor-reaction', min: EDITOR.reaction[0], max: EDITOR.reaction[1], value: EDIT.reaction_ticks,
    on: { change: (e) => { EDIT.reaction_ticks = Number(e.target.value); redo(); } } });
  const status = check.ok ? say('btlab.editor.ok', {}, { class: 'ok', id: 'editor-status' })
    : say(check.key, { n: (check.rule ?? 0) + 1 }, { class: 'refused', id: 'editor-status' });
  const codeOut = el('textarea', { id: 'editor-code', rows: 3, readonly: true, 'aria-label': t('btlab.editor.code') });
  const codeIn = el('textarea', { id: 'editor-code-in', rows: 3, 'aria-label': t('btlab.editor.paste') });
  box.replaceChildren(
    el('h2', { 'data-copy': 'btlab.editor.heading' }, t('btlab.editor.heading')),
    say('btlab.editor.intro', { max_rules: EDITOR.max_rules, max_conds: EDITOR.max_conds }, { class: 'desc' }),
    el('p', {}, el('label', { for: 'editor-name', 'data-copy': 'btlab.editor.name' }, t('btlab.editor.name')), ' ', nameIn),
    el('p', {}, el('label', { for: 'editor-reaction', 'data-copy': 'btlab.editor.reaction' }, t('btlab.editor.reaction', { lo: EDITOR.reaction[0], hi: EDITOR.reaction[1] })), ' ', react,
      ' ', el('span', { 'data-copy': 'btlab.editor.reaction_ms' }, t('btlab.editor.reaction_ms', { ms: Math.round(EDIT.reaction_ticks * 1000 / N.ticks_per_second) }))),
    el('ol', { class: 'rules' }, ...rules),
    el('div', { class: 'actions' },
      button('btlab.editor.add_rule.label', () => { EDIT.rules.push({ if: [], do: 'guard' }); redo(); }, {}, { id: 'editor-add-rule' }),
      button('btlab.editor.reset.label', () => { EDIT = JSON.parse(JSON.stringify(STARTER)); redo(); })),
    status,
    el('div', { class: 'actions' },
      button('btlab.editor.watch.label', () => { if (check.ok) { renderSetup(); start('watch'); $('lab-section').scrollIntoView({ behavior: 'smooth' }); } }, { opponent: name(S.right) }, { id: 'editor-watch', disabled: !check.ok }),
      button('btlab.editor.fight.label', () => { if (check.ok) { start('play'); $('lab-section').scrollIntoView({ behavior: 'smooth' }); } }, {}, { id: 'editor-fight', disabled: !check.ok })),
    say('btlab.editor.against', {}, { class: 'desc' }),
    el('h3', { 'data-copy': 'btlab.editor.share' }, t('btlab.editor.share')),
    el('div', { class: 'actions' }, button('btlab.editor.make_code.label', () => {
      codeOut.value = toCode(EDIT);
      codeOut.select();
      // A browser may refuse the clipboard; the code is in the box anyway.
      if (navigator.clipboard) navigator.clipboard.writeText(codeOut.value).catch(() => {});
    }, {}, { id: 'editor-make-code' })),
    codeOut,
    codeIn,
    el('div', { class: 'actions' }, button('btlab.editor.load_code.label', () => {
      try { EDIT = fromCode(codeIn.value); redo(); } catch { $('editor-status').replaceWith(say('btlab.editor.refuse.code', {}, { class: 'refused', id: 'editor-status' })); }
    }, {}, { id: 'editor-load-code' })));
}

// --- start ---------------------------------------------------------------------------------

async function main() {
  try {
    WASM = await init();
    COPY = JSON.parse(copy_json());
    N = JSON.parse(coreNumbers());
    PAL = JSON.parse(palette_json());
    CONTROLS = JSON.parse(controls_json());
    ROSTER = JSON.parse(lab_roster_json());
    MOVES = JSON.parse(lab_moves_json());
    EDITOR = JSON.parse(lab_conditions_json());
    ACTION_BITS = Object.fromEntries(N.actions);
  } catch (e) {
    console.error(e);
    $('loading-error').hidden = false;
    return;
  }
  listen((code) => S.mode === 'play' && Object.values(CONTROLS.solo).includes(code));
  draw = renderer($('stage'), PAL, N);
  draw.costumes(false);
  draw.backgrounds(false);
  draw.bubbleLayout((list, w, h) => JSON.parse(bubbles_step(JSON.stringify(list), w, h)));
  // Space pauses and resumes; the full stop steps a tick while paused.
  window.addEventListener('keydown', (e) => {
    if (e.target && ['INPUT', 'SELECT', 'TEXTAREA'].includes(e.target.tagName)) return;
    if (e.code === 'Space') { e.preventDefault(); S.paused = !S.paused; renderTransport(); }
    if (e.code === 'Period' && S.paused) { e.preventDefault(); stepOnce = true; }
  });
  $('status').hidden = true;
  $('lab').hidden = false;
  renderEditor();
  renderSetup();
  setUp(labStep(LESSON[0]));
  renderLesson();
  document.body.dataset.ready = '1';
  // For the gate: what the lab shows and what core pressed.
  window.btlab = { report: () => report, keys: () => (lab ? Array.from(lab.keys()) : []), tick: () => (lab ? lab.tick() : 0), state: () => ({ ...S }),
    lesson: () => (demo ? { page: demo.pg.id, tick: demo.lab.tick(), seed: demo.seed, speed: demo.pg.demo.speed, rings: demo.draw.shown().rings } : null),
    // How large core's memory has grown, in bytes: a leak shows here first.
    memory: () => WASM.memory.buffer.byteLength,
    // Each page's ringed nodes, counted in its left fighter's tree.
    lessonRings: () => Object.fromEntries(LESSON.filter((pg) => pg.demo).map((pg) => [pg.id, ringedIds(pg).size])) };
  requestAnimationFrame(loop);
}

main();
