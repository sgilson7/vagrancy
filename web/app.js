// The page. It draws numbers core sent and decides nothing: no physics, no
// contact, no copy of a constant (CLAUDE.md). Every word it shows comes from
// data/copy.en.json, which reaches it through the wasm module; which sentence
// to show for an outcome is chosen by core (content::messages).
import init, {
  copy_json, palette_json, controls_json, numbers as coreNumbers, script_checksum, Game, Online,
} from './pkg/vagrancy_wasm.js';
import * as rtc from './rtc.js';
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
  hangUpOnline();
  delete document.body.dataset.phase;
  const item = (key, action) => el('div', { class: 'item' }, button(`${key}.label`, action), say(`${key}.desc`, {}, { class: 'desc' }));
  show(
    item('menu.local', local),
    item('menu.online', online),
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

// --- online (D14) -------------------------------------------------------------------

let NET = null; // { sess, link, peer, timer }

function hangUpOnline() {
  if (!NET) return;
  clearInterval(NET.timer);
  NET.link.close();
  NET = null;
}

function online() {
  stop();
  hangUpOnline();
  const field = el('input', { id: 'room-code', type: 'text', autocomplete: 'off', spellcheck: 'false' });
  show(
    say('online.intro'),
    say('online.privacy', {}, { class: 'desc' }),
    say('online.music', {}, { class: 'desc' }),
    el('div', { class: 'item' }, button('online.host_room.label', () => hostRoom()), say('online.host_room.desc', {}, { class: 'desc' })),
    el('div', { class: 'item' },
      button('online.join_room.label', () => field.value.trim() && joinRoom(field.value.trim().toUpperCase())),
      say('online.join_room.desc', {}, { class: 'desc' }),
      el('p', {}, el('label', { for: 'room-code', 'data-copy': 'online.join_room.field' }, t('online.join_room.field')), ' ', field)),
    el('div', { class: 'item' }, button('online.host_paste.label', hostPaste), say('online.host_paste.desc', {}, { class: 'desc' })),
    el('div', { class: 'item' }, button('online.join_paste.label', joinPaste)),
    button('menu.back.label', menu),
  );
}

function roomCode() {
  const alphabet = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';
  const r = crypto.getRandomValues(new Uint8Array(6));
  return Array.from(r, (b) => alphabet[b % alphabet.length]).join('');
}

// Open the transport and the session together. `paint` redraws the lobby.
function openNet(isHost, mode, code, paint) {
  hangUpOnline();
  const sess = isHost ? Online.host(seed(), tuning(), BUILD) : Online.join(BUILD);
  const net = { sess, peer: null, error: null, link: null, timer: null, started: false };
  NET = net;
  const flush = () => {
    if (net.peer === null) return;
    const out = net.sess.outbox();
    const view = new DataView(out.buffer, out.byteOffset, out.byteLength);
    for (let i = 0; i < out.length;) {
      const n = view.getUint32(i, true);
      net.link.send(net.peer, out.slice(i + 4, i + 4 + n));
      i += 4 + n;
    }
  };
  net.flush = flush;
  net.link = rtc.open({
    isHost, mode, room: code, build: BUILD,
    onPeer: (id) => {
      if (net.peer === null) { net.peer = id; flush(); paint(); } else net.link.send(id, Online.refusal_full());
    },
    onLeft: (id) => { if (id === net.peer) { net.left = true; paint(); } },
    onMessage: (id, bytes) => {
      if (id !== net.peer) return;
      sess.receive(performance.now(), bytes);
      flush();
      if (!net.started && JSON.parse(sess.status()).kind === 'playing') beginOnline(net);
      paint();
    },
    onError: (key) => { net.error = key; paint(); },
  });
  net.timer = setInterval(() => { sess.poll(performance.now()); flush(); if (!net.started) paint(); }, 100);
  return net;
}

function copyButton(text) {
  const done = el('span', { 'aria-live': 'polite' });
  const b = button('online.copy.label', async () => {
    try {
      await navigator.clipboard.writeText(text());
      done.replaceChildren(say('online.copy.done', {}, { class: 'desc' }));
    } catch (e) {
      done.replaceChildren(say('online.copy.failed', {}, { class: 'desc', role: 'alert' }));
    }
  });
  return el('div', {}, b, done);
}

// The lobby's lines for a session's state, or its refusal.
function lobbyLines(net, waitingKey, vars) {
  if (net.error) return [say(net.error, {}, { role: 'alert' })];
  const st = JSON.parse(net.sess.status());
  if (st.kind === 'full') return [say('online.error.full', {}, { role: 'alert' })];
  if (st.kind === 'build') return [say('online.error.build', {}, { role: 'alert' })];
  if (net.left || st.kind === 'left') return [say('online.status.left', {}, { role: 'alert' })];
  if (st.kind === 'connected') {
    if (net.sess.seat() === 0) {
      return [say('online.connected.host', { delay_ms: st.delay_ms }),
        button('online.start.label', () => { net.sess.start(performance.now()); net.flush(); beginOnline(net); })];
    }
    return [say('online.connected.join', { delay_ms: st.delay_ms })];
  }
  return waitingKey ? [say(waitingKey, vars)] : [];
}

function hostRoom() {
  const code = roomCode();
  const link = `${location.origin}${location.pathname}#room=${code}`;
  const box = el('div', { id: 'lobby' });
  const paint = () => box.replaceChildren(...lobbyLines(net, 'online.waiting', { code }));
  show(box, el('p', {}, fillText(link)), copyButton(() => link), button('menu.back.label', online));
  const net = openNet(true, 'room', code, paint);
  paint();
}

function joinRoom(code) {
  const box = el('div', { id: 'lobby' });
  const paint = () => box.replaceChildren(...lobbyLines(net, 'online.joining', { code }));
  show(box, button('menu.back.label', online));
  const net = openNet(false, 'room', code, paint);
  paint();
}

function hostPaste() {
  const invite = el('textarea', { id: 'invite', readonly: '', rows: '4', cols: '60' });
  const reply = el('textarea', { id: 'reply', rows: '4', cols: '60' });
  const box = el('div', { id: 'lobby' });
  const paint = () => box.replaceChildren(...lobbyLines(net, null, {}));
  show(
    say('online.host_paste.step_copy'), invite, copyButton(() => invite.value),
    say('online.host_paste.step_reply'), reply, box, button('menu.back.label', online),
  );
  const net = openNet(true, 'code', '', paint);
  net.link.invitation().then((code) => { invite.value = code; document.body.dataset.invite = '1'; });
  reply.addEventListener('input', () => {
    net.link.accept(reply.value).catch(() => { net.error = 'online.error.bad_code'; paint(); });
  });
  paint();
}

function joinPaste() {
  const invite = el('textarea', { id: 'invite', rows: '4', cols: '60' });
  const out = el('div', { id: 'reply-box' });
  const box = el('div', { id: 'lobby' });
  const paint = () => box.replaceChildren(...lobbyLines(net, null, {}));
  show(say('online.join_paste.step_paste'), invite, out, box, button('menu.back.label', online));
  const net = openNet(false, 'code', '', paint);
  invite.addEventListener('input', () => {
    net.link.reply(invite.value).then((code) => {
      const reply = el('textarea', { id: 'reply', readonly: '', rows: '4', cols: '60' });
      reply.value = code;
      out.replaceChildren(say('online.join_paste.step_reply'), reply, copyButton(() => code));
      document.body.dataset.reply = '1';
    }).catch(() => { net.error = 'online.error.bad_code'; paint(); });
  });
  paint();
}

// The match itself: the shared loop drives an adapter over the session, so
// the page draws online play exactly as it draws a local match.
function beginOnline(net) {
  if (net.started) return;
  net.started = true;
  READY = false;
  const sess = net.sess;
  const adapter = {
    step: (a) => { sess.step(performance.now(), a); net.flush(); },
    frame: () => sess.frame(),
    phase_text: () => sess.phase_text(),
    checksum: () => sess.checksum(),
    tick: () => sess.tick(),
    is_replay: () => false,
    done: () => false,
    replay_bytes: () => sess.replay_bytes(),
    recorded_checksum: () => '',
  };
  const status = el('div', { id: 'net-status', role: 'status' });
  const watch = matchWatcher('', () => [el('div', { class: 'actions' },
    button('results.replay.label', () => download(sess.replay_bytes(), 'vagrancy.replay')),
    button('menu.back.label', online))]);
  show(watch.panel, status, el('div', { class: 'actions' }, button('menu.back.label', online)));
  const delay = JSON.parse(sess.status()).delay_ms;
  let shown = '';
  start(adapter, withReady(() => [bits(BINDINGS.solo, ACTION_BITS), 0]), (f) => {
    watch.tick(f);
    $('hud').append(el('span', { 'data-copy': 'hud.delay' }, t('hud.delay', { delay_ms: delay })));
    const st = JSON.parse(sess.status());
    if (st.kind === shown) return;
    shown = st.kind;
    if (st.kind === 'waiting_on') status.replaceChildren(say('online.status.waiting_on'));
    else if (st.kind === 'left' || net.left) status.replaceChildren(say('online.status.left', {}, { role: 'alert' }));
    else if (st.kind === 'desync') status.replaceChildren(say('online.status.desync', { tick: st.tick }, { role: 'alert' }),
      button('results.replay.label', () => download(sess.replay_bytes(), 'vagrancy.replay')));
    else status.replaceChildren();
  });
  document.body.dataset.online = 'playing';
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
    online: () => NET && { status: JSON.parse(NET.sess.status()), tick: NET.sess.tick(), checksum: NET.sess.checksum() },
  };
  const m = location.hash.match(/^#room=([A-Z0-9]{4,12})$/);
  if (m) joinRoom(m[1]); else menu();
  requestAnimationFrame(loop);
  document.body.dataset.ready = '1';
}

main();
