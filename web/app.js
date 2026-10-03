// The page. It draws numbers core sent and decides nothing: no physics, no
// contact, no copy of a constant (CLAUDE.md). Every word it shows comes from
// data/copy.en.json, which reaches it through the wasm module.
import init, {
  copy_json, palette_json, controls_json, numbers as coreNumbers, script_checksum, Game,
} from './pkg/vagrancy_wasm.js';
import { renderer } from './draw.js';
import { listen, bits, keyName } from './keys.js';
import { download, pick } from './files.js';

const BUILD = '__BUILD__';
const $ = (id) => document.getElementById(id);

let COPY = null;
let N = null; // numbers from core
let PALETTE = null;
let CONTROLS = null;
let ACTION_BITS = {};

// Look up a dotted key and fill its placeholders. `{game}` and `{hash}` are
// always available; everything else is passed in by the caller from core.
export function t(key, vars = {}) {
  const s = key.split('.').reduce((o, k) => (o == null ? o : o[k]), COPY);
  if (typeof s !== 'string') throw new Error(`no copy string at ${key}`);
  const all = { game: COPY.game.name, hash: BUILD, ...vars };
  return s.replace(/\{([a-z_.]+)\}/g, (m, p) => {
    if (!(p in all)) throw new Error(`no value for {${p}} in ${key}`);
    return String(all[p]);
  });
}

// --- building screens from copy keys -----------------------------------------

function el(tag, attrs = {}, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === 'on') for (const [ev, fn] of Object.entries(v)) e.addEventListener(ev, fn);
    else if (v !== undefined && v !== null) e.setAttribute(k, v);
  }
  for (const k of kids) if (k != null) e.append(k);
  return e;
}
// A paragraph holding one copy string; `data-copy` names it for the gate.
const say = (key, vars, attrs = {}) => el('p', { 'data-copy': key, ...attrs }, t(key, vars));
const button = (key, onclick, vars) => el('button', { type: 'button', 'data-copy': key, on: { click: onclick } }, t(key, vars));

function show(...nodes) {
  const s = $('screen');
  s.replaceChildren(...nodes);
  s.hidden = false;
}

// Placeholders for the solo keys, by action: {key.shoulder_up} and so on.
function keyVars(binding) {
  const v = {};
  for (const [action, code] of Object.entries(binding)) v[`key.${action}`] = keyName(code);
  return v;
}

// --- the loop ---------------------------------------------------------------------

let game = null;
let seats = () => [0, 0];
let prevFrame = null;
let curFrame = null;
let acc = 0;
let last = 0;
let draw = null;
let onTick = null;

function start(g, seatFn, tickFn = null) {
  game = g;
  seats = seatFn;
  onTick = tickFn;
  curFrame = JSON.parse(game.frame());
  prevFrame = null;
  acc = 0;
  $('stage').hidden = false;
}

function stop() {
  game = null;
  $('stage').hidden = true;
}

function loop(now) {
  const tickMs = 1000 / N.ticks_per_second;
  if (game) {
    acc += Math.min(now - (last || now), 250);
    // Catch up at most eight ticks a frame, so a stalled tab does not
    // replay a burst of stale keys (Floodline caps its catch-up the same way).
    let n = 0;
    while (acc >= tickMs && n < 8) {
      const [a, b] = seats();
      game.step(a, b);
      prevFrame = curFrame;
      curFrame = JSON.parse(game.frame());
      acc -= tickMs;
      n += 1;
      if (onTick) onTick();
      if (!game) break;
    }
    if (game) draw(prevFrame, curFrame, Math.min(1, acc / tickMs));
  }
  last = now;
  requestAnimationFrame(loop);
}

// --- screens ------------------------------------------------------------------------

function menu() {
  stop();
  const item = (key, action) => el('div', { class: 'item' }, button(`${key}.label`, action), say(`${key}.desc`, {}, { class: 'desc' }));
  show(
    item('menu.practice', practice),
    item('menu.replay', loadReplay),
  );
}

function practice() {
  const binding = CONTROLS.solo;
  const vars = keyVars(binding);
  show(
    say('practice.intro'),
    el('ol', { id: 'steps' },
      el('li', {}, say('practice.step.shoulder', vars)),
      el('li', {}, say('practice.step.elbow', vars))),
    el('div', { class: 'actions' },
      button('replay.download.label', () => download(game.replay_bytes(), 'vagrancy.replay')),
      button('replay.load.label', loadReplay),
      button('menu.back.label', menu)),
  );
  start(Game.alone(seed(), N.default_tuning), () => [bits(binding, ACTION_BITS), 0]);
}

async function loadReplay() {
  const file = await pick('.replay');
  if (!file) return;
  let g;
  try {
    g = Game.load_replay(file.bytes);
  } catch (err) {
    const { key, vars } = JSON.parse(err);
    show(say(key, vars, { role: 'alert' }), button('menu.back.label', menu));
    stop();
    return;
  }
  show(say('replay.playing'), button('replay.stop.label', menu));
  start(g, () => [0, 0], () => {
    if (game && game.done()) document.body.dataset.replayDone = '1';
  });
  delete document.body.dataset.replayDone;
}

function seed() {
  // A fresh match gets a fresh spawn jitter. The seed travels in the replay,
  // so playback does not depend on this.
  return (Date.now() & 0x7fffffff) >>> 0;
}

async function main() {
  try {
    await init();
  } catch (e) {
    $('status').hidden = true;
    $('loading-error').hidden = false;
    console.warn(e);
    return;
  }
  COPY = JSON.parse(copy_json());
  N = JSON.parse(coreNumbers());
  PALETTE = JSON.parse(palette_json());
  CONTROLS = JSON.parse(controls_json());
  ACTION_BITS = Object.fromEntries(N.actions);
  const bound = new Set(Object.values(CONTROLS.solo));
  listen((code) => game && bound.has(code));
  draw = renderer($('stage'), PALETTE, N);
  $('build').textContent = t('game.build');
  $('status').hidden = true;
  // Hooks for testing/drive.py. They read core; they change nothing.
  window.vagrancy = {
    scriptChecksum: (n) => script_checksum(n),
    checksum: () => game && game.checksum(),
    tick: () => game && game.tick(),
    recordedChecksum: () => game && game.recorded_checksum(),
  };
  menu();
  requestAnimationFrame(loop);
  document.body.dataset.ready = '1';
}

main();
