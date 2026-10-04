// The page. It draws numbers core sent and decides nothing: no physics, no
// contact, no copy of a constant (CLAUDE.md). Every word it shows comes from
// data/copy.en.json, which reaches it through the wasm module; which sentence
// to show for an outcome is chosen by core (content::messages).
import init, {
  copy_json, palette_json, controls_json, numbers as coreNumbers, script_checksum, Game, Online, Road,
  road_json, save_fresh, save_read,
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
let SAVE = null; // the save file's parsed contents: { format, version, state }

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
// One line that names every key, shown in every fight that one person plays.
const keysLine = (binding) => say('hud.keys', keyVars(binding), { class: 'desc', id: 'keys-line' });

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
  document.body.dataset.phase = phase;
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
      document.body.dataset.phase = phase;
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
      // The first button is the way on, and Enter presses it (Sam asked to
      // go on without the mouse): it takes the focus, and the page-wide Enter
      // below finds it if the focus has wandered.
      if (kids.some((k) => k.querySelector && (k.matches('button') || k.querySelector('button')))) {
        kids.push(say('results.key_hint', { key: keyName('Enter') }, { class: 'desc' }));
      }
      panel.replaceChildren(...kids);
      const first = panel.querySelector('button');
      if (first) first.focus({ preventScroll: true });
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
    item('menu.road', road),
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
  // Every control, in the order it is easiest to learn: the arm, moving,
  // the jump and the dodge, then what a blade does, then the tricks.
  const steps = ['shoulder', 'elbow', 'move', 'jump', 'air_jump', 'stand', 'dodge', 'roll', 'air_dodge', 'cooldown',
    'cut', 'block', 'plant', 'swing', 'ink'];
  show(
    say('practice.intro'),
    keysLine(binding),
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

// --- the road (D16) ----------------------------------------------------------------

function road() {
  stop();
  const stops = JSON.parse(road_json());
  const cleared = new Set(SAVE.state.road.cleared);
  const next = stops.find((s) => !cleared.has(s.id));
  const cards = stops.map((s) => {
    const o = (k) => `opponents.${s.id}.${k}`;
    const mid = { opponent_mid: t(o('name_mid')) };
    let action;
    if (cleared.has(s.id)) action = [say('road.cleared', mid, { class: 'desc' }), button('road.fight.label', () => fight(s.id), mid)];
    else if (next && next.id === s.id) action = [button('road.fight.label', () => fight(s.id), mid)];
    else action = [say('road.locked', {}, { class: 'desc' })];
    return el('section', { class: 'stop', 'data-stop': s.id },
      el('h3', { 'data-copy': o('name') }, t(o('name'))),
      say(o('place'), {}, { class: 'desc' }),
      el('h4', { 'data-copy': 'road.does_heading' }, t('road.does_heading')),
      say(o('does'), s.numbers),
      el('h4', { 'data-copy': 'road.try_heading' }, t('road.try_heading')),
      say(o('try'), s.numbers),
      ...action);
  });
  show(say('road.intro'), ...(next ? [] : [say('road.end')]), ...cards, button('menu.back.label', menu));
}

function fight(id) {
  READY = false;
  let won = false;
  const stops = JSON.parse(road_json()).map((s) => s.id);
  const next = stops[stops.indexOf(id) + 1];
  const watch = matchWatcher(id, () => {
    // After a win, the first button is the next stop on the road; after a
    // loss, this opponent again. Enter presses the first.
    const again = button('results.again.label', () => fight(id));
    const onward = next && game && game.won && game.won()
      ? button('road.fight.label', () => fight(next), { opponent_mid: t(`opponents.${next}.name_mid`) })
      : null;
    return [el('div', { class: 'actions' },
      onward, again,
      button('results.to_road.label', road),
      button('results.replay.label', () => download(game.replay_bytes(), 'vagrancy.replay')))];
  });
  show(watch.panel, keysLine(BINDINGS.solo), el('div', { class: 'actions' }, button('results.to_road.label', road)));
  const g = new Road(seed(), tuning(), id);
  start(g, withReady(() => [bits(BINDINGS.solo, ACTION_BITS), 0]), (f) => {
    watch.tick(f);
    if (!won && game && game.won && game.won()) {
      won = true;
      if (!SAVE.state.road.cleared.includes(id)) SAVE.state.road.cleared.push(id);
      persist();
    }
  });
}

// --- the save file (D15) ----------------------------------------------------------------

const AUTOSAVE = 'vagrancy.autosave';

// Write the convenience copy. Core reads it back first, so a state the page
// got wrong is never kept.
function persist() {
  SAVE.state.bindings = BINDINGS;
  try {
    const text = save_read(JSON.stringify(SAVE));
    localStorage.setItem(AUTOSAVE, text);
  } catch (e) { console.warn(e); }
}

function restore() {
  let text = null;
  try { text = localStorage.getItem(AUTOSAVE); } catch (e) { /* storage off */ }
  try {
    SAVE = JSON.parse(save_read(text || save_fresh()));
  } catch (e) {
    SAVE = JSON.parse(save_fresh());
  }
  BINDINGS = SAVE.state.bindings;
}

let SAVE_NOTE = null; // { key, vars } after a load

function saveSection() {
  return el('section', { id: 'save' },
    el('h2', { 'data-copy': 'settings.save.title' }, t('settings.save.title')),
    el('div', { class: 'actions' },
      button('settings.save.download.label', () => download(save_read(JSON.stringify(SAVE)), 'vagrancy.save.json', 'application/json')),
      button('settings.save.load.label', async () => {
        const file = await pick('.json,application/json');
        if (!file) return;
        try {
          SAVE = JSON.parse(save_read(new TextDecoder().decode(file.bytes)));
          BINDINGS = SAVE.state.bindings;
          persist();
          SAVE_NOTE = { key: 'settings.save.loaded', vars: {} };
        } catch (err) {
          SAVE_NOTE = JSON.parse(err);
        }
        settings();
      })),
    say('settings.save.download.desc', {}, { class: 'desc' }),
    SAVE_NOTE ? say(SAVE_NOTE.key, SAVE_NOTE.vars, { role: 'status', id: 'save-note' }) : null,
    say('settings.save.autosave', {}, { class: 'desc' }),
  );
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
  const net = { sess, peer: null, error: null, link: null, timer: null, started: false, shown: null };
  NET = net;
  // Redraw the lobby only when what it says has changed. It used to redraw
  // ten times a second, which replaced the Start button under the player's
  // mouse: a press on one button and a release on its replacement is not a
  // click, so Start did nothing (reported by Sam; SECOND-ORDER-M5).
  net.paint = (force) => {
    const now = JSON.stringify([sess.status(), net.error, net.left, net.peer !== null]);
    if (!force && now === net.shown) return;
    net.shown = now;
    paint();
  };
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
      if (net.peer === null) { net.peer = id; flush(); net.paint(); } else net.link.send(id, Online.refusal_full());
    },
    onLeft: (id) => { if (id === net.peer) { net.left = true; net.paint(); } },
    onMessage: (id, bytes) => {
      if (id !== net.peer) return;
      sess.receive(performance.now(), bytes);
      flush();
      if (!net.started && JSON.parse(sess.status()).kind === 'playing') beginOnline(net);
      if (!net.started) net.paint();
    },
    onError: (key) => { net.error = key; net.paint(); },
  });
  net.timer = setInterval(() => { sess.poll(performance.now()); flush(); if (!net.started) net.paint(); }, 100);
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
    // A first exchange both players can see (Sam): each says they are ready,
    // each sees the other's answer, and the host's Start appears once both
    // have.
    const host = net.sess.seat() === 0;
    const lines = [say(host ? 'online.connected.host' : 'online.connected.join', { delay_ms: st.delay_ms })];
    lines.push(st.me_ready
      ? say('online.ready.you')
      : button('online.ready.label', () => { net.sess.ready(); net.flush(); net.paint(true); }));
    lines.push(say(st.them_ready ? 'online.ready.friend' : 'online.ready.friend_not'));
    if (host && st.me_ready && st.them_ready) {
      lines.push(button('online.start.label', () => { net.sess.start(performance.now()); net.flush(); beginOnline(net); }));
    }
    return lines;
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
  show(watch.panel, status, keysLine(BINDINGS.solo), el('div', { class: 'actions' }, button('menu.back.label', online)));
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
  show(musicSection(), keysSection(), saveSection(), button('menu.back.label', menu));
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
  vol.addEventListener('input', () => {
    music.setVolume(Number(vol.value) / 100);
    SAVE.state.options.music_volume = Number(vol.value);
    persist();
  });
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

// Key bindings live in the save file (D15), and the save's convenience copy
// in this browser.
function saveBindings() {
  persist();
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
      BINDINGS = JSON.parse(save_fresh()).state.bindings;
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
  restore();
  listen((code) => game && Object.values(BINDINGS).some((b) => Object.values(b).includes(code)));
  draw = renderer($('stage'), PALETTE, N);
  music.subscribe(({ error }) => {
    MUSIC_ERROR = error || null;
    if (document.getElementById('music')) settings();
  });
  music.setVolume(SAVE.state.options.music_volume / 100);
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
    save: () => SAVE,
    online: () => NET && { status: JSON.parse(NET.sess.status()), tick: NET.sess.tick(), checksum: NET.sess.checksum() },
  };
  // Enter goes on: it presses the first button of a result, wherever the
  // focus is.
  window.addEventListener('keydown', (e) => {
    if (e.code !== 'Enter' && e.code !== 'NumpadEnter') return;
    const first = document.querySelector('#result button');
    if (first && document.activeElement !== first && !(document.activeElement && document.activeElement.matches('input, textarea'))) {
      e.preventDefault();
      first.click();
    }
  });
  const m = location.hash.match(/^#room=([A-Z0-9]{4,12})$/);
  if (m) joinRoom(m[1]); else menu();
  requestAnimationFrame(loop);
  document.body.dataset.ready = '1';
}

main();
