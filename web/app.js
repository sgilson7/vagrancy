// The page. It draws numbers core sent and decides nothing: no physics, no
// contact, no copy of a constant (CLAUDE.md). Every word it shows comes from
// data/copy.en.json, which reaches it through the wasm module; which sentence
// to show for an outcome is chosen by core (content::messages).
import init, {
  copy_json, palette_json, controls_json, numbers as coreNumbers, script_checksum, Game, Online, Road, Mission,
  road_json, tutorial_json, weapons_json, save_choose_weapon, save_fresh, save_read,
} from './pkg/vagrancy_wasm.js';
import * as rtc from './rtc.js';
import { renderer } from './draw.js';
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
    item('menu.tutorial', tutorial),
    item('menu.road', road),
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

function weaponList() {
  return JSON.parse(weapons_json(JSON.stringify(SAVE)));
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
  start(Game.versus(seed(), tuning(), LOCAL_WEAPONS[0], LOCAL_WEAPONS[1]),
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
    keysLine(binding),
    el('ol', { id: 'steps' }, ...steps.map((k) => el('li', {}, say(`practice.step.${k}`, vars)))),
    el('div', { class: 'actions' },
      button('practice.reset.label', practice),
      button('replay.download.label', () => download(game.replay_bytes(), 'vagrancy.replay')),
      button('replay.load.label', loadReplay),
      button('menu.back.label', menu)),
  );
  start(Game.practice(seed(), tuning(), SAVE.state.weapon), () => [bits(binding, ACTION_BITS), 0]);
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
  return say(r.key, { ...r.vars, opponent_mid: t(`opponents.${r.stop}.name_mid`) }, attrs);
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
    const b = el('button', { type: 'button', class: `node ${n.state}${n.reward ? ' reward' : ''}`, [attr]: n.id, 'data-copy': n.name,
      on: {
        click: () => pick(n.id),
        mouseenter: () => hoverNode(n.id),
        focus: () => hoverNode(n.id),
        mouseleave: unhover,
        blur: unhover,
      } }, t(n.name, n.nameVars));
    buttons.set(n.id, b);
    return b;
  };
  if (layout === 'chart') {
    // A chart, after Weapon Master's map: each row a region in its own
    // band, its fights as seals set about it rather than in a line, joined
    // by routes. The scatter is a fixed pattern, so the chart is the same
    // each time it is drawn.
    const BAND = 132;
    tree.style.height = `${rows.length * BAND}px`;
    rows.forEach((row, l) => {
      if (!row) return;
      const band = el('div', { class: `band band-${l % 2}`, 'data-level': String(l) }, rowLabel(l));
      band.style.top = `${l * BAND}px`;
      band.style.height = `${BAND}px`;
      tree.append(band);
      row.forEach((n, i) => {
        const b = nodeButton(n);
        // Kept off the edges, so a seal and its name stay on the chart.
        const x = Math.max(9, Math.min(91, (i + 0.5) / row.length * 100 + Math.sin(l * 1.7 + i * 2.3) * (36 / row.length)));
        const y = l * BAND + 30 + (Math.cos(l * 1.3 + i * 1.9) + 1) * 14;
        b.style.left = `${x}%`;
        b.style.top = `${y}px`;
        tree.append(b);
      });
    });
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
    for (const n of nodes) {
      for (const r of n.requires) {
        const a = buttons.get(r.from).getBoundingClientRect();
        const c = buttons.get(n.id).getBoundingClientRect();
        const x1 = a.left - box.left + a.width / 2, y1 = a.bottom - box.top;
        const x2 = c.left - box.left + c.width / 2, y2 = c.top - box.top;
        const k = (y2 - y1) / 2;
        const d = `M ${x1} ${y1} C ${x1} ${y1 + k}, ${x2} ${y2 - k}, ${x2} ${y2}`;
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
  show(...before, card, tree, tip, button('menu.back.label', menu));
  pick(first);
  wire();
  new ResizeObserver(() => { if (tree.isConnected) wire(); }).observe(tree);
}

// Weapon Master's equipment: what you carry, and what the road has yet to
// give you.
function weaponPanel(redraw) {
  const cards = weaponList().map((w) => {
    const name = t(`weapons.${w.id}.name`);
    const kids = [
      el('h4', { 'data-copy': `weapons.${w.id}.name` }, name),
      say(`weapons.${w.id}.desc`, {}, { class: 'desc' }),
    ];
    if (w.carried) kids.push(say('road.carried', { weapon: name }));
    else if (w.unlocked) {
      kids.push(button('road.carry.label', () => {
        SAVE = JSON.parse(save_choose_weapon(JSON.stringify(SAVE), w.id));
        persist();
        redraw();
      }, { weapon: name }));
    } else {
      kids.push(say('road.weapon_locked'), reqLine(w.unlock, { class: w.unlock.met ? 'met' : 'unmet' }));
    }
    return el('div', { class: `weapon ${w.carried ? 'carried' : w.unlocked ? 'open' : 'locked'}`, 'data-weapon': w.id }, ...kids);
  });
  return el('section', { id: 'weapons' }, el('h3', { 'data-copy': 'road.weapon_heading' }, t('road.weapon_heading')), el('div', { class: 'weapon-list' }, ...cards));
}

// Which design the road is drawn in, remembered in this browser only.
const ROAD_VIEWS = ['tree', 'chart', 'chapters'];
function roadView() {
  try { const v = localStorage.getItem('vagrancy.roadView'); if (ROAD_VIEWS.includes(v)) return v; } catch { /* storage off */ }
  return 'tree';
}

function viewSwitch() {
  const now = roadView();
  return el('div', { id: 'road-views', class: 'actions', role: 'group' }, ...ROAD_VIEWS.map((v) => {
    const b = button(`road.view.${v}.label`, () => {
      try { localStorage.setItem('vagrancy.roadView', v); } catch { /* storage off */ }
      road();
    });
    b.setAttribute('aria-pressed', String(v === now));
    b.dataset.view = v;
    return b;
  }));
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
  const view = roadView();
  const rowLabel = view === 'chart'
    ? (l) => say(`road.region.${l}`, {}, { class: 'tier-label' })
    : (l) => l === 0 ? say('road.tier.none', {}, { class: 'tier-label' })
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
    if (s.condition) kids.push(el('h4', { 'data-copy': 'road.condition_heading' }, t('road.condition_heading')), say(s.condition));
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
  const before = [weaponPanel(road), viewSwitch()];
  if (view === 'chapters') chaptersScreen({ stops, detail, first: first.id, before });
  else mapScreen({ nodes, rowLabel, detail, first: first.id, attr: 'data-stop', before, layout: view === 'chart' ? 'chart' : 'rows' });
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
    ...s.requires.map((r) => reqLine(r, { class: r.met ? 'met' : 'unmet' })));
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

function fight(id) {
  READY = false;
  ROAD_PICK = id;
  let won = false;
  let opened = [];
  const watch = matchWatcher(id, () => {
    // After a win that opened fights, the first button is the first of
    // them; otherwise this opponent again. Enter presses the first.
    const again = button('results.again.label', () => fight(id));
    const onward = opened.length
      ? button('road.fight.label', () => fight(opened[0]), { opponent_mid: t(`opponents.${opened[0]}.name_mid`) })
      : null;
    return [
      ...opened.map((o) => say('road.opened', { opponent: t(`opponents.${o}.name`) }, { class: 'desc' })),
      el('div', { class: 'actions' },
        onward, again,
        button('results.to_road.label', road),
        button('results.replay.label', () => download(game.replay_bytes(), 'vagrancy.replay')))];
  });
  show(watch.panel, keysLine(BINDINGS.solo), el('div', { class: 'actions' }, button('results.to_road.label', road)));
  const g = new Road(seed(), tuning(), id, SAVE.state.weapon);
  start(g, withReady(() => [bits(BINDINGS.solo, ACTION_BITS), 0]), (f) => {
    // Kept before the result is drawn, so the result can name what opened.
    if (!won && game && game.won && game.won()) {
      won = true;
      const r = JSON.parse(game.record(JSON.stringify(SAVE)));
      SAVE = JSON.parse(r.save);
      opened = r.opened;
      persist();
    }
    watch.tick(f);
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
    say('online.privacy', {}, { class: 'desc' }),
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
  const sess = isHost ? Online.host(seed(), tuning(), BUILD, SAVE.state.weapon) : Online.join(BUILD, SAVE.state.weapon);
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
  show(musicSection(), youtubeSection(), keysSection(), saveSection(), button('menu.back.label', menu));
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
    edges: () => curFrame && curFrame.swords.map((w) => w.edges.length),
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
}

main();
