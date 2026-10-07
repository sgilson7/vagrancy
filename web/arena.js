// The stream's arena (Sam, 2026-10-07): fights shown full screen with both
// trees over the fighters, for a Twitch stream run by a local bot
// (vagrancy-arena). The bot, on the streaming machine, tells this page what
// to show over a WebSocket on 127.0.0.1 (`?bot=ws://127.0.0.1:8787`) and
// hears the results; with no bot, the page runs exhibitions on its own. Like
// the game's page it draws what core sends and decides nothing about a
// match; every word is a copy string, and viewers' names are drawn as data.
import init, {
  copy_json, palette_json, numbers as coreNumbers, tree_json, Lab, Online, lab_roster_json, bubbles_step,
  lab_check, lab_describe_json,
} from './pkg/vagrancy_wasm.js';
import { renderer } from './draw.js';
import * as rtc from './rtc.js';

const BUILD = '__BUILD__';
let COPY, N, PAL, ROSTER, draw;
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
  for (const [k, v] of Object.entries(attrs)) if (v !== undefined && v !== null) e.setAttribute(k, v);
  for (const k of kids) if (k != null && k !== false) e.append(k);
  return e;
}
const say = (key, vars, attrs = {}) => el('p', { 'data-copy': key, ...attrs }, t(key, vars));
const fill = (text, attrs = {}) => el('span', { 'data-fill': '', ...attrs }, String(text));

// --- what is on screen ----------------------------------------------------------------------

let game = null;       // a Lab for an exhibition, or a challenge (below)
let fighters = [];     // [{ name, tree, by }], left then right
let challenge = null;  // { sess, link, peer, viewer, opponent, started, timer }
let prev = null, cur = null, acc = 0, last = 0, reported = false, nextAt = 0;
let bot = null;
// Whether the trees are drawn over the fighters: the bot can turn them off
// for a fight (Sam, 2026-10-07: "turn off the behavior trees sometimes").
let showTrees = true;
// How fast an exhibition plays, as the bot says (chat's !speed). A live
// challenge stays at full speed: its other player is in real time.
let speed = 1;
// Each round's end played again slowly, closing in on the cut that decided
// it, with the clock held, so one round reads apart from the next on stream
// (Sam, 2026-10-07). The frames are the last ones core sent; nothing is
// integrated. A live challenge is not held: its other player is in real time.
const REPLAY_TICKS = 60, REPLAY_SPEED = 0.3;
let recent = [], replay = null, holdUntil = 0, lastPhase = 'fight';

function nameOf(f) { return f.id ? t(`opponents.${f.id}.name`) : (f.name || t('btlab.editor.unnamed')); }
function treeFor(f) {
  const fillText = (n) => { n.text = t(n.label.key, n.label.vars); n.children.forEach(fillText); return n; };
  return fillText(JSON.parse(f.id ? tree_json(f.id) : lab_describe_json(f.tree)));
}

function exhibition(left, right) {
  endChallenge();
  const seed = (Math.random() * 0xffffffff) >>> 0;
  if (left.tree) game = Lab.watch_custom(seed, N.default_tuning, left.tree, right.id, 'flat');
  else game = Lab.watch(seed, N.default_tuning, left.id, right.id, 'flat');
  fighters = [{ ...left, treeData: treeFor(left) }, { ...right, treeData: treeFor(right) }];
  begin();
}

function begin() {
  cur = JSON.parse(game.frame());
  prev = null;
  acc = 0;
  recent = [];
  replay = null;
  holdUntil = 0;
  lastPhase = 'fight';
  reported = false;
  draw.reset();
  names();
  $('arena-result').replaceChildren();
}

function names() {
  const box = $('arena-names');
  if (!fighters.length) { box.replaceChildren(); return; }
  const side = (f, k) => el('div', { class: `fighter-name side-${k}` }, el('strong', { 'data-fill': '' }, nameOf(f)),
    f.by ? el('span', {}, ' ', el('span', { 'data-copy': 'arena.by' }, t('arena.by')), ' ', fill(f.by)) : null);
  box.replaceChildren(side(fighters[0], 0), say('arena.versus', {}, { class: 'versus' }), side(fighters[1], 1));
}

function loop(now) {
  const tickMs = 1000 / N.ticks_per_second;
  if (game) {
    acc += Math.min(now - (last || now), 250) * (challenge ? 1 : speed);
    if (!challenge && performance.now() < holdUntil) acc = 0;
    let n = 0;
    while (acc >= tickMs && n < 8) {
      if (challenge) {
        if (!challenge.started) break;
        challenge.sess.step_piloted(performance.now());
        challenge.flush();
        if (!challenge.sess.playing()) break;
        prev = cur; cur = JSON.parse(challenge.sess.frame());
      } else {
        game.step(0, 0);
        prev = cur; cur = JSON.parse(game.frame());
      }
      draw.events(cur);
      recent.push(cur);
      if (recent.length > REPLAY_TICKS) recent.shift();
      if (cur.phase !== lastPhase) {
        const was = lastPhase;
        lastPhase = cur.phase;
        if (was === 'fight') roundEnded();
        else if (cur.phase === 'fight') roundCard(cur.round);
        if (!challenge && performance.now() < holdUntil) { acc = 0; break; }
      }
      acc -= tickMs;
      n += 1;
    }
    if (cur && cur.phase) {
      const traces = challenge ? [JSON.parse(challenge.sess.pilot_report())].filter(Boolean) : JSON.parse(game.report());
      draw.trees(!showTrees ? [] : traces.map((r) => ({ seat: r.seat, tree: challenge ? fighters[1].treeData : fighters[r.seat].treeData, trace: r, caption: (nd) => t('tree.now', { node: nd.text }) })));
      if (replay && performance.now() < holdUntil) {
        const f = ((performance.now() - replay.start) * REPLAY_SPEED) / tickMs;
        const i = Math.min(replay.frames.length - 1, Math.floor(f)), j = Math.min(replay.frames.length - 1, i + 1);
        draw(replay.frames[i], replay.frames[j], i === j ? 1 : f - i);
      } else {
        replay = null;
        draw(prev || cur, cur, Math.min(1, acc / tickMs));
      }
      if (cur.phase === 'match_over' && !reported && performance.now() >= holdUntil) finish();
    }
  }
  if (!bot && !challenge && nextAt && now > nextAt) { nextAt = 0; randomExhibition(); }
  last = now;
  requestAnimationFrame(loop);
}

// A round has just ended: say how, and play its end again slowly toward
// the deciding cut while the clock is held.
function roundEnded() {
  const said = JSON.parse(game.phase_text(''));
  if (!said) return;
  $('arena-result').replaceChildren(el('p', { class: 'round-end', 'data-copy': said.popup.key }, t(said.popup.key, said.popup.vars)));
  if (challenge || !said.focus) return;
  const start = performance.now();
  const dur = (REPLAY_TICKS * 1000) / N.ticks_per_second / REPLAY_SPEED + 900;
  holdUntil = start + dur;
  replay = recent.length > 1 ? { frames: recent.slice(), start } : null;
  draw.focus({ at: said.focus, start, dur, still: false });
}
// The next round named, so the stream can tell one round from the last.
function roundCard(round) {
  draw.focus(null);
  $('arena-result').replaceChildren(el('p', { class: 'round-start', 'data-copy': 'hud.round' }, t('hud.round', { round })));
}

function finish() {
  reported = true;
  const winner = cur.wins[0] > cur.wins[1] ? 0 : cur.wins[1] > cur.wins[0] ? 1 : null;
  const who = winner === null ? null : (challenge ? (winner === challenge.seat ? nameOf(fighters[1]) : challenge.viewer) : nameOf(fighters[winner]));
  $('arena-result').replaceChildren(who === null ? say('arena.draw') : el('p', {}, fill(who), ' ', el('span', { 'data-copy': 'arena.won' }, t('arena.won', { wins: Math.max(...cur.wins), losses: Math.min(...cur.wins) }))));
  send({ type: 'result', winner: challenge ? (winner === null ? null : (winner === challenge.seat ? 'arena' : 'viewer')) : winner, wins: cur.wins });
  if (challenge) setTimeout(endChallenge, 6000);
  if (!bot) nextAt = performance.now() + 6000;
}

// --- a viewer's challenge: the arena hosts an online room, a pilot plays its side ----------

function challengeRoom(code, viewer, opponent) {
  endChallenge();
  const seed = (Math.random() * 0xffffffff) >>> 0;
  const stop = ROSTER.find((r) => r.id === opponent);
  const sess = Online.host(seed, N.default_tuning, BUILD, (stop && stop.weapon) || 'sword', 'flat');
  sess.set_pilot(opponent);
  const c = { sess, peer: null, viewer, opponent, started: false, seat: sess.seat(), link: null };
  c.flush = () => {
    if (c.peer === null) return;
    const out = c.sess.outbox();
    const view = new DataView(out.buffer, out.byteOffset, out.byteLength);
    for (let i = 0; i < out.length;) { const n = view.getUint32(i, true); c.link.send(c.peer, out.slice(i + 4, i + 4 + n)); i += 4 + n; }
  };
  c.link = rtc.open({
    isHost: true, mode: 'room', room: code, build: BUILD,
    onPeer: (id) => { if (c.peer === null) { c.peer = id; c.flush(); } else c.link.send(id, Online.refusal_full()); },
    onLeft: (id) => { if (id === c.peer) { send({ type: 'challenge', state: 'left' }); endChallenge(); } },
    onMessage: (id, bytes) => {
      if (id !== c.peer) return;
      c.sess.receive(performance.now(), bytes);
      const st = JSON.parse(c.sess.status());
      if (st.kind === 'connected' && !st.me_ready) c.sess.ready();
      if (st.kind === 'connected' && st.me_ready && st.them_ready && !c.started) { c.sess.start(performance.now()); c.started = true; send({ type: 'challenge', state: 'playing' }); }
      c.flush();
    },
    onError: () => send({ type: 'challenge', state: 'error' }),
  });
  c.timer = setInterval(() => { c.sess.poll(performance.now()); c.flush(); }, 100);
  challenge = c;
  game = { report: () => '[]' };
  fighters = [{ name: viewer, by: null }, { id: opponent, treeData: treeFor({ id: opponent }) }];
  cur = null;
  reported = false;
  draw.reset();
  names();
  $('arena-result').replaceChildren(el('p', { class: 'join' }, el('span', { 'data-copy': 'arena.join' }, t('arena.join')), ' ', fill(code, { class: 'code' })),
    el('p', {}, el('span', { 'data-copy': 'arena.join_for' }, t('arena.join_for')), ' ', fill(viewer)));
}

function endChallenge() {
  if (!challenge) return;
  clearInterval(challenge.timer);
  try { challenge.link.close(); } catch { /* already closed */ }
  challenge = null;
  game = null;
}

// --- the side panel --------------------------------------------------------------------------

function predict(p) {
  const total = Math.max(1, p.left + p.right);
  const bar = (k, n) => el('div', { class: `bar side-${k}` }, el('div', { class: 'fill', style: `width:${Math.round(100 * n / total)}%` }), fill(n));
  $('arena-predict').replaceChildren(
    el('h2', { 'data-copy': 'arena.predict.heading' }, t('arena.predict.heading')),
    p.open ? say('arena.predict.open', { seconds: p.seconds }) : say('arena.predict.closed'),
    bar(0, p.left), bar(1, p.right));
}
// How to take part, with the commands the bot says it answers.
function how(cmds) {
  if (!cmds) { $('arena-how').replaceChildren(); return; }
  $('arena-how').replaceChildren(el('h2', { 'data-copy': 'arena.how.heading' }, t('arena.how.heading')),
    el('ul', {},
      el('li', { 'data-copy': 'arena.how.predict' }, t('arena.how.predict', { left: cmds.left, right: cmds.right })),
      el('li', { 'data-copy': 'arena.how.xp' }, t('arena.how.xp', { command: cmds.xp })),
      el('li', { 'data-copy': 'arena.how.fight' }, t('arena.how.fight', { command: cmds.fight, cost: cmds.cost })),
      el('li', { 'data-copy': 'arena.how.submit' }, t('arena.how.submit', { command: cmds.submit }))));
}
function list(boxId, headKey, items, line) {
  $(boxId).replaceChildren(el('h2', { 'data-copy': headKey }, t(headKey)),
    items.length ? el('ol', {}, ...items.map(line)) : say('arena.empty'));
}

// --- the bot -------------------------------------------------------------------------------

function send(msg) { if (bot && bot.readyState === 1) bot.send(JSON.stringify(msg)); }
function connect(url) {
  bot = new WebSocket(url);
  bot.onopen = () => send({ type: 'hello', build: BUILD, roster: ROSTER.map((r) => r.id) });
  bot.onmessage = (e) => {
    const m = JSON.parse(e.data);
    if (m.type === 'exhibition') exhibition(m.left, m.right);
    else if (m.type === 'challenge') challengeRoom(m.code, m.viewer, m.opponent);
    else if (m.type === 'predict') predict(m);
    else if (m.type === 'queue') list('arena-queue', 'arena.queue', m.items, (q) => el('li', {}, fill(q.viewer), ' ', el('span', { 'data-copy': `arena.kind.${q.kind}` }, t(`arena.kind.${q.kind}`))));
    else if (m.type === 'leaders') list('arena-leaders', 'arena.leaders', m.items, (q) => el('li', {}, fill(q.viewer), ' ', fill(q.xp)));
    else if (m.type === 'how') how(m);
    else if (m.type === 'trees') showTrees = !!m.show;
    else if (m.type === 'speed' && [0.25, 0.5, 1, 2].includes(m.value)) speed = m.value;
    else if (m.type === 'check') send({ type: 'checked', id: m.id, ...JSON.parse(lab_check(m.tree)) });
  };
  bot.onclose = () => { bot = null; setTimeout(() => connect(url), 3000); };
  bot.onerror = () => {};
}

function randomExhibition() {
  const fightable = ROSTER.filter((r) => r.kind !== 'still');
  const pick = () => fightable[Math.floor(Math.random() * fightable.length)];
  exhibition({ id: pick().id }, { id: pick().id });
}

async function main() {
  try {
    await init();
    COPY = JSON.parse(copy_json());
    N = JSON.parse(coreNumbers());
    PAL = JSON.parse(palette_json());
    ROSTER = JSON.parse(lab_roster_json());
  } catch (e) {
    console.error(e);
    $('loading-error').hidden = false;
    return;
  }
  draw = renderer($('stage'), PAL, N);
  draw.bubbleLayout((list0, w, h) => JSON.parse(bubbles_step(JSON.stringify(list0), w, h)));
  $('status').hidden = true;
  $('arena-main').hidden = false;
  how(null);
  predict({ open: false, left: 0, right: 0, seconds: 0 });
  list('arena-queue', 'arena.queue', [], () => null);
  list('arena-leaders', 'arena.leaders', [], () => null);
  const url = new URLSearchParams(location.search).get('bot');
  if (url && /^ws:\/\/(127\.0\.0\.1|localhost)(:\d+)?\/?$/.test(url)) connect(url);
  else randomExhibition();
  document.body.dataset.ready = '1';
  window.arena = { state: () => ({ fighters: fighters.map(nameOf), tick: cur ? cur.tick : 0, phase: cur && cur.phase, challenge: !!challenge, trees: showTrees, speed, held: performance.now() < holdUntil, replaying: !!replay }) };
  requestAnimationFrame(loop);
}
main();
