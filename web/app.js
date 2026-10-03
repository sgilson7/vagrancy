// The page. It draws numbers core sent and decides nothing: no physics, no
// contact, no copy of a constant (CLAUDE.md). Every word it shows comes from
// data/copy.en.json, which reaches it through the wasm module.
import init, {
  copy_json, palette_json, controls_json, numbers as coreNumbers, script_checksum, Game,
} from './pkg/vagrancy_wasm.js';
import { renderer } from './draw.js';
import { listen, bits, keyName } from './keys.js';
import { download, pick } from './files.js';
import * as music from './music.js';

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
  music.fight(!game.is_replay());
}

function stop() {
  game = null;
  $('stage').hidden = true;
  music.fight(false);
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
    el('div', { class: 'item' }, button('menu.settings.label', settings)),
  );
}

function practice() {
  const binding = CONTROLS.solo;
  const vars = keyVars(binding);
  const steps = ['shoulder', 'elbow', 'cut', 'plant', 'swing'];
  show(
    say('practice.intro'),
    el('ol', { id: 'steps' }, ...steps.map((k) => el('li', {}, say(`practice.step.${k}`, vars)))),
    say('practice.done'),
    el('div', { class: 'actions' },
      button('practice.reset.label', practice),
      button('replay.download.label', () => download(game.replay_bytes(), 'vagrancy.replay')),
      button('replay.load.label', loadReplay),
      button('menu.back.label', menu)),
  );
  start(Game.practice(seed(), tuning()), () => [bits(binding, ACTION_BITS), 0]);
}

// `?tuning=0|1|2` picks one of the candidate tunings in sim::balance, so Sam
// can play them side by side (M2.0). Any other value takes the default.
function tuning() {
  const v = Number(new URLSearchParams(location.search).get('tuning'));
  return Number.isInteger(v) && v >= 0 && v < N.tunings ? v : N.default_tuning;
}

// --- settings ------------------------------------------------------------------------

function settings() {
  stop();
  show(musicSection(), button('menu.back.label', menu));
}

function musicSection() {
  const state = music.current();
  const status = state.name
    ? say('settings.music.loaded', { file_name: state.name }, { id: 'music-status' })
    : say('settings.music.none', {}, { id: 'music-status' });
  const remember = el('input', { type: 'checkbox', id: 'music-remember' });
  remember.checked = REMEMBER;
  remember.addEventListener('change', async () => {
    REMEMBER = remember.checked;
    await music.setRemember(REMEMBER);
  });
  const vol = el('input', { type: 'range', id: 'music-volume', min: '0', max: '100', value: String(Math.round(state.volume * 100)) });
  vol.addEventListener('input', () => music.setVolume(Number(vol.value) / 100));
  return el('section', { id: 'music' },
    el('h2', { 'data-copy': 'settings.music.title' }, t('settings.music.title')),
    say('settings.music.desc'),
    el('div', { class: 'actions' },
      button('settings.music.load.label', async () => {
        const file = await pick('audio/*');
        if (file) music.load(file, REMEMBER);
      }),
      state.name ? button('settings.music.remove.label', () => music.remove()) : null),
    status,
    MUSIC_ERROR ? say('settings.music.error', { error: MUSIC_ERROR }, { role: 'alert' }) : null,
    say('settings.music.privacy'),
    el('p', {}, remember, ' ', el('label', { for: 'music-remember', 'data-copy': 'settings.music.remember.label' }, t('settings.music.remember.label'))),
    say('settings.music.remember.desc', {}, { class: 'desc' }),
    el('p', {}, el('label', { for: 'music-volume', 'data-copy': 'settings.music.volume.label' }, t('settings.music.volume.label')), ' ', vol),
    say('settings.music.formats', {}, { class: 'desc' }),
  );
}

let REMEMBER = false;
let MUSIC_ERROR = null;

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
  music.subscribe(({ error }) => {
    MUSIC_ERROR = error || null;
    if (document.getElementById('music')) settings();
  });
  const kept = await music.remembered();
  if (kept) {
    REMEMBER = true;
    music.load(kept, false);
  }
  $('build').textContent = t('game.build');
  $('status').hidden = true;
  // Hooks for testing/drive.py. They read core; they change nothing.
  window.vagrancy = {
    scriptChecksum: (n) => script_checksum(n),
    checksum: () => game && game.checksum(),
    tick: () => game && game.tick(),
    recordedChecksum: () => game && game.recorded_checksum(),
    music: () => music.current(),
  };
  menu();
  requestAnimationFrame(loop);
  document.body.dataset.ready = '1';
}

main();
