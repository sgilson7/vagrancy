// The page. It draws numbers core sent and decides nothing: no physics, no
// contact, no copy of a constant (CLAUDE.md). Every word it shows comes from
// data/copy.en.json, which reaches it through the wasm module; which sentence
// to show for an outcome is chosen by core (content::messages).
import init, {
  copy_json, palette_json, controls_json, numbers as coreNumbers, script_checksum, Game, Online, Road, Mission, StoryRun, Exhibition, story_json,
  road_json, tutorial_json, weapons_json, slosh_step, agent_next, maps_json, tree_json, save_choose_weapon, save_fresh, save_read,
  arms_json, save_choose_arms, bubbles_step, story_extra_json,
} from './pkg/vagrancy_wasm.js';
import * as rtc from './rtc.js';
import * as sfx from './sfx.js';
import { renderer, cursedStrands } from './draw.js';
import { listen, bits, keyName } from './keys.js';
import { download, pick } from './files.js';
import * as music from './music.js';
import * as youtube from './youtube.js';

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
// How fast real time feeds the clock: 1, or a half or a quarter in watch
// mode, so the trees are easier to follow (Sam, 2026-10-06). The world steps
// the same ticks, fewer of them each second; nothing about a match changes.
let SPEED = 1;
let last = 0;
let draw = null;
// Until when the clock is held as a round ends, and where to look.
let FREEZE = null;
// Each round's end is played again slowly, closing in on the cut that
// decided it, before the next round (Sam, 2026-10-07: "each round ends
// instantly and it looks wacky. when someone dies, it should slow down and
// zoom into the cut ... then zoom back out and start the next round").
// The page keeps the last REPLAY_TICKS frames core sent and draws them
// again at REPLAY_SPEED; it integrates nothing.
// First the fight runs on slowly for a moment, so the player sees who fell
// (Sam: "a samurai pause moment to see who died"); then, unless the player
// turned it off, the replay, closing in on the cut.
const PAUSE_MS = 1600;
const PAUSE_SPEED = 0.4;
const REPLAY_TICKS = 90;
const REPLAY_SPEED = 0.4;
const OUT_MS = 700;
let RECENT = [];
let REPLAY = null; // { frames, start }
let ENDING = null; // { pauseUntil, replayed }
// Whether each round's end is replayed: this browser's choice, on unless
// turned off in Settings (Sam: "you have to be able to turn off the
// extended end of round replays").
function replaysOn() {
  try { return localStorage.getItem('vagrancy.roundReplays') !== 'off'; } catch { return true; }
}
function holdMs() {
  return PAUSE_MS + (replaysOn() && !stillMotion() ? Math.round((REPLAY_TICKS * 1000) / 60 / REPLAY_SPEED) : 0) + OUT_MS;
}
// And for at least this many drawn frames: where frames come slowly (CI's
// headless Firefox, a loaded laptop), a hold measured in time alone could
// end between two frames, so the card showed for one frame and was gone
// (SECOND-ORDER-M5 row 53). At 60 frames a second this is well inside
// HOLD_MS, so it changes nothing there.
const HOLD_FRAMES = 24;
const stillMotion = () => window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
let onTick = null;

function start(g, seatFn, tickFn = null) {
  game = g;
  seats = seatFn;
  onTick = tickFn;
  curFrame = JSON.parse(game.frame());
  prevFrame = null;
  acc = 0;
  RECENT = [];
  REPLAY = null;
  ENDING = null;
  draw.reset();
  $('stage').hidden = false;
  music.fight(!game.is_replay());
}

// The card that comes up over the arena when a round ends.
function popupShow(said) {
  const box = $('death-popup');
  box.replaceChildren(
    el('p', { class: 'popup-head', 'data-copy': said.popup.key }, t(said.popup.key, said.popup.vars)),
    sayChosen(said.round, { class: 'popup-line' }));
  box.classList.toggle('headshot', !!said.headshot);
  // The winner's color; a draw keeps the paper's.
  box.classList.remove('won-0', 'won-1');
  if (said.popup.winner === 0 || said.popup.winner === 1) box.classList.add(`won-${said.popup.winner}`);
  box.hidden = false;
}
// The card for a won match on the road, in the same box: what the win
// opened, or, when it opened nothing, the goal core picked for next.
function popupMatchWon(opened, next, revealed = []) {
  const box = $('death-popup');
  const kids = [el('p', { class: 'popup-head', 'data-copy': 'results.popup.match_won' }, t('results.popup.match_won'))];
  for (const o of revealed) kids.push(say('road.revealed', { opponent: t(`opponents.${o}.name`) }, { class: 'popup-line' }));
  for (const o of opened) kids.push(say('road.opened', { opponent: t(`opponents.${o}.name`) }, { class: 'popup-line' }));
  if (!opened.length && next) {
    kids.push(say('road.next', { target_mid: t(`opponents.${next.open}.name_mid`) }, { class: 'popup-line' }));
    kids.push(reqLine(next, { class: 'popup-line' }));
    if (next.key === 'road.req.with') kids.push(say('road.next_carry', { weapon: t(`weapons.${next.vars.weapon}.name`) }, { class: 'popup-line' }));
  }
  box.replaceChildren(...kids);
  box.classList.remove('headshot', 'won-1');
  box.classList.add('won-0');
  box.hidden = false;
}
// "Round 2" over the arena as a round starts, for a moment.
function popupRound(round) {
  const box = $('death-popup');
  box.replaceChildren(el('p', { class: 'popup-head', 'data-copy': 'hud.round' }, t('hud.round', { round })));
  box.classList.remove('headshot', 'won-0', 'won-1');
  box.hidden = false;
  setTimeout(() => { if (box.firstChild && box.firstChild.dataset.copy === 'hud.round') box.hidden = true; }, 1100);
}
function popupHide() {
  const box = $('death-popup');
  if (box) { box.hidden = true; box.replaceChildren(); }
  FREEZE = null;
  if (draw && draw.focus) draw.focus(null);
}

function stop() {
  popupHide();
  if (draw && draw.trees) draw.trees([]);
  game = null;
  SPEED = 1;
  $('stage').hidden = true;
  $('hud').hidden = true;
  music.fight(false);
}

function loop(now) {
  const tickMs = 1000 / N.ticks_per_second;
  if (game) {
    // A round has just ended: the fight runs on slowly for a moment, then
    // its end is played again (if wanted) with the clock held.
    let pace = SPEED;
    // Where the next round waits for the player (arcade mode, story mode),
    // the fight runs on slowly; elsewhere the seats are ready the tick a
    // round ends (two pilots, a recorded replay), so the pause holds still.
    if (ENDING && performance.now() < ENDING.pauseUntil) pace *= game instanceof Road || game instanceof StoryRun ? PAUSE_SPEED : 0;
    else if (ENDING && !ENDING.replayed) {
      ENDING.replayed = true;
      const ms = holdMs() - PAUSE_MS;
      if (replaysOn() && !stillMotion() && RECENT.length > 1) {
        REPLAY = { frames: RECENT.slice(), start: performance.now() };
        // Only the replay closes in (Sam: "the initial kill should not zoom
        // in, only the replay should").
        draw.focus({ at: ENDING.at, start: REPLAY.start, dur: ms, still: false });
      }
      FREEZE = { until: performance.now() + ms - OUT_MS, frames: 0 };
    }
    acc += Math.min(now - (last || now), 250) * pace;
    // Catch up at most eight ticks a frame, so a stalled tab does not
    // replay a burst of stale keys (Floodline caps its catch-up the same way).
    let n = 0;
    // A headshot holds the clock while the view closes in on it. Read on
    // performance.now(), the clock the hold was set on: the frame's own time
    // need not agree with it (CI's headless Firefox ran past every hold).
    if (FREEZE && (performance.now() < FREEZE.until || FREEZE.frames < HOLD_FRAMES)) {
      FREEZE.frames += 1;
      acc = 0;
    } else FREEZE = null;
    while (acc >= tickMs && n < 8) {
      const [a, b] = seats();
      game.step(a, b);
      prevFrame = curFrame;
      curFrame = JSON.parse(game.frame());
      draw.events(curFrame);
      RECENT.push(curFrame);
      if (RECENT.length > REPLAY_TICKS) RECENT.shift();
      acc -= tickMs;
      n += 1;
      if (onTick) onTick(curFrame);
      if (!game) break;
      // A hold that began on this tick stops the batch here. Without this,
      // a slow frame's catch-up played on past the headshot into the next
      // round before the card was ever drawn (CI's Firefox, 2026-10-06:
      // the page reached the replay's end with the hold never seen).
      if (FREEZE) { acc = 0; break; }
    }
    if (game) {
      // The round's end again, slowly, then held on its last frame while
      // the view draws back out.
      if (REPLAY && FREEZE) {
        const f = ((performance.now() - REPLAY.start) * REPLAY_SPEED) / tickMs;
        const i = Math.min(REPLAY.frames.length - 1, Math.floor(f));
        const j = Math.min(REPLAY.frames.length - 1, i + 1);
        draw(REPLAY.frames[i], REPLAY.frames[j], i === j ? 1 : f - i);
      } else {
        REPLAY = null;
        draw(prevFrame, curFrame, Math.min(1, acc / tickMs));
      }
    }
  }
  last = now;
  requestAnimationFrame(loop);
}

// --- the match: HUD, rounds, results -------------------------------------------

// Watches the frames of a match and shows what core says about its phase.
// `opponent` is a road opponent's id, or '' for versus. `onEnd` builds the
// buttons for the end of the match.
function matchWatcher(opponent, endButtons, names = null) {
  // Each side's name on the score line: the fighters' colors unless the
  // caller names them (watch mode names the opponents).
  const [leftName, rightName] = names || [t('fighters.left.name'), t('fighters.right.name')];
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
          left_name: leftName, left_wins: frame.wins[0],
          right_name: rightName, right_wins: frame.wins[1],
        })),
      );
      if (frame.phase === phase) return;
      phase = frame.phase;
      document.body.dataset.phase = phase;
      if (phase === 'fight') {
        panel.replaceChildren();
        popupHide();
        // The next round named over the arena, so one round reads apart
        // from the last.
        ENDING = null;
        REPLAY = null;
        if (frame.round > 1) popupRound(frame.round);
        return;
      }
      const said = JSON.parse(game.phase_text(opponent));
      // How the round ended, over the arena (Sam: "pop a popup on the
      // screen about how someone died"), and for a headshot, the clock held
      // while the view closes in where the blade landed.
      popupShow(said);
      if (said.focus) {
        const start = performance.now();
        ENDING = { pauseUntil: start + PAUSE_MS, replayed: false, at: said.focus };
      }
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
  const item = (key, action) => el('div', { class: 'item' }, button(`${key}.label`, action), say(`${key}.desc`, key === 'menu.story' ? { lives: JSON.parse(story_json()).lives } : {}, { class: 'desc' }));
  // Arcade mode first, set apart (Sam, 2026-10-06): its door makes a sound
  // and opens on the chart.
  const arcade = el('div', { class: 'item featured' },
    button('menu.road.label', () => {
      sfx.arcade(SAVE.state.options.music_volume / 100);
      setRoadView('chart');
      road();
    }),
    say('menu.road.desc', {}, { class: 'desc' }));
  show(
    arcade,
    item('menu.story', storyMode),
    item('menu.tutorial', tutorial),
    item('menu.train', train),
    item('menu.encyclopedia', encyclopedia),
    item('menu.watch', watchMode),
    item('menu.local', local),
    item('menu.online', online),
    item('menu.practice', practice),
    item('menu.replay', loadReplay),
    el('div', { class: 'item' }, button('menu.settings.label', settings)),
  );
}

// What each seat at one keyboard carries; both choose from what this save
// has unlocked.
const LOCAL_WEAPONS = ['sword', 'sword'];
// The ground for a match at one keyboard, and for one this page hosts.
const MAP_CHOICE = { local: 'flat', online: 'flat', watch: 'flat' };
// What this player carries online, from what the save has unlocked.
let ONLINE_WEAPON = null;

function mapPicker(which, labelKey) {
  const id = `map-${which}`;
  const ids = JSON.parse(maps_json()).maps.map((m) => m.id);
  const desc = say(`maps.${MAP_CHOICE[which]}.desc`, keyVars(BINDINGS.solo), { class: 'desc' });
  const pick = el('select', { id, on: { change: () => {
    MAP_CHOICE[which] = pick.value;
    desc.replaceWith(say(`maps.${pick.value}.desc`, keyVars(BINDINGS.solo), { class: 'desc' }));
  } } }, ...ids.map((m) => el('option', { value: m, 'data-copy': `maps.${m}.name` }, t(`maps.${m}.name`))));
  pick.value = MAP_CHOICE[which];
  return el('div', {}, el('p', {}, el('label', { for: id, 'data-copy': labelKey }, t(labelKey)), ' ', pick), desc);
}

function onlineWeaponPicker() {
  const open = weaponList().filter((w) => w.unlocked);
  if (!ONLINE_WEAPON || !open.some((w) => w.id === ONLINE_WEAPON)) ONLINE_WEAPON = SAVE.state.weapon;
  const pick = el('select', { id: 'weapon-online', on: { change: () => { ONLINE_WEAPON = pick.value; } } },
    ...open.map((w) => el('option', { value: w.id, 'data-copy': `weapons.${w.id}.name` }, t(`weapons.${w.id}.name`))));
  pick.value = ONLINE_WEAPON;
  return el('p', {}, el('label', { for: 'weapon-online', 'data-copy': 'online.weapon' }, t('online.weapon')), ' ', pick);
}

// The weapons this player may see, and how far the cursed blade has
// gathered (content::weapons, through the shim).
function weaponState() {
  return JSON.parse(weapons_json(JSON.stringify(SAVE)));
}
function weaponList() {
  return weaponState().weapons;
}

function weaponPicker(seat, labelKey, vars) {
  const id = `weapon-${seat}`;
  const open = weaponList().filter((w) => w.unlocked);
  if (!open.some((w) => w.id === LOCAL_WEAPONS[seat])) LOCAL_WEAPONS[seat] = 'sword';
  const pick = el('select', { id, on: { change: () => { LOCAL_WEAPONS[seat] = pick.value; } } },
    ...open.map((w) => el('option', { value: w.id, 'data-copy': `weapons.${w.id}.name` }, t(`weapons.${w.id}.name`))));
  pick.value = LOCAL_WEAPONS[seat];
  return el('p', {}, el('label', { for: id, 'data-copy': labelKey }, t(labelKey, vars)), ' ', pick);
}

function local() {
  stop();
  show(
    say('local.intro', {
      left_name: t('fighters.left.name'), left_keys: keyList(BINDINGS.left),
      right_name: t('fighters.right.name'), right_keys: keyList(BINDINGS.right),
    }),
    weaponPicker(0, 'local.weapon_left', { left_name: t('fighters.left.name') }),
    weaponPicker(1, 'local.weapon_right', { right_name: t('fighters.right.name') }),
    mapPicker('local', 'local.map'),
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
  start(Game.versus(seed(), tuning(), LOCAL_WEAPONS[0], LOCAL_WEAPONS[1], MAP_CHOICE.local),
    withReady(() => [bits(BINDINGS.left, ACTION_BITS), bits(BINDINGS.right, ACTION_BITS)]),
    (f) => watch.tick(f));
}

function practice() {
  const binding = BINDINGS.solo;
  const vars = keyVars(binding);
  // Every control, in the order it is easiest to learn: the arm, moving,
  // the jump and the dodge, then what a blade does, then the tricks.
  const steps = ['shoulder', 'elbow', 'move', 'jump', 'air_jump', 'stand', 'dodge', 'roll', 'air_dodge', 'cooldown',
    'cut', 'block', 'plant', 'swing', 'throw', 'ink'];
  show(
    keysLine(binding),
    el('ol', { id: 'steps' }, ...steps.map((k) => el('li', {}, say(`practice.step.${k}`, vars)))),
    el('div', { class: 'actions' },
      button('practice.reset.label', practice),
      button('replay.download.label', () => download(game.replay_bytes(), 'vagrancy.replay')),
      button('replay.load.label', loadReplay),
      button('menu.back.label', menu)),
  );
  start(Game.practice(seed(), tuning(), SAVE.state.weapon, !!SAVE.state.four_arms), () => [bits(binding, ACTION_BITS), 0]);
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

const AGENT = (() => {
  const v = new URLSearchParams(location.search).get('player');
  return v !== null && /^\d+$/.test(v) ? Number(v) : null;
})();
let AGENT_PLAYED = 0;

function seed() {
  // A fresh match gets a fresh spawn jitter. The seed travels in the replay,
  // so playback does not depend on this.
  return (Date.now() & 0x7fffffff) >>> 0;
}

// --- the road (D16) ----------------------------------------------------------------

// The road is a tree of fights (Sam): one row per number of requirements,
// the fight at the top open from the start, the hardest at the bottom. A
// line runs from each requirement's fight down to the fight it opens; a
// line or a fight under the pointer says what it asks for. After the
// errands map in gear-master-2d (web/app.js paintLog, drawChainWires).
const SVG = 'http://www.w3.org/2000/svg';
let ROAD_PICK = null;

function roadData() {
  return JSON.parse(road_json(JSON.stringify(SAVE)));
}

// One requirement as its sentence, with the opponent's name filled in.
function reqLine(r, attrs = {}) {
  const vars = { ...r.vars, opponent_mid: t(`opponents.${r.stop}.name_mid`) };
  // A weapon challenge names its weapon by id; the words are the copy's.
  if (r.vars && r.vars.weapon) vars.weapon = t(`weapons.${r.vars.weapon}.name`);
  return say(r.key, vars, attrs);
}

// A map of nodes in rows with a line from each requirement down to what
// it opens: the road's tree and the tutorial's missions. Hovering a node or
// a line says what it asks for; picking a node fills the card above.
//   nodes: [{ id, row, name: element text key, nameVars, state:
//            'open'|'won'|'flawless'|'locked', requires: [{ from, met, line() }] }]
//   rowLabel(row) -> element; detail(id, card) fills the card; first: id
function mapScreen({ nodes, rowLabel, detail, first, attr, before = [], layout = 'rows' }) {
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const rows = [];
  for (const n of nodes) (rows[n.row] ||= []).push(n);
  // Within a row, by where the requirements sit in the rows above, which
  // keeps the lines from crossing more than they must.
  const place = new Map();
  rows.forEach((row, l) => {
    if (!row) return;
    if (l > 0) {
      const at = (n) => n.requires.reduce((a, r) => a + (place.get(r.from) ?? 0), 0) / Math.max(1, n.requires.length);
      row.sort((a, b) => at(a) - at(b));
    }
    row.forEach((n, i) => place.set(n.id, i / Math.max(1, row.length - 1)));
  });
  const card = el('section', { id: 'stop-detail' });
  const tip = el('div', { id: 'road-tip', role: 'tooltip', hidden: '' });
  const buttons = new Map();
  const tree = el('div', { id: 'road-tree', class: layout });
  const wires = document.createElementNS(SVG, 'svg');
  wires.setAttribute('class', 'wires');
  wires.setAttribute('aria-hidden', 'true');
  tree.append(wires);
  const nodeButton = (n) => {
    const b = el('button', { type: 'button', class: `node ${n.state}${n.reward ? ' reward' : ''}${n.next ? ' next-up' : ''}`, [attr]: n.id, 'data-copy': n.name,
      on: {
        click: () => pick(n.id),
        mouseenter: () => hoverNode(n.id),
        focus: () => hoverNode(n.id),
        mouseleave: unhover,
        blur: unhover,
      } }, t(n.name, n.nameVars));
    // On the chart each fight stands in a small scene from its place
    // (analysis/art/seals.py, Sam 2026-10-07): the drover among cows, the
    // kite maker's seal a kite. The page greys or lights it by state.
    if (layout === 'chart') {
      b.replaceChildren(el('img', { class: 'seal-art', src: `art/seal/${n.id}.png`, alt: '', 'aria-hidden': 'true', draggable: 'false' }),
        el('span', { class: 'seal-name' }, t(n.name, n.nameVars)));
    }
    buttons.set(n.id, b);
    return b;
  };
  if (layout === 'chart') {
    // A chart, after Weapon Master's map: each row a region in its own band,
    // over that region's art (analysis/art/regions.py), its fights as seals
    // joined by routes. Wide and scrolled sideways, like a large skill tree
    // (Sam, 2026-10-07), so the fights sit farther apart and fewer routes
    // cross. The scatter is a fixed pattern, so the chart is the same each
    // time it is drawn.
    const BAND = 190, GAP = 200, EDGE = 90;
    const widest = Math.max(...rows.filter(Boolean).map((r) => r.length));
    const width = Math.max(1100, widest * GAP + 2 * EDGE);
    tree.style.width = `${width}px`;
    tree.style.height = `${rows.length * BAND}px`;
    rows.forEach((row, l) => {
      if (!row) return;
      // The art runs a little past the band on each side and fades out, so
      // each region melts into the next.
      const art = el('div', { class: 'band-art', 'aria-hidden': 'true' });
      art.style.top = `${l * BAND - 36}px`;
      art.style.height = `${BAND + 72}px`;
      art.style.backgroundImage = `url(art/region-${l}.png)`;
      tree.append(art);
      const band = el('div', { class: 'band', 'data-level': String(l) }, rowLabel(l));
      band.style.top = `${l * BAND}px`;
      band.style.height = `${BAND}px`;
      tree.append(band);
      const step = (width - 2 * EDGE) / row.length;
      row.forEach((n, i) => {
        const b = nodeButton(n);
        const x = EDGE + (i + 0.5) * step + Math.sin(l * 1.7 + i * 2.3) * step * 0.14;
        const y = l * BAND + 14 + (Math.cos(l * 1.3 + i * 1.9) + 1) * 10;
        b.style.left = `${x}px`;
        b.style.top = `${y}px`;
        tree.append(b);
      });
    });
  } else if (layout === 'sunburst') {
    // A sunburst laid out as separate paths (Sam, 2026-10-05: "4-8 distinct
    // and separable paths, using the chart style of connecting lines
    // between the highest impact pre-requisites"). Each fight hangs from
    // one prerequisite: its most connected requirement in the row directly
    // inside it (every fight has one there, as the chart checks). The
    // fights of the first row each start a path, a wedge running outward;
    // every later fight joins its prerequisite's wedge, and its ring is its
    // row. The open-from-the-start fights sit at the center.
    const degree = new Map(nodes.map((n) => [n.id, n.requires.length]));
    for (const n of nodes) for (const r of n.requires) degree.set(r.from, (degree.get(r.from) || 0) + 1);
    const parent = new Map();
    for (const n of nodes) {
      if (!n.requires.length) continue;
      const inner = n.requires.filter((r) => byId.get(r.from).row === n.row - 1);
      const pool = inner.length ? inner : n.requires;
      parent.set(n.id, pool.reduce((best, r) => (degree.get(r.from) > degree.get(best.from) ? r : best)).from);
    }
    const kids = new Map(nodes.map((n) => [n.id, []]));
    for (const [c, p] of parent) kids.get(p).push(c);
    const size = (id) => 1 + kids.get(id).reduce((t, c) => t + size(c), 0);
    // The paths: start from the first-row fights that lead somewhere, and
    // split the largest at its fight while it holds more than a quarter of
    // the tree, until there are no more than eight. A fight split this way
    // sits where its paths meet; a first-row fight that leads nowhere sits
    // in the inner ring with the center.
    let heads = nodes.filter((n) => n.row === 1 && kids.get(n.id).length).map((n) => n.id);
    const joints = [];
    const spread = nodes.filter((n) => n.row >= 1).length;
    for (;;) {
      const big = heads.reduce((m, h) => (size(h) > size(m) ? h : m), heads[0]);
      const grown = kids.get(big).filter((c) => kids.get(c).length);
      if (size(big) <= spread / 4 || !grown.length || heads.length - 1 + grown.length > 8) break;
      const i = heads.indexOf(big);
      heads.splice(i, 1, ...grown);
      joints.push(big);
    }
    // Each fight's path: the head its chain of prerequisites reaches first;
    // a leaf hanging from a joint joins its largest sibling's path.
    const below = (h, j) => { let x = h; while (x && x !== j) x = parent.get(x); return x === j; };
    const pathOf = (id) => {
      let x = id;
      while (x && !heads.includes(x)) {
        const p = parent.get(x);
        if (p && joints.includes(p)) {
          // A leaf under a joint joins the largest path below that joint.
          const under = heads.filter((h) => below(h, p));
          if (under.length) return under.reduce((m, c) => (size(c) > size(m) ? c : m));
        }
        x = p;
      }
      return x || heads[0];
    };
    const inner = new Set(nodes.filter((n) => n.row === 0 || (n.row === 1 && !heads.includes(n.id) && !joints.includes(n.id))).map((n) => n.id));
    const members = new Map(heads.map((h) => [h, []]));
    for (const n of nodes) {
      if (inner.has(n.id) || joints.includes(n.id)) continue;
      const h = pathOf(n.id);
      if (h) members.get(h).push(n);
    }
    // Each path's share of the circle follows its widest row, so no row of
    // it is crowded more than another path's.
    const widest = (h) => Math.max(...Object.values(members.get(h).reduce((c, n) => ({ ...c, [n.row]: (c[n.row] || 0) + 1 }), {})));
    const total = heads.reduce((t, h) => t + widest(h), 0);
    const wedges = [];
    let at = -Math.PI / 2;
    for (const h of heads) {
      const span = (2 * Math.PI * widest(h)) / total;
      wedges.push({ head: h, from: at, to: at + span });
      at += span;
    }
    const radius = (row) => (row === 0 ? 0 : 9 + 5.4 * row); // percent of the width
    const angle = new Map();
    const placeAt = (n, a, r, path) => {
      angle.set(n.id, a);
      const btn = nodeButton(n);
      btn.style.left = `${50 + Math.cos(a) * r}%`;
      btn.style.top = `${50 + Math.sin(a) * r}%`;
      btn.dataset.path = String(path);
      tree.append(btn);
    };
    // The center: the first fight in the middle, the other fights of the
    // first two rows that start no path in a ring close around it.
    const centre = nodes.filter((n) => inner.has(n.id));
    centre.forEach((n, i) => placeAt(n, i === 0 ? 0 : -Math.PI / 2 + ((i - 1) * 2 * Math.PI) / (centre.length - 1), i === 0 ? 0 : 8, -1));
    wedges.forEach((w, wi) => {
      const mine = members.get(w.head);
      const deepest = Math.max(...mine.map((n) => n.row));
      for (let row = 1; row <= deepest; row += 1) {
        const ring = mine.filter((n) => n.row === row);
        ring.sort((p, q) => (angle.get(parent.get(p.id)) ?? 0) - (angle.get(parent.get(q.id)) ?? 0));
        const pad = (w.to - w.from) * 0.08;
        ring.forEach((n, i) => {
          const a = ring.length === 1 ? (w.from + w.to) / 2 : w.from + pad + ((w.to - w.from - 2 * pad) * (i + 0.5)) / ring.length;
          placeAt(n, a, radius(row), wi);
        });
      }
    });
    // A joint sits on its row's ring, centred over the paths it split into.
    for (const j of joints.slice().reverse()) {
      const under = wedges.filter((w) => below(w.head, j));
      const a = under.length ? (under[0].from + under[under.length - 1].to) / 2 : 0;
      placeAt(byId.get(j), a, radius(byId.get(j).row), -1);
    }
    tree.sunburst = { wedges, parent, rings: Math.max(...nodes.map((n) => n.row)) + 1, radius };
  } else {
    rows.forEach((row, l) => {
      if (!row) return;
      const tier = el('div', { class: 'tier' }, ...row.map(nodeButton));
      tree.append(el('div', { class: 'level', 'data-level': String(l) }, rowLabel(l), tier));
    });
  }
  const paths = [];
  function wire() {
    wires.replaceChildren();
    paths.length = 0;
    const box = tree.getBoundingClientRect();
    wires.setAttribute('viewBox', `0 0 ${box.width} ${box.height}`);
    if (layout === 'sunburst') {
      // Each path a wedge, shaded in turn, with a line between neighbors,
      // so the paths read as separate; and the rows as faint rings.
      const { wedges, rings, radius } = tree.sunburst;
      const cx = box.width / 2, cy = box.height / 2;
      const R = (pct) => (box.width * pct) / 100;
      const r0 = R(radius(1) - 2.7), r1 = R(radius(rings - 1) + 3);
      wedges.forEach((w, i) => {
        const pt = (r, a) => `${cx + Math.cos(a) * r} ${cy + Math.sin(a) * r}`;
        const big = w.to - w.from > Math.PI ? 1 : 0;
        const sector = document.createElementNS(SVG, 'path');
        sector.setAttribute('d', `M ${pt(r0, w.from)} L ${pt(r1, w.from)} A ${r1} ${r1} 0 ${big} 1 ${pt(r1, w.to)} L ${pt(r0, w.to)} A ${r0} ${r0} 0 ${big} 0 ${pt(r0, w.from)} Z`);
        sector.setAttribute('class', `wedge wedge-${i % 2}`);
        const edge = document.createElementNS(SVG, 'path');
        edge.setAttribute('d', `M ${pt(r0, w.from)} L ${pt(r1, w.from)}`);
        edge.setAttribute('class', 'spoke');
        wires.append(sector, edge);
      });
      for (let i = 1; i < rings; i += 1) {
        const c = document.createElementNS(SVG, 'circle');
        c.setAttribute('cx', cx);
        c.setAttribute('cy', cy);
        c.setAttribute('r', R(radius(i)));
        c.setAttribute('class', 'ring');
        wires.append(c);
      }
    }
    for (const n of nodes) {
      // The chart draws one route into each fight (Sam: "just one line per
      // layer between nodes"): from the requirement in the row directly
      // above, the nearest across if there are several. Every fight has one
      // there (the_chart_has_a_route_into_every_fight_from_the_row_above).
      // Hovering still lists all it asks for. The tree draws every line.
      let shown = n.requires;
      if (layout === 'sunburst') shown = n.requires.filter((r) => r.from === tree.sunburst.parent.get(n.id));
      if (layout === 'chart' && n.requires.length > 1) {
        const cx = (id) => { const r = buttons.get(id).getBoundingClientRect(); return r.left + r.width / 2; };
        const above = n.requires.filter((r) => byId.get(r.from).row === n.row - 1);
        const pool = above.length ? above : n.requires;
        const here = cx(n.id);
        shown = [pool.reduce((best, r) => Math.abs(cx(r.from) - here) < Math.abs(cx(best.from) - here) ? r : best)];
      }
      for (const r of shown) {
        const a = buttons.get(r.from).getBoundingClientRect();
        const c = buttons.get(n.id).getBoundingClientRect();
        let d;
        if (layout === 'sunburst') {
          // Dot to dot, as on the chart.
          const x1 = a.left - box.left + a.width / 2, y1 = a.top - box.top + 12;
          const x2 = c.left - box.left + c.width / 2, y2 = c.top - box.top + 12;
          d = `M ${x1} ${y1} L ${x2} ${y2}`;
        } else {
          const x1 = a.left - box.left + a.width / 2, y1 = a.bottom - box.top;
          const x2 = c.left - box.left + c.width / 2, y2 = c.top - box.top;
          const k = (y2 - y1) / 2;
          d = `M ${x1} ${y1} C ${x1} ${y1 + k}, ${x2} ${y2 - k}, ${x2} ${y2}`;
        }
        const line = document.createElementNS(SVG, 'path');
        line.setAttribute('d', d);
        line.setAttribute('class', r.met ? 'met' : 'unmet');
        // A wider, invisible twin is what the pointer finds.
        const hit = document.createElementNS(SVG, 'path');
        hit.setAttribute('d', d);
        hit.setAttribute('class', 'hit');
        hit.addEventListener('mouseenter', (e) => { light([line]); showTip([r.line()], e.clientX, e.clientY); });
        hit.addEventListener('mousemove', (e) => moveTip(e.clientX, e.clientY));
        hit.addEventListener('mouseleave', unhover);
        wires.append(line, hit);
        paths.push({ from: r.from, to: n.id, line });
      }
    }
  }
  function light(lines) {
    for (const p of paths) p.line.classList.toggle('hot', lines.includes(p.line));
  }
  function showTip(kids, x, y) {
    tip.replaceChildren(...kids);
    tip.hidden = false;
    moveTip(x, y);
  }
  function moveTip(x, y) {
    const w = tip.offsetWidth, h = tip.offsetHeight;
    tip.style.left = `${Math.max(8, Math.min(x + 14, innerWidth - w - 8))}px`;
    tip.style.top = `${y + 18 + h > innerHeight ? y - h - 10 : y + 18}px`;
  }
  function hoverNode(id) {
    const n = byId.get(id);
    light(paths.filter((p) => p.to === id).map((p) => p.line));
    if (n.state !== 'locked') { tip.hidden = true; return; }
    const r = buttons.get(id).getBoundingClientRect();
    // Beside the node, not over the row below it.
    showTip([say(n.lockedKey), ...n.requires.map((q) => {
      const e = q.line();
      e.classList.add(q.met ? 'met' : 'unmet');
      return e;
    })], r.right - 6, r.top - 18);
  }
  function unhover() {
    light([]);
    tip.hidden = true;
  }
  function pick(id) {
    for (const [k, b] of buttons) b.classList.toggle('picked', k === id);
    detail(id, card);
  }
  // The chart scrolls sideways in its own frame, opened on the fight to
  // take next.
  const holder = layout === 'chart' ? el('div', { class: 'chart-scroll' }, tree) : tree;
  show(...before, card, holder, tip, button('menu.back.label', menu));
  if (layout === 'chart') {
    const next = tree.querySelector('.next-up');
    if (next) holder.scrollLeft = Math.max(0, next.offsetLeft - holder.clientWidth / 2);
  }
  pick(first);
  wire();
  new ResizeObserver(() => { if (tree.isConnected) wire(); }).observe(tree);
}

// Weapon Master's equipment: what you carry, and what the road has yet to
// give you.
function weaponPanel(redraw) {
  const state = weaponState();
  const cards = state.weapons.map((w) => {
    const name = t(`weapons.${w.id}.name`);
    const kids = [
      el('h4', { 'data-copy': `weapons.${w.id}.name` }, name),
      say(`weapons.${w.id}.desc`, {}, { class: 'desc' }),
    ];
    // The cursed blade's box: liquid that gathers with each fight won, until
    // the village deity falls and it sets into the blade (Sam, 2026-10-07).
    if (w.prize) kids.push(cursedBox(state.prize));
    if (w.carried) kids.push(say('road.carried', { weapon: name }));
    else if (w.unlocked) {
      kids.push(button('road.carry.label', () => {
        SAVE = JSON.parse(save_choose_weapon(JSON.stringify(SAVE), w.id));
        persist();
        redraw();
      }, { weapon: name }));
    } else if (w.unlock) {
      // A rumor says who carries it: one the player came with, or one a
      // beaten villager told.
      kids.push(w.rumor && w.rumor.from ? say('road.rumor.from', { opponent_mid: t(`opponents.${w.rumor.from}.name_mid`) }) : say('road.rumor.start'),
        reqLine(w.unlock, { class: w.unlock.met ? 'met' : 'unmet' }));
    } else {
      // Core leaves out how the cursed blade is won until it is found.
      kids.push(say('road.weapon_unfound', { weapon: name }));
    }
    return el('div', { class: `weapon ${w.carried ? 'carried' : w.unlocked ? 'open' : 'locked'}`, 'data-weapon': w.id }, ...kids);
  });
  // Four arms, won at the final fight: two of the carried weapon. Not shown
  // until it is won (Sam, 2026-10-06: "4 arm should be hidden until you
  // defeat the deity").
  const arms = JSON.parse(arms_json(JSON.stringify(SAVE)));
  if (arms.stop && arms.open) {
    const n = { arms: arms.arms };
    const kids = [el('h4', { 'data-copy': 'road.arms.name' }, t('road.arms.name', n)), say('road.arms.desc', {}, { class: 'desc' })];
    const choose = (four) => () => {
      SAVE = JSON.parse(save_choose_arms(JSON.stringify(SAVE), four));
      persist();
      redraw();
    };
    if (arms.on) kids.push(say('road.arms.on', n), button('road.arms.two.label', choose(false)));
    else if (arms.open) kids.push(button('road.arms.four.label', choose(true), n));
    else kids.push(say('road.weapon_locked'), reqLine({ stop: arms.stop, key: 'road.req.beat', vars: {} }, { class: 'unmet' }));
    cards.push(el('div', { class: `weapon ${arms.on ? 'carried' : arms.open ? 'open' : 'locked'}`, 'data-weapon': 'four_arms' }, ...kids));
  }
  return el('section', { id: 'weapons' }, el('h3', { 'data-copy': 'road.weapon_heading' }, t('road.weapon_heading')), el('div', { class: 'weapon-list' }, ...cards));
}

// The cursed blade's box. Core steps the liquid (content::slosh); the page
// draws it, tips it a little with the pointer, and remembers in this
// browser how full it was last seen, so a win is seen pouring in. Found, it
// sets into the blade, once, and then shows the blade.
function cursedBox(prize) {
  const cv = el('canvas', { class: 'cursed-box', width: 480, height: 200, 'aria-hidden': 'true' });
  const ctx = cv.getContext('2d');
  const target = prize.total ? prize.won / prize.total : 0;
  let seen = 0, setSeen = false;
  try { seen = Number(localStorage.getItem('vagrancy.cursedLevel') || 0); setSeen = localStorage.getItem('vagrancy.cursedSet') === '1'; } catch { /* storage off */ }
  let state = JSON.stringify({ level: Math.min(seen, target), h: [], v: [] });
  let push = 0, t0 = performance.now(), setAt = null;
  cv.addEventListener('pointermove', (e) => { push += e.movementX * 0.00025; });
  const W = cv.width, H = cv.height, pad = 14;
  const blade = (alpha) => {
    ctx.globalAlpha = alpha;
    // The hilt, then the two strands from the guard to the point.
    ctx.fillStyle = PALETTE.hilt;
    ctx.fillRect(pad + 6, H / 2 - 7, 56, 14);
    ctx.fillRect(pad + 58, H / 2 - 26, 9, 52);
    cursedStrands(ctx, [pad + 70, H / 2], [W - pad - 10, H / 2], 20, PALETTE, 4);
    ctx.globalAlpha = 1;
  };
  const frame = (now) => {
    if (!cv.isConnected) return;
    requestAnimationFrame(frame);
    const sway = Math.sin((now - t0) / 900) * 0.0004;
    const fill = prize.found ? 1 : target;
    const out = JSON.parse(slosh_step(state, fill, push + sway));
    state = JSON.stringify(out);
    push *= 0.9;
    ctx.clearRect(0, 0, W, H);
    // The blade it will become, faint, until it is found.
    if (!prize.found) blade(0.14);
    // Found and not yet seen set: the liquid fills, then gives way to the
    // blade over a second and a half.
    let liquid = 1;
    if (prize.found) {
      if (setSeen) { blade(1); return; }
      if (setAt === null && out.level > 0.97) setAt = now;
      if (setAt !== null) {
        const k = Math.min(1, (now - setAt) / 1500);
        liquid = 1 - k;
        blade(k);
        if (k >= 1) { setSeen = true; try { localStorage.setItem('vagrancy.cursedSet', '1'); } catch { /* storage off */ } }
      }
    }
    const n = out.h.length, top = (i) => H - pad - (H - 2 * pad) * Math.min(1, Math.max(0, out.level + out.h[i]));
    ctx.globalAlpha = 0.92 * liquid;
    ctx.fillStyle = PALETTE.cursed.liquid;
    ctx.beginPath();
    ctx.moveTo(pad, H - pad);
    for (let i = 0; i < n; i += 1) ctx.lineTo(pad + ((W - 2 * pad) * i) / (n - 1), top(i));
    ctx.lineTo(W - pad, H - pad);
    ctx.closePath();
    ctx.fill();
    ctx.strokeStyle = PALETTE.cursed.liquid_light;
    ctx.lineWidth = 2;
    ctx.beginPath();
    for (let i = 0; i < n; i += 1) { const x = pad + ((W - 2 * pad) * i) / (n - 1); if (i) ctx.lineTo(x, top(i)); else ctx.moveTo(x, top(i)); }
    ctx.stroke();
    ctx.globalAlpha = 1;
    // The box.
    ctx.strokeStyle = PALETTE.line;
    ctx.lineWidth = 2;
    ctx.strokeRect(pad, pad, W - 2 * pad, H - 2 * pad);
  };
  try { localStorage.setItem('vagrancy.cursedLevel', String(target)); } catch { /* storage off */ }
  requestAnimationFrame(frame);
  return cv;
}

// Which design the road is drawn in, remembered in this browser only.
const ROAD_VIEWS = ['tree', 'chart', 'chapters', 'sunburst'];
function roadView() {
  try { const v = localStorage.getItem('vagrancy.roadView'); if (ROAD_VIEWS.includes(v)) return v; } catch { /* storage off */ }
  return 'chart';
}
function setRoadView(v) {
  try { localStorage.setItem('vagrancy.roadView', v); } catch { /* storage off */ }
}

// The chart is the road's view (Sam, 2026-10-07: "the chart should be the
// default view, the views for the other ones should be somewhat hidden"):
// the others sit folded away at the foot of the screen.
function viewSwitch() {
  const now = roadView();
  const box = el('div', { id: 'road-views', class: 'actions', role: 'group' }, ...ROAD_VIEWS.map((v) => {
    const b = button(`road.view.${v}.label`, () => {
      try { localStorage.setItem('vagrancy.roadView', v); } catch { /* storage off */ }
      road();
    });
    b.setAttribute('aria-pressed', String(v === now));
    b.dataset.view = v;
    return b;
  }));
  return el('details', { id: 'road-views-box', open: now !== 'chart' },
    el('summary', { 'data-copy': 'road.view.other' }, t('road.view.other')), box);
}

function road() {
  stop();
  const stops = roadData();
  const byId = new Map(stops.map((s) => [s.id, s]));
  const nodes = stops.map((s) => ({
    id: s.id,
    row: s.level,
    name: `opponents.${s.id}.name`,
    state: s.flawless ? 'flawless' : s.won ? 'won' : s.open ? 'open' : 'locked',
    reward: s.rewards.length > 0,
    lockedKey: 'road.locked',
    requires: s.requires.map((r) => ({ from: r.stop, met: r.met, line: () => reqLine(r) })),
  }));
  // The deepest fight that is open and not yet won, set apart on the chart
  // with the arcade button's flair (Sam, 2026-10-07): how far down the
  // player has come.
  const ready = stops.filter((s) => s.open && !s.won);
  const deepest = ready.length ? Math.max(...ready.map((s) => s.level)) : null;
  const nextUp = ready.find((s) => s.level === deepest);
  if (nextUp) nodes.find((n) => n.id === nextUp.id).next = true;
  const view = roadView();
  // A row a fight sets for itself (the deities') is not a count of its
  // requirements, so it goes by its region in every view.
  const named = new Set(stops.filter((s) => s.named_row).map((s) => s.level));
  const rowLabel = view === 'chart'
    ? (l) => say(`road.region.${l}`, {}, { class: 'tier-label' })
    : (l) => named.has(l) ? say(`road.region.${l}`, {}, { class: 'tier-label' })
      : l === 0 ? say('road.tier.none', {}, { class: 'tier-label' })
      : l === 1 ? say('road.tier.one', {}, { class: 'tier-label' })
      : say('road.tier.many', { count: l }, { class: 'tier-label' });
  const detail = (id, card) => {
    ROAD_PICK = id;
    const s = byId.get(id);
    const o = (k) => `opponents.${id}.${k}`;
    const mid = { opponent_mid: t(o('name_mid')) };
    const kids = [
      el('h3', { 'data-copy': o('name') }, t(o('name'))),
      say(o('place'), {}, { class: 'desc' }),
    ];
    if (s.weapon) kids.push(say('road.carries', { opponent: t(o('name')), weapon: t(`weapons.${s.weapon}.name`) }));
    if (s.companion) {
      kids.push(say('road.companion', { companion: t(`opponents.${s.companion.pilot}.name`), weapon: t(`weapons.${s.companion.weapon}.name`) }));
    }
    if (s.condition) kids.push(el('h4', { 'data-copy': 'road.condition_heading' }, t('road.condition_heading')), say(s.condition));
    if (s.map) kids.push(el('h4', { 'data-copy': 'road.map_heading' }, t('road.map_heading')), say(`maps.${s.map}.desc`, keyVars(BINDINGS.solo)));
    for (const r of s.rewards) {
      kids.push(say('road.reward', { weapon: t(`weapons.${r.weapon}.name`) }), reqLine(r, { class: r.met ? 'met' : 'unmet' }));
    }
    kids.push(
      el('h4', { 'data-copy': 'road.does_heading' }, t('road.does_heading')), say(o('does'), s.numbers),
      el('h4', { 'data-copy': 'road.try_heading' }, t('road.try_heading')), say(o('try'), s.numbers));
    if (s.open) {
      if (s.flawless) kids.push(say('road.flawless', mid, { class: 'desc' }));
      else if (s.won) kids.push(say('road.cleared', mid, { class: 'desc' }));
      kids.push(el('div', { class: 'actions' }, button('road.fight.label', () => fight(id), mid)));
    } else {
      kids.push(say('road.locked'), el('ul', { class: 'reqs' },
        ...s.requires.map((q) => el('li', { class: q.met ? 'met' : 'unmet' }, reqLine(q), q.met ? say('road.req.met', {}, { class: 'desc' }) : null))));
    }
    card.replaceChildren(...kids);
  };
  // The fight picked last, or the first open one not yet won.
  const first = (ROAD_PICK && byId.get(ROAD_PICK)) || stops.find((s) => s.open && !s.won) || stops[0];
  const before = [say('road.quest', {}, { class: 'desc', id: 'road-quest' }), weaponPanel(road)];
  if (view === 'chapters') chaptersScreen({ stops, detail, first: first.id, before });
  else mapScreen({ nodes, rowLabel, detail, first: first.id, attr: 'data-stop', before, layout: view === 'chart' || view === 'sunburst' ? view : 'rows' });
  $('screen').append(viewSwitch());
}

// Weapon Master's chapter select: a chapter for each row of the tree, and
// the chosen chapter's fights as cards that say what each asks for.
let CHAPTER_PICK = null;
function chaptersScreen({ stops, detail, first, before }) {
  const card = el('section', { id: 'stop-detail' });
  const levels = [...new Set(stops.map((s) => s.level))].sort((a, b) => a - b);
  const firstStop = stops.find((s) => s.id === first);
  if (CHAPTER_PICK === null || !levels.includes(CHAPTER_PICK)) CHAPTER_PICK = firstStop.level;
  const list = el('ol', { id: 'chapters' }, ...levels.map((l) => {
    const here = stops.filter((s) => s.level === l);
    const won = here.filter((s) => s.won).length;
    const open = here.some((s) => s.open);
    const b = button('road.chapter', () => { CHAPTER_PICK = l; road(); }, { n: l + 1, region: t(`road.region.${l}`) });
    b.classList.toggle('picked', l === CHAPTER_PICK);
    if (!open) b.classList.add('locked');
    b.dataset.chapter = String(l);
    return el('li', {}, b, say('road.chapter_progress', { won, count: here.length }, { class: 'desc' }));
  }));
  const cards = stops.filter((s) => s.level === CHAPTER_PICK).map((s) => {
    const state = s.flawless ? 'flawless' : s.won ? 'won' : s.open ? 'open' : 'locked';
    const b = el('button', { type: 'button', class: `stage ${state}${s.rewards.length ? ' reward' : ''}`, 'data-stop': s.id,
      on: { click: () => { for (const x of stage.querySelectorAll('.stage')) x.classList.toggle('picked', x === b); detail(s.id, card); } } },
    el('strong', { 'data-copy': `opponents.${s.id}.name` }, t(`opponents.${s.id}.name`)),
    // Requirements under a fight still locked, each struck through once met;
    // an open or won fight shows none.
    ...(state === 'locked' ? s.requires.map((r) => reqLine(r, { class: r.met ? 'met' : 'unmet' })) : []));
    return b;
  });
  const stage = el('div', { id: 'chapter-stages' }, ...cards);
  show(...before, el('div', { id: 'chapter-view' }, list, stage), card, button('menu.back.label', menu));
  const pickId = stops.some((s) => s.id === first && s.level === CHAPTER_PICK) ? first : stops.find((s) => s.level === CHAPTER_PICK).id;
  const pb = stage.querySelector(`[data-stop="${pickId}"]`);
  if (pb) pb.classList.add('picked');
  detail(pickId, card);
}

// --- the tutorial: a map of missions in the order the knowledge-component
// graph teaches (analysis/kc/RESULTS.md) -------------------------------------------

let MISSION_PICK = null;

function tutorialData() {
  return JSON.parse(tutorial_json(JSON.stringify(SAVE)));
}

// A mission's name: what it teaches first, or for a comparison its two
// opponents.
function missionName(m) {
  if (m.name) return { key: m.name, vars: {} };
  if (m.tasks.length === 2) {
    return { key: 'tutorial.compare_name', vars: { first: t(`opponents.${m.tasks[0].at}.name`), second: t(`opponents.${m.tasks[1].at}.name`) } };
  }
  return { key: `kc.${m.teaches[0]}.name`, vars: {} };
}

function sayMission(m, attrs = {}) {
  const n = missionName(m);
  return say(n.key, n.vars, attrs);
}

function taskLine(task) {
  const where = task.at === 'yard' ? say('tutorial.in_yard') : say('tutorial.against', { opponent_mid: t(`opponents.${task.at}.name_mid`) });
  return [where, say(task.goal, task.vars)];
}

function tutorial() {
  stop();
  const ms = tutorialData();
  const byId = new Map(ms.map((m) => [m.id, m]));
  const nodes = ms.map((m) => {
    const n = missionName(m);
    return {
      id: m.id,
      row: m.row,
      name: n.key,
      nameVars: n.vars,
      state: m.done ? 'won' : m.open ? 'open' : 'locked',
      lockedKey: 'tutorial.locked',
      requires: m.requires.map((r) => ({ from: r.id, met: r.done, line: () => sayMission(byId.get(r.id)) })),
    };
  });
  const rowLabel = (l) => {
    // A row is named by the chapter most of its missions belong to.
    const count = {};
    for (const m of ms) if (m.row === l) count[m.chapter] = (count[m.chapter] || 0) + 1;
    const ch = Object.entries(count).sort((a, b) => b[1] - a[1])[0][0];
    return say(`tutorial.chapter.${ch}`, {}, { class: 'tier-label' });
  };
  const detail = (id, card) => {
    MISSION_PICK = id;
    const m = byId.get(id);
    const n = missionName(m);
    const kids = [el('h3', { 'data-copy': n.key }, t(n.key, n.vars))];
    for (const k of m.teaches) {
      if (m.teaches.length > 1 && `kc.${k}.name` !== n.key) kids.push(el('h4', { 'data-copy': `kc.${k}.name` }, t(`kc.${k}.name`)));
      kids.push(
        el('h4', { 'data-copy': 'tutorial.when_heading' }, t('tutorial.when_heading')), say(`kc.${k}.when`),
        el('h4', { 'data-copy': 'tutorial.then_heading' }, t('tutorial.then_heading')), say(`kc.${k}.then`));
    }
    if (m.builds_on.length) {
      kids.push(el('h4', { 'data-copy': 'tutorial.builds_heading' }, t('tutorial.builds_heading')),
        ...m.builds_on.map((e) => say(`kc_edge.${e}`)));
    }
    if (m.tasks.length === 2) {
      kids.push(el('h4', { 'data-copy': 'tutorial.compare_heading' }, t('tutorial.compare_heading')), say(`kc_compare.${m.id}`));
    }
    kids.push(el('h4', { 'data-copy': 'tutorial.task_heading' }, t('tutorial.task_heading')));
    m.tasks.forEach((task, i) => {
      if (m.tasks.length > 1) kids.push(say('tutorial.part', { n: i + 1, count: m.tasks.length }, { class: 'desc' }));
      kids.push(...taskLine(task));
      if (task.done) kids.push(say('tutorial.done', {}, { class: 'desc' }));
    });
    if (m.open) {
      const next = m.tasks.findIndex((x) => !x.done);
      kids.push(el('div', { class: 'actions' }, button('tutorial.start.label', () => mission(id, next < 0 ? 0 : next))));
    } else {
      kids.push(say('tutorial.locked'), el('ul', { class: 'reqs' },
        ...m.requires.map((r) => el('li', { class: r.done ? 'met' : 'unmet' }, sayMission(byId.get(r.id)), r.done ? say('tutorial.done', {}, { class: 'desc' }) : null))));
    }
    card.replaceChildren(...kids);
  };
  const first = (MISSION_PICK && byId.get(MISSION_PICK)) || ms.find((m) => m.open && !m.done) || ms[0];
  mapScreen({ nodes, rowLabel, detail, first: first.id, attr: 'data-mission' });
}

function mission(id, part) {
  READY = false;
  MISSION_PICK = id;
  const ms = tutorialData();
  const m = ms.find((x) => x.id === id);
  const task = m.tasks[part];
  let finished = false;
  let opened = [];
  const status = el('div', { id: 'mission-status', role: 'status' });
  const goal = el('div', { id: 'mission-goal' }, ...taskLine(task));
  const ended = () => {
    // Done: what opened, then the next task of a comparison or the map.
    const nextPart = m.tasks.findIndex((x, i) => i !== part && !x.done);
    const buttons = [];
    if (nextPart >= 0) buttons.push(button('tutorial.next.label', () => mission(id, nextPart)));
    buttons.push(button('tutorial.to_map.label', tutorial));
    status.replaceChildren(say('tutorial.task_done'),
      ...opened.map((o) => say('tutorial.opened', { mission: t(missionName(ms.find((x) => x.id === o)).key, missionName(ms.find((x) => x.id === o)).vars) }, { class: 'desc' })),
      el('div', { class: 'actions' }, ...buttons));
    const b = status.querySelector('button');
    if (b) b.focus({ preventScroll: true });
  };
  // A fight has rounds and a result; the yard has neither.
  const watch = task.at === 'yard' ? null : matchWatcher(task.at, () => [el('div', { class: 'actions' },
    button('tutorial.again.label', () => mission(id, part)),
    button('tutorial.to_map.label', tutorial))]);
  show(status, goal, ...(watch ? [watch.panel] : []), keysLine(BINDINGS.solo),
    el('div', { class: 'actions' }, button('tutorial.again.label', () => mission(id, part)), button('tutorial.to_map.label', tutorial)));
  const g = new Mission(seed(), tuning(), id, part);
  start(g, withReady(() => [bits(BINDINGS.solo, ACTION_BITS), 0]), (f) => {
    if (!finished && game && game.met && game.met()) {
      finished = true;
      const r = JSON.parse(game.record(JSON.stringify(SAVE)));
      SAVE = JSON.parse(r.save);
      opened = r.opened;
      persist();
      ended();
      document.body.dataset.mission = 'done';
    }
    if (watch) watch.tick(f);
  });
  delete document.body.dataset.mission;
}

// Going on to a goal's fight: a weapon challenge switches the player to its
// weapon first (Sam, 2026-10-06: "the player should be immediately switched
// to that weapon as they press enter"). Core chose the goal so that its
// weapon is one the player has unlocked, and save_choose_weapon refuses any
// other.
function fightForGoal(goal) {
  if (goal.vars && goal.vars.weapon && goal.key === 'road.req.with') {
    SAVE = JSON.parse(save_choose_weapon(JSON.stringify(SAVE), goal.vars.weapon));
    persist();
  }
  fight(goal.stop, goal);
}

// The goal a fight is played for, beside the arena in bold: what it opens
// and what it asks (Sam: "extra bold side text telling the player to meet
// the condition").
function goalCallout(goal) {
  const kids = [say('road.next', { target_mid: t(`opponents.${goal.open}.name_mid`) }), reqLine(goal)];
  if (goal.key === 'road.req.with') kids.push(say('road.carried', { weapon: t(`weapons.${goal.vars.weapon}.name`) }));
  return el('div', { id: 'goal-callout', role: 'note' }, ...kids);
}

function fight(id, goal = null) {
  READY = false;
  ROAD_PICK = id;
  let won = false;
  let opened = [];
  let next = null;
  let revealed = [];
  let cardShown = false;
  const watch = matchWatcher(id, () => {
    // After a win that opened fights, the first button is the first of
    // them; after one that opened nothing, the fight of the next goal;
    // otherwise this opponent again. Enter presses the first.
    const again = button('results.again.label', () => fight(id));
    const to = opened.length ? opened[0] : next ? next.stop : null;
    const go = opened.length ? () => fight(to) : () => fightForGoal(next);
    const onward = to ? button('road.fight.label', go, { opponent_mid: t(`opponents.${to}.name_mid`) }) : null;
    return [
      ...opened.map((o) => say('road.opened', { opponent: t(`opponents.${o}.name`) }, { class: 'desc' })),
      el('div', { class: 'actions' },
        onward, again,
        button('results.to_road.label', road),
        button('results.replay.label', () => download(game.replay_bytes(), 'vagrancy.replay')))];
  });
  show(goal ? goalCallout(goal) : '', watch.panel, keysLine(BINDINGS.solo), el('div', { class: 'actions' }, button('results.to_road.label', road)));
  // With ?player=<seed>, the agent flies the player's seat (content::agent,
  // for Sam's video): each fight's seed is that number plus the fights it
  // has played, and its level is that count, so a run plays as `lab
  // agent-run <seed>` printed it. The page sends it only the ready bit.
  const g = AGENT === null ? new Road(seed(), tuning(), id, SAVE.state.weapon, !!SAVE.state.four_arms)
    : Road.with_agent(AGENT + AGENT_PLAYED, tuning(), id, SAVE.state.weapon, !!SAVE.state.four_arms, AGENT_PLAYED);
  if (AGENT !== null) AGENT_PLAYED += 1;
  start(g, withReady(() => [AGENT === null ? bits(BINDINGS.solo, ACTION_BITS) : 0, 0]), (f) => {
    // Kept before the result is drawn, so the result can name what opened.
    if (!won && game && game.won && game.won()) {
      won = true;
      const r = JSON.parse(game.record(JSON.stringify(SAVE)));
      SAVE = JSON.parse(r.save);
      opened = r.opened;
      next = r.next;
      revealed = r.revealed || [];
      persist();
    }
    watch.tick(f);
    // Once the last round's card has been read (and a headshot's hold is
    // over), the match's own card takes its place (Sam: "when you win a
    // fight, it should pop up in a box like the other boxes").
    if (won && f.phase === 'match_over' && !cardShown) {
      cardShown = true;
      const mine = game;
      setTimeout(() => { if (game === mine) popupMatchWon(opened, next, revealed); }, ENDING ? holdMs() + 700 : 1600);
    }
  });
}

// --- story mode (Sam, 2026-10-06; analysis/story/PROPOSAL.md) ---------------------

function storyMode() {
  stop();
  const st = JSON.parse(story_json());
  const done = SAVE.state.story || 0;
  const chapters = st.chapters.map((c, i) => {
    const n = { n: i + 1, region: t(`road.region.${c.region}`) };
    const kids = [
      el('h3', { 'data-copy': 'road.chapter' }, t('road.chapter', n)),
      say(`story.chapter_intro.${c.region}`, {}, { class: 'desc' }),
      el('ul', { class: 'scenes' }, ...c.scenes.map((sc) => el('li', {}, say(`story.scene.${sc.id}`, { fights: sc.fights.length })))),
    ];
    if (i < done) kids.push(say('story.done', {}, { class: 'desc' }));
    if (i <= done) kids.push(el('div', { class: 'actions' }, button('story.start.label', () => storyPlay(i), { n: i + 1 })));
    else kids.push(say('story.locked', {}, { class: 'desc' }));
    return el('article', { class: `chapter ${i < done ? 'done' : i <= done ? 'open' : 'locked'}`, 'data-chapter': String(i) }, ...kids);
  });
  // The full run, and the extra chapter its best opens (Sam, 2026-10-06).
  const ex = JSON.parse(story_extra_json(JSON.stringify(SAVE)));
  const full = el('section', { id: 'story-full' },
    el('h3', { 'data-copy': 'story.full.heading' }, t('story.full.heading')),
    say('story.full.desc', { lives: ex.lives }, { class: 'desc' }),
    say('story.full.best', { count: ex.best, chapters: ex.chapters }),
    el('div', { class: 'actions' }, button('story.full.start.label', storyPlayFull)));
  const extras = ex.extra.map((x, i) => {
    const kids = [say(`story.scene.${x.id}`, { fights: x.fights.length, rounds: x.rounds })];
    if (x.open) kids.push(el('div', { class: 'actions' }, button('story.extra.start.label', () => storyPlayExtra(i))));
    else kids.push(say('story.extra.needs', { count: x.needs }, { class: 'desc' }));
    return el('article', { class: `chapter ${x.open ? 'open' : 'locked'}`, 'data-extra': String(i) }, ...kids);
  });
  show(say('story.intro', { seconds: st.fight_seconds }, { class: 'desc' }),
    full,
    el('section', { id: 'story-chapters' }, ...chapters),
    el('section', { id: 'story-extra' },
      el('h3', { 'data-copy': 'story.extra.heading' }, t('story.extra.heading')),
      say('story.extra.intro', {}, { class: 'desc' }), ...extras),
    el('div', { class: 'actions' }, button('menu.back.label', menu)));
}

function storyPlay(chapter) {
  storyCard(new StoryRun(seed(), tuning(), chapter, SAVE.state.weapon, !!SAVE.state.four_arms));
}
function storyPlayFull() {
  storyCard(StoryRun.full(seed(), tuning(), SAVE.state.weapon, !!SAVE.state.four_arms));
}
function storyPlayExtra(i) {
  storyCard(StoryRun.extra(seed(), tuning(), i, SAVE.state.weapon, !!SAVE.state.four_arms));
}

// Before each fight: the chapter, and what this scene asks.
function storyCard(run) {
  stop();
  const s = JSON.parse(run.status());
  const extra = s.extra !== null && s.extra !== undefined;
  const heading = extra
    ? el('h3', { 'data-copy': 'story.extra.heading' }, t('story.extra.heading'))
    : el('h3', { 'data-copy': 'road.chapter' }, t('road.chapter', { n: s.chapter + 1, region: t(`road.region.${s.region}`) }));
  show(heading,
    !extra && s.scene === 0 && s.fight === 0 ? say(`story.chapter_intro.${s.region}`, {}, { class: 'desc' }) : '',
    say(`story.scene.${s.scene_id}`, { fights: s.fights, rounds: s.rounds }),
    el('h4', { 'data-copy': `opponents.${s.stop}.name` }, t(`opponents.${s.stop}.name`)),
    say(`opponents.${s.stop}.place`, {}, { class: 'desc' }),
    storyHud(s),
    el('div', { class: 'actions' }, button('story.begin.label', () => storyFight(run)), button('menu.back.label', storyMode)));
  const first = $('screen').querySelector('button');
  if (first) first.focus({ preventScroll: true });
}

function storyHud(s) {
  return s.kind === 'hold'
    ? say('story.hud_hold', { lives: s.lives, seconds: s.seconds_left }, { id: 'story-hud' })
    : say('story.hud', { lives: s.lives, fight: s.fight + 1, fights: s.fights, seconds: s.seconds_left }, { id: 'story-hud' });
}

function storyFight(run) {
  READY = false;
  const s0 = JSON.parse(run.status());
  const watch = matchWatcher(s0.stop, () => []);
  const hud = el('div', { id: 'story-hud-box' }, storyHud(s0));
  show(hud, watch.panel, keysLine(BINDINGS.solo), el('div', { class: 'actions' }, button('menu.back.label', storyMode)));
  let shown = JSON.stringify([s0.lives, s0.seconds_left]);
  let decided = false;
  start(run, withReady(() => [bits(BINDINGS.solo, ACTION_BITS), 0]), (f) => {
    watch.tick(f);
    const s = JSON.parse(run.status());
    const now = JSON.stringify([s.lives, s.seconds_left]);
    if (now !== shown) { shown = now; hud.replaceChildren(storyHud(s)); }
    if (!decided && s.outcome !== 'playing') {
      decided = true;
      document.body.dataset.storyOutcome = s.outcome;
      // Let the last round's card be read, then say how the fight went.
      setTimeout(() => storyAfter(run, s), ENDING ? holdMs() + 900 : 1400);
    }
  });
}

function storyAfter(run, s) {
  if (game !== run) return;
  const next = run.after();
  if (next === 'chapter' || next === 'end' || next === 'game_over') {
    SAVE = JSON.parse(save_read(run.record(JSON.stringify(SAVE))));
    persist();
  }
  const after = JSON.parse(run.status());
  stop();
  // An extra scene ends on its own words; a run, on how far it got.
  const key = after.extra !== null && after.extra !== undefined && (next === 'end' || next === 'game_over') ? `story.extra.${next}` : `story.next.${next}`;
  show(say(`story.${s.outcome}`),
    say(key, { lives: after.lives, region: t(`road.region.${after.region}`), count: run.cleared() }),
    el('div', { class: 'actions' },
      button('story.go_on.label', next === 'end' || next === 'game_over' ? storyMode : () => storyCard(run)),
      button('menu.back.label', storyMode)));
  document.body.dataset.storyNext = next;
  const first = $('screen').querySelector('button');
  if (first) first.focus({ preventScroll: true });
}

// --- behavior trees: the encyclopedia and training (Sam, 2026-10-06) -----------

// An opponent's tree as core describes it, each node's words filled from
// the copy; the same tree the figure in web/trees/ was drawn from.
const TREES = new Map();
function treeOf(id) {
  if (!TREES.has(id)) {
    const fill = (n) => { n.text = t(n.label.key, n.label.vars); n.children.forEach(fill); return n; };
    TREES.set(id, fill(JSON.parse(tree_json(id))));
  }
  return TREES.get(id);
}

// The drawn tree of an opponent, for a card or a tile.
function treeImage(id) {
  return el('img', { class: 'tree-img', src: `trees/${id}.png`, alt: t('encyclopedia.tree_alt', { opponent_mid: t(`opponents.${id}.name_mid`) }) });
}

// What a stop's card says, in arcade mode's words: place, what it does,
// one thing to try, and what it carries, its companion and its ground.
function stopFacts(s) {
  const o = (k) => `opponents.${s.id}.${k}`;
  const kids = [say(o('place'), {}, { class: 'desc' })];
  if (s.weapon) kids.push(say('road.carries', { opponent: t(o('name')), weapon: t(`weapons.${s.weapon}.name`) }));
  if (s.companion) kids.push(say('road.companion', { companion: t(`opponents.${s.companion.pilot}.name`), weapon: t(`weapons.${s.companion.weapon}.name`) }));
  if (s.map) kids.push(say(`maps.${s.map}.desc`, keyVars(BINDINGS.solo)));
  kids.push(
    el('h4', { 'data-copy': 'road.does_heading' }, t('road.does_heading')), say(o('does'), s.numbers),
    el('h4', { 'data-copy': 'road.try_heading' }, t('road.try_heading')), say(o('try'), s.numbers));
  return kids;
}

function encyclopedia() {
  stop();
  const stops = roadData();
  const kinds = ['selector', 'sequence', 'parallel', 'repeat', 'condition', 'action', 'search'];
  const icons = { selector: 'selector', sequence: 'sequence', parallel: 'parallel', repeat: 'repeat', condition: 'far', action: 'approach', search: 'search' };
  const legend = el('ul', { class: 'legend' }, ...kinds.map((k) => el('li', {},
    el('img', { src: `icons/${icons[k]}.png`, alt: '' }), say(`tree.kind_desc.${k}`, {}, { class: 'desc' }))));
  const entries = stops.map((s) => {
    const o = (k) => `opponents.${s.id}.${k}`;
    const kids = [el('h3', { 'data-copy': o('name') }, t(o('name')))];
    if (s.open || s.won) {
      kids.push(...stopFacts(s), el('div', { class: 'tree-scroll' }, treeImage(s.id)));
      if (s.companion) kids.push(el('div', { class: 'tree-scroll' }, treeImage(s.companion.pilot)));
    } else {
      kids.push(say('encyclopedia.locked', {}, { class: 'desc' }),
        el('ul', { class: 'reqs' }, ...s.requires.map((q) => el('li', { class: q.met ? 'met' : 'unmet' }, reqLine(q)))));
    }
    return el('article', { class: `entry ${s.open || s.won ? 'open' : 'locked'}`, 'data-stop': s.id }, ...kids);
  });
  show(
    el('h2', { 'data-copy': 'encyclopedia.heading' }, t('encyclopedia.heading')),
    say('encyclopedia.intro', {}, { class: 'desc' }),
    el('section', { id: 'how-they-decide' },
      el('h3', { 'data-copy': 'encyclopedia.how_heading' }, t('encyclopedia.how_heading')),
      say('encyclopedia.how_tree'), say('encyclopedia.how_search'), say('encyclopedia.how_loop'),
      el('h4', { 'data-copy': 'encyclopedia.legend_heading' }, t('encyclopedia.legend_heading')), legend),
    el('div', { class: 'actions' }, button('menu.back.label', menu)),
    el('section', { id: 'entries' }, ...entries),
    el('div', { class: 'actions' }, button('menu.back.label', menu)),
  );
}

function train() {
  stop();
  const won = roadData().filter((s) => s.won);
  if (!won.length) {
    show(say('train.none'), el('div', { class: 'actions' }, button('menu.back.label', menu)));
    return;
  }
  const card = el('section', { id: 'train-detail' });
  const pick = (s) => {
    const mid = { opponent_mid: t(`opponents.${s.id}.name_mid`) };
    card.replaceChildren(
      el('h3', { 'data-copy': `opponents.${s.id}.name` }, t(`opponents.${s.id}.name`)),
      ...stopFacts(s),
      el('div', { class: 'tree-scroll' }, treeImage(s.id)),
      el('div', { class: 'actions' }, button('train.fight.label', () => trainFight(s), mid)));
    card.scrollIntoView({ block: 'nearest' });
  };
  const tiles = el('div', { id: 'train-tiles' }, ...won.map((s) => el('button', {
    type: 'button', class: 'tile', 'data-stop': s.id, on: { click: () => pick(s) } },
    el('span', { class: 'tile-name', 'data-copy': `opponents.${s.id}.name` }, t(`opponents.${s.id}.name`)),
    treeImage(s.id),
    say(`opponents.${s.id}.does`, s.numbers, { class: 'desc' }))));
  show(say('train.intro', {}, { class: 'desc' }), tiles, card, el('div', { class: 'actions' }, button('menu.back.label', menu)));
  pick(won[0]);
}

// A match against a beaten opponent, with its tree over its head. Nothing
// is recorded: the save is not touched.
function trainFight(s) {
  READY = false;
  const mid = { opponent_mid: t(`opponents.${s.id}.name_mid`) };
  const watch = matchWatcher(s.id, () => [el('div', { class: 'actions' },
    button('train.fight.label', () => trainFight(s), mid),
    button('menu.train.label', train),
    button('results.replay.label', () => download(game.replay_bytes(), 'vagrancy.replay')))]);
  show(watch.panel, say('train.watch', mid, { class: 'desc' }), keysLine(BINDINGS.solo), el('div', { class: 'actions' }, button('menu.train.label', train)));
  const ids = [s.id, s.companion ? s.companion.pilot : null];
  const caption = (n) => t('tree.now', { node: n.text });
  const g = new Road(seed(), tuning(), s.id, SAVE.state.weapon, !!SAVE.state.four_arms);
  start(g, withReady(() => [bits(BINDINGS.solo, ACTION_BITS), 0]), (f) => {
    watch.tick(f);
    const traces = JSON.parse(game.traces());
    LAST_TRACES = traces;
    draw.trees(traces.filter((tr) => ids[tr.seat - 1]).map((tr) => ({ seat: tr.seat, tree: treeOf(ids[tr.seat - 1]), trace: tr, caption })));
  });
}
let LAST_TRACES = [];

// --- watch mode (Sam, 2026-10-06: an enemy against enemy watching mode, like
// the fighting game engines that pit computer fighters against each other) ------------

// Who fights on each side, from the opponents this save has beaten.
const WATCH = [null, null];
// Whether watch mode draws the trees, remembered in this browser only.
function watchTrees() {
  try { return localStorage.getItem('vagrancy.watchTrees') !== 'off'; } catch { return true; }
}
function setWatchTrees(on) {
  try { localStorage.setItem('vagrancy.watchTrees', on ? 'on' : 'off'); } catch { /* storage off */ }
}

function watchMode() {
  stop();
  const won = roadData().filter((s) => s.won);
  if (!won.length) {
    show(say('watch.none'), el('div', { class: 'actions' }, button('menu.back.label', menu)));
    return;
  }
  const ids = won.map((s) => s.id);
  for (let k = 0; k < 2; k += 1) if (!ids.includes(WATCH[k])) WATCH[k] = ids[Math.min(k, ids.length - 1)];
  const picker = (k, labelKey) => {
    const id = `watch-${k}`;
    const pick = el('select', { id, on: { change: () => { WATCH[k] = pick.value; } } },
      ...ids.map((o) => el('option', { value: o, 'data-copy': `opponents.${o}.name` }, t(`opponents.${o}.name`))));
    pick.value = WATCH[k];
    return el('p', {}, el('label', { for: id, 'data-copy': labelKey }, t(labelKey)), ' ', pick);
  };
  show(
    say('watch.intro', {}, { class: 'desc' }),
    picker(0, 'watch.left'),
    picker(1, 'watch.right'),
    mapPicker('watch', 'local.map'),
    el('p', {}, el('label', { for: 'watch-trees', 'data-copy': 'watch.trees' },
      el('input', { type: 'checkbox', id: 'watch-trees', checked: watchTrees() ? '' : null, on: { change: (e) => setWatchTrees(e.target.checked) } }),
      ' ', t('watch.trees'))),
    el('div', { class: 'actions' },
      button('watch.start.label', watchFight),
      button('watch.random.label', () => { watchRandom(ids); watchFight(); }),
      button('menu.back.label', menu)),
  );
}

// Two of the beaten opponents, picked by the page: which pair to show is a
// choice of the menu, not of the match, which core plays.
function watchRandom(ids) {
  WATCH[0] = ids[Math.floor(Math.random() * ids.length)];
  WATCH[1] = ids[Math.floor(Math.random() * ids.length)];
}

function watchFight() {
  READY = false;
  const [left, right] = WATCH;
  const ids = roadData().filter((s) => s.won).map((s) => s.id);
  // The same opponent on both sides goes by the fighters' colors.
  const names = left === right ? null : [t(`opponents.${left}.name`), t(`opponents.${right}.name`)];
  const watch = matchWatcher('', () => [el('div', { class: 'actions' },
    button('watch.again.label', watchFight),
    button('watch.random.label', () => { watchRandom(ids); watchFight(); }),
    button('menu.watch.label', watchMode))], names);
  // Show or hide the trees mid-match; the button names what it will do.
  const toggle = el('div', { class: 'actions' });
  const drawToggle = () => toggle.replaceChildren(button(watchTrees() ? 'watch.hide_trees.label' : 'watch.show_trees.label', () => {
    setWatchTrees(!watchTrees());
    // Off at once, though no tick follows (a match already over).
    if (!watchTrees()) draw.trees([]);
    drawToggle();
  }));
  drawToggle();
  show(watch.panel,
    say('watch.watching', { left_opponent: t(`opponents.${left}.name`), right_opponent: t(`opponents.${right}.name`) }, { class: 'desc' }),
    toggle,
    speedButtons(),
    el('div', { class: 'actions' }, button('watch.random.label', () => { watchRandom(ids); watchFight(); }), button('menu.watch.label', watchMode)));
  const caption = (n) => t('tree.now', { node: n.text });
  const g = new Exhibition(seed(), tuning(), left, right, MAP_CHOICE.watch);
  start(g, () => [0, 0], (f) => {
    watch.tick(f);
    const traces = JSON.parse(game.traces());
    LAST_TRACES = traces;
    draw.trees(watchTrees() ? traces.map((tr) => ({ seat: tr.seat, tree: treeOf(WATCH[tr.seat]), trace: tr, caption })) : []);
  });
  SPEED = WATCH_SPEED;
}

// Watch mode's speed, kept from one match to the next in this visit.
let WATCH_SPEED = 1;
const SPEEDS = [[1, 'watch.speed.full.label'], [0.5, 'watch.speed.half.label'], [0.25, 'watch.speed.quarter.label']];
function speedButtons() {
  const box = el('div', { class: 'actions', id: 'watch-speed', role: 'group' });
  const draw = () => box.replaceChildren(...SPEEDS.map(([v, key]) => {
    const b = button(key, () => { WATCH_SPEED = v; SPEED = v; draw(); });
    b.classList.toggle('picked', WATCH_SPEED === v);
    b.setAttribute('aria-pressed', String(WATCH_SPEED === v));
    return b;
  }));
  draw();
  return box;
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
  // The agent's run starts from nothing when asked to (?player=<seed>&fresh).
  if (AGENT !== null && new URLSearchParams(location.search).has('fresh')) {
    try { for (const k of [AUTOSAVE, 'vagrancy.cursedLevel', 'vagrancy.cursedSet', 'vagrancy.roadView']) localStorage.removeItem(k); } catch (e) { /* storage off */ }
  }
  try { text = localStorage.getItem(AUTOSAVE); } catch (e) { /* storage off */ }
  try {
    SAVE = JSON.parse(save_read(text || save_fresh()));
  } catch (e) {
    SAVE = JSON.parse(save_fresh());
  }
  BINDINGS = SAVE.state.bindings;
}

let SAVE_NOTE = null; // { key, vars } after a load
let RESET_ASKED = false; // the first press of "start over", waiting for the second

// A fresh save that keeps the player's keys and options (Sam, 2026-10-07:
// "a button to reset your active save file, i do it alot for testing").
function resetSave() {
  const fresh = JSON.parse(save_fresh());
  fresh.state.bindings = SAVE.state.bindings;
  fresh.state.options = SAVE.state.options;
  SAVE = JSON.parse(save_read(JSON.stringify(fresh)));
  BINDINGS = SAVE.state.bindings;
  persist();
  // What this browser remembers of the cursed blade's box goes too.
  try { localStorage.removeItem('vagrancy.cursedLevel'); localStorage.removeItem('vagrancy.cursedSet'); } catch { /* storage off */ }
  ROAD_PICK = null;
  SAVE_NOTE = { key: 'settings.save.reset.done', vars: {} };
}

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
      }),
      RESET_ASKED
        ? button('settings.save.reset.confirm.label', () => { RESET_ASKED = false; resetSave(); settings(); }, {}, { id: 'save-reset-confirm' })
        : button('settings.save.reset.label', () => { RESET_ASKED = true; SAVE_NOTE = null; settings(); }, {}, { id: 'save-reset' })),
    RESET_ASKED ? say('settings.save.reset.desc', {}, { class: 'desc', role: 'status' }) : null,
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
    say('online.privacy', {}, { class: 'desc' }),
    onlineWeaponPicker(),
    mapPicker('online', 'online.map'),
    say('online.map_join', {}, { class: 'desc' }),
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
  const sess = isHost ? Online.host(seed(), tuning(), BUILD, ONLINE_WEAPON || SAVE.state.weapon, MAP_CHOICE.online)
    : Online.join(BUILD, ONLINE_WEAPON || SAVE.state.weapon);
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
    net.draw();
  };
  net.draw = paint;
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
    return [say(host ? 'online.connected.host' : 'online.connected.join', { delay_ms: st.delay_ms }),
      ...readyLines(net, st, 'online.ready.label', () => beginOnline(net))];
  }
  return waitingKey ? [say(waitingKey, vars)] : [];
}

// Ready for each player, what the friend has said, and the host's Start
// once both are ready: before the first match, and between matches.
function readyLines(net, st, readyKey, started) {
  const lines = [st.me_ready
    ? say('online.ready.you')
    : button(readyKey, () => { net.sess.ready(); net.flush(); net.paint(true); })];
  lines.push(say(st.them_ready ? 'online.ready.friend' : 'online.ready.friend_not'));
  if (net.sess.seat() === 0 && st.me_ready && st.them_ready) {
    lines.push(button('online.start.label', () => { net.sess.start(performance.now()); net.flush(); started(); }));
  }
  return lines;
}

// After a match, the next one on the same connection (Sam: "dont need to
// refind the game after each set"). The session is back in its lobby; the
// finished match stays on screen until the host starts the next.
function againLines(net) {
  const st = JSON.parse(net.sess.status());
  if (st.kind !== 'connected') return [];
  return readyLines(net, st, 'online.ready.again', () => {});
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
  const again = el('div', { id: 'again' });
  net.draw = () => again.replaceChildren(...againLines(net));
  const watch = matchWatcher('', () => {
    net.paint(true);
    return [again, el('div', { class: 'actions' },
      button('results.replay.label', () => download(sess.replay_bytes(), 'vagrancy.replay')),
      button('menu.back.label', online))];
  });
  show(watch.panel, status, keysLine(BINDINGS.solo), el('div', { class: 'actions' }, button('menu.back.label', online)));
  const delay = JSON.parse(sess.status()).delay_ms;
  let shown = '';
  start(adapter, withReady(() => [bits(BINDINGS.solo, ACTION_BITS), 0]), (f) => {
    watch.tick(f);
    net.paint();
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
  show(musicSection(), youtubeSection(), roundsSection(), keysSection(), saveSection(), button('menu.back.label', menu));
}

// The end of each round: the replay can be turned off, which keeps the
// pause after the deciding cut.
function roundsSection() {
  const box = el('input', { type: 'checkbox', id: 'round-replays', checked: replaysOn() || null, on: { change: (e) => {
    try { localStorage.setItem('vagrancy.roundReplays', e.target.checked ? 'on' : 'off'); } catch { /* storage off */ }
  } } });
  return el('section', { id: 'rounds' },
    el('h2', { 'data-copy': 'settings.rounds.title' }, t('settings.rounds.title')),
    el('p', {}, box, ' ', el('label', { for: 'round-replays', 'data-copy': 'settings.rounds.replay' }, t('settings.rounds.replay'))),
    say('settings.rounds.desc', {}, { class: 'desc' }));
}

let REMEMBER = false;
let MUSIC_ERROR = null;

// A YouTube link the player chooses, in a visible player that outlives the
// screen it was started from.
function youtubeSection() {
  const field = el('input', { type: 'url', id: 'youtube-link', size: '40', value: youtube.remembered() });
  const error = el('div', { id: 'youtube-error' });
  const stopButton = () => button('settings.youtube.stop.label', () => { youtube.stop(); settingsIfShown(); });
  return el('section', { id: 'youtube' },
    el('h2', { 'data-copy': 'settings.youtube.title' }, t('settings.youtube.title')),
    say('settings.youtube.desc'),
    el('p', {}, el('label', { for: 'youtube-link', 'data-copy': 'settings.youtube.field' }, t('settings.youtube.field')), ' ', field),
    error,
    el('div', { class: 'actions' },
      button('settings.youtube.play.label', () => {
        const id = youtube.videoId(field.value);
        if (!id) {
          error.replaceChildren(say('settings.youtube.error', {}, { role: 'alert' }));
          return;
        }
        error.replaceChildren();
        youtube.play(id, stopButton());
      }),
      youtube.playing() ? stopButton() : null),
  );
}

// Redraw Settings if it is what is on screen, so a Stop button there goes.
function settingsIfShown() {
  if (document.querySelector('#screen #youtube')) settings();
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
  draw.bubbleLayout((list, w, h) => JSON.parse(bubbles_step(JSON.stringify(list), w, h)));
  // The arena: the canvas, and the card that comes up over it.
  const stage = $('stage');
  const arena = el('div', { id: 'arena' });
  stage.replaceWith(arena);
  arena.append(stage, el('div', { id: 'death-popup', role: 'status', hidden: '' }));
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
    // The agent's next fight by core's plan, and how many it has played.
    agentNext: () => JSON.parse(agent_next(JSON.stringify(SAVE))),
    agentPlayed: () => AGENT_PLAYED,
    scriptChecksum: (n) => script_checksum(n),
    checksum: () => game && game.checksum(),
    tick: () => game && game.tick(),
    recordedChecksum: () => game && game.recorded_checksum(),
    music: () => music.current(),
    phase: () => curFrame && curFrame.phase,
    edges: () => curFrame && curFrame.swords.map((w) => w.edges.length),
    fighters: () => curFrame && curFrame.fighters.filter(Boolean).length,
    traces: () => LAST_TRACES,
    trees: () => draw.shown(),
    platforms: () => curFrame && (curFrame.platforms || []).length,
    save: () => SAVE,
    online: () => NET && { status: JSON.parse(NET.sess.status()), tick: NET.sess.tick(), checksum: NET.sess.checksum() },
  };
  // Enter goes on: it presses the first button of a result, wherever the
  // focus is.
  window.addEventListener('keydown', (e) => {
    if (e.code !== 'Enter' && e.code !== 'NumpadEnter') return;
    // Between online matches, Enter says ready or starts, and never saves a
    // replay or leaves.
    const again = document.querySelector('#again');
    const first = again ? again.querySelector('button') : document.querySelector('#result button');
    if (first && document.activeElement !== first && !(document.activeElement && document.activeElement.matches('input, textarea'))) {
      e.preventDefault();
      first.click();
    }
  });
  const m = location.hash.match(/^#room=([A-Z0-9]{4,12})$/);
  if (m) joinRoom(m[1]); else menu();
  requestAnimationFrame(loop);
  document.body.dataset.ready = '1';
  // The agent's run for Sam's video: the director works the page.
  if (AGENT !== null) import('./director.js').then((d) => d.direct()).catch((e) => console.error(e));
}

main();
