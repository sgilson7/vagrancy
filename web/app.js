// The page. It draws numbers core sent and decides nothing: no physics, no
// contact, no copy of a constant (CLAUDE.md). Every word it shows comes from
// data/copy.en.json, which reaches it through the wasm module; which sentence
// to show for an outcome is chosen by core (content::messages).
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
let BINDINGS = null; // { solo, left, right }: action -> KeyboardEvent.code

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
// A value filled from data, not a sentence: a key's name. The gate's copy
// check skips `data-fill` text, as it skips the values inside a placeholder.
const fillText = (text) => el('kbd', { 'data-fill': '' }, text);
// A sentence chosen by core: { key, vars }.
const sayChosen = (msg, attrs = {}) => say(msg.key, msg.vars, attrs);

function show(...nodes) {
  const s = $('screen');
  s.replaceChildren(...nodes);
  s.hidden = false;
}

// Placeholders for one seat's keys, by action: {key.shoulder_up} and so on.
function keyVars(binding) {
  const v = {};
  for (const [action, code] of Object.entries(binding)) v[`key.${action}`] = keyName(code);
  return v;
}
// "the bound keys for that seat, joined in binding order" (_placeholders).
const keyList = (binding) => N.actions.map(([a]) => keyName(binding[a])).join(', ');

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
  draw.reset();
  $('stage').hidden = false;
  music.fight(!game.is_replay());
}

function stop() {
  game = null;
  $('stage').hidden = true;
  $('hud').hidden = true;
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
      draw.events(curFrame);
      acc -= tickMs;
      n += 1;
      if (onTick) onTick(curFrame);
      if (!game) break;
    }
    if (game) draw(prevFrame, curFrame, Math.min(1, acc / tickMs));
  }
  last = now;
  requestAnimationFrame(loop);
}

// --- the match: HUD, rounds, results -------------------------------------------

// Watches the frames of a match and shows what core says about its phase.
// `opponent` is a road opponent's id, or '' for versus. `onEnd` builds the
// buttons for the end of the match.
function matchWatcher(opponent, endButtons) {
  let phase = 'fight';
  const hud = $('hud');
  hud.hidden = false;
  const panel = el('div', { id: 'result', role: 'status' });
  return {
    panel,
    tick(frame) {
      hud.replaceChildren(
        el('span', { 'data-copy': 'hud.round' }, t('hud.round', { round: frame.round })),
        el('span', { 'data-copy': 'hud.score' }, t('hud.score', {
          left_name: t('fighters.left.name'), left_wins: frame.wins[0],
          right_name: t('fighters.right.name'), right_wins: frame.wins[1],
        })),
      );
      if (frame.phase === phase) return;
      phase = frame.phase;
      if (phase === 'fight') {
        panel.replaceChildren();
        return;
      }
      const said = JSON.parse(game.phase_text(opponent));
      const kids = [sayChosen(said.round, { class: 'result' })];
      if (said.match) {
        kids.push(sayChosen(said.match, { class: 'result' }));
        kids.push(...endButtons());
      } else if (!game.is_replay()) {
        kids.push(button('results.next_round.label', () => { READY = true; }));
      }
      panel.replaceChildren(...kids);
      document.body.dataset.phase = phase;
    },
  };
}

let READY = false;
// Adds "ready" to both seats until the next round has begun.
function withReady(fn) {
  return () => {
    const [a, b] = fn();
    if (READY && curFrame && curFrame.phase !== 'round_over') READY = false;
    return READY ? [a | N.ready_bit, b | N.ready_bit] : [a, b];
  };
}

// --- screens ------------------------------------------------------------------------

function menu() {
  stop();
  delete document.body.dataset.phase;
  const item = (key, action) => el('div', { class: 'item' }, button(`${key}.label`, action), say(`${key}.desc`, {}, { class: 'desc' }));
  show(
    item('menu.local', local),
    item('menu.practice', practice),
    item('menu.replay', loadReplay),
    el('div', { class: 'item' }, button('menu.settings.label', settings)),
  );
}

function local() {
  stop();
  show(
    say('local.intro', {
      left_name: t('fighters.left.name'), left_keys: keyList(BINDINGS.left),
      right_name: t('fighters.right.name'), right_keys: keyList(BINDINGS.right),
    }),
    say('local.keyboard_limit', {}, { class: 'desc' }),
    el('div', { class: 'actions' }, button('local.start.label', startLocal), button('menu.back.label', menu)),
  );
}

function startLocal() {
  READY = false;
  const watch = matchWatcher('', () => [el('div', { class: 'actions' },
    button('results.rematch.label', startLocal),
    button('results.replay.label', () => download(game.replay_bytes(), 'vagrancy.replay')),
    button('menu.back.label', menu))]);
  show(watch.panel, el('div', { class: 'actions' }, button('menu.back.label', menu)));
  start(Game.versus(seed(), tuning()),
    withReady(() => [bits(BINDINGS.left, ACTION_BITS), bits(BINDINGS.right, ACTION_BITS)]),
    (f) => watch.tick(f));
}

function practice() {
  const binding = BINDINGS.solo;
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
  delete document.body.dataset.replayDone;
  const watch = matchWatcher('', () => []);
  show(say('replay.playing'), watch.panel, button('replay.stop.label', menu));
  start(g, () => [0, 0], (f) => {
    watch.tick(f);
    if (game && game.done()) document.body.dataset.replayDone = '1';
  });
}

function seed() {
  // A fresh match gets a fresh spawn jitter. The seed travels in the replay,
  // so playback does not depend on this.
  return (Date.now() & 0x7fffffff) >>> 0;
}

// --- settings ------------------------------------------------------------------------

function settings() {
  stop();
  show(musicSection(), keysSection(), button('menu.back.label', menu));
}

let REMEMBER = false;
let MUSIC_ERROR = null;

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

// Key bindings. The convenience copy lives in this browser's storage; M5's
// save file carries them too.
const KEYS_STORE = 'vagrancy.keys';

function loadBindings() {
  const defaults = { solo: { ...CONTROLS.solo }, left: { ...CONTROLS.left }, right: { ...CONTROLS.right } };
  try {
    const kept = JSON.parse(localStorage.getItem(KEYS_STORE) || 'null');
    if (kept && kept.solo && kept.left && kept.right) return kept;
  } catch (e) { /* storage off: the defaults stand */ }
  return defaults;
}

function saveBindings() {
  try { localStorage.setItem(KEYS_STORE, JSON.stringify(BINDINGS)); } catch (e) { /* storage off */ }
}

let KEY_CONFLICT = null;

function keysSection() {
  // One player's keys, and each seat's at one keyboard. A key may serve
  // only one action within a group (settings.keys.desc).
  const group = (name, heading) => {
    const rows = N.actions.map(([action]) => {
      const b = el('button', { type: 'button', class: 'bind', 'data-group': name, 'data-action': action }, fillText(keyName(BINDINGS[name][action])));
      b.addEventListener('click', () => capture(name, action, b));
      return el('p', {}, el('span', { 'data-copy': `settings.keys.actions.${action}` }, t(`settings.keys.actions.${action}`)), ' ', b);
    });
    return el('div', { class: 'keys' }, el('h3', { 'data-copy': heading }, t(heading)), ...rows);
  };
  return el('section', { id: 'keys' },
    el('h2', { 'data-copy': 'settings.keys.title' }, t('settings.keys.title')),
    say('settings.keys.desc'),
    KEY_CONFLICT ? say('settings.keys.conflict', KEY_CONFLICT, { role: 'alert' }) : null,
    group('solo', 'settings.keys.solo.heading'),
    group('left', 'fighters.left.name'),
    group('right', 'fighters.right.name'),
    button('settings.keys.reset.label', () => {
      BINDINGS = { solo: { ...CONTROLS.solo }, left: { ...CONTROLS.left }, right: { ...CONTROLS.right } };
      KEY_CONFLICT = null;
      saveBindings();
      settings();
    }),
  );
}

function capture(groupName, action, btn) {
  btn.classList.add('listening');
  const onKey = (e) => {
    e.preventDefault();
    window.removeEventListener('keydown', onKey, true);
    const taken = Object.entries(BINDINGS[groupName]).find(([a, code]) => code === e.code && a !== action);
    if (taken) {
      KEY_CONFLICT = { key: keyName(e.code), action: t(`settings.keys.actions.${taken[0]}`) };
    } else {
      BINDINGS[groupName][action] = e.code;
      KEY_CONFLICT = null;
      saveBindings();
    }
    settings();
  };
  window.addEventListener('keydown', onKey, true);
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
  BINDINGS = loadBindings();
  listen((code) => game && Object.values(BINDINGS).some((b) => Object.values(b).includes(code)));
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
    phase: () => curFrame && curFrame.phase,
  };
  menu();
  requestAnimationFrame(loop);
  document.body.dataset.ready = '1';
}

main();
