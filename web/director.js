// The director (Sam, 2026-10-07): for a video of the agent playing arcade
// mode, it works the page as a person would, with a cursor that glides to
// each thing before it clicks: it opens arcade mode, looks over the chart,
// opens a fight's card, reads it, fights, takes the next round, goes back to
// the chart and picks the next fight. Which fight comes next is core's
// (content::agent::next); the fighting is the agent's, in core. Loaded only
// with ?player in the address; it writes no words of its own.

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const $ = (sel) => document.querySelector(sel);
const visible = (el) => !!el && el.offsetParent !== null && !el.disabled;
// A person's pace varies: a pause of about `ms`, give or take a third.
const beat = (ms) => sleep(ms * (0.75 + Math.random() * 0.5));

let cursor = null, at = [960, 540];
function makeCursor() {
  cursor = document.createElement('div');
  cursor.setAttribute('aria-hidden', 'true');
  cursor.style.cssText = 'position:fixed;left:0;top:0;width:28px;height:28px;z-index:99999;pointer-events:none;transition:none;';
  cursor.innerHTML = '<svg width="28" height="28" viewBox="0 0 28 28"><path d="M3 2 L3 22 L8.5 17 L12.5 26 L16 24.5 L12 15.5 L19.5 15.5 Z" fill="var(--paper)" stroke="var(--line)" stroke-width="1.6" stroke-linejoin="round"/></svg>';
  document.body.append(cursor);
  place(at);
}
function place([x, y]) {
  at = [x, y];
  cursor.style.transform = `translate(${x - 3}px, ${y - 2}px)`;
}
// A glide along a gentle arc, quick in the middle and slow at each end.
async function glide(to, ms = 700) {
  const from = at.slice();
  const bend = (Math.random() - 0.5) * 0.3 * Math.hypot(to[0] - from[0], to[1] - from[1]);
  const start = performance.now();
  for (;;) {
    const t = Math.min(1, (performance.now() - start) / ms);
    const e = t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2;
    const nx = -(to[1] - from[1]), ny = to[0] - from[0], nl = Math.hypot(nx, ny) || 1;
    const arc = Math.sin(Math.PI * e) * bend;
    place([from[0] + (to[0] - from[0]) * e + (nx / nl) * arc, from[1] + (to[1] - from[1]) * e + (ny / nl) * arc]);
    if (t >= 1) return;
    await new Promise(requestAnimationFrame);
  }
}
function centre(el) {
  const r = el.getBoundingClientRect();
  return [r.left + r.width * (0.4 + Math.random() * 0.2), r.top + r.height * (0.4 + Math.random() * 0.2)];
}
// Bring it into view the way a reader scrolls: smoothly, and a chart
// fight by scrolling the chart's own frame sideways.
async function reveal(el) {
  const frame = el.closest('.chart-scroll');
  if (frame) {
    const want = Math.max(0, el.offsetLeft - frame.clientWidth / 2);
    frame.scrollTo({ left: want, behavior: 'smooth' });
  }
  const r = el.getBoundingClientRect();
  if (r.top < 80 || r.bottom > innerHeight - 80) el.scrollIntoView({ behavior: 'smooth', block: 'center' });
  await sleep(650);
}
async function hover(el) {
  await reveal(el);
  await glide(centre(el), 500 + Math.random() * 400);
  el.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
  el.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
}
async function click(el) {
  await hover(el);
  await beat(250);
  // A ring where the click lands.
  const ring = document.createElement('div');
  ring.setAttribute('aria-hidden', 'true');
  ring.style.cssText = `position:fixed;left:${at[0] - 14}px;top:${at[1] - 14}px;width:28px;height:28px;border-radius:50%;border:3px solid var(--focus);z-index:99998;pointer-events:none;transition:transform 300ms ease-out, opacity 300ms ease-out;`;
  document.body.append(ring);
  requestAnimationFrame(() => { ring.style.transform = 'scale(1.8)'; ring.style.opacity = '0'; });
  setTimeout(() => ring.remove(), 350);
  el.click();
}
async function until(fn, ms = 600000) {
  const end = performance.now() + ms;
  for (;;) {
    const v = fn();
    if (v) return v;
    if (performance.now() > end) throw new Error('the director waited too long');
    await sleep(120);
  }
}

// A note to the server the page came from, so whoever records the run
// can follow it in the server's log; it fails quietly (the address is not
// a file). Same origin, so nothing leaves the page's own server.
const note = (what) => fetch(`./run-${what}`, { cache: 'no-store' }).catch(() => {});

// Look over the chart: a few fights' cards, as a player reads what is ahead.
async function lookAround(n) {
  const nodes = [...document.querySelectorAll('#road-tree .node')].filter(visible);
  const ahead = nodes.filter((b) => b.classList.contains('open') || b.classList.contains('locked'));
  for (let k = 0; k < n && ahead.length; k += 1) {
    const b = ahead[Math.floor(Math.random() * Math.min(ahead.length, 14))];
    await hover(b);
    await beat(900);
  }
}

async function fight(stop) {
  const node = await until(() => $(`#road-tree [data-stop="${stop}"]`));
  await click(node);
  await beat(2600); // reading the card
  await click(await until(() => [...document.querySelectorAll('[data-copy="road.fight.label"]')].find(visible)));
  // Round by round: once the round's end has played and its button is up,
  // take the next round; at the match's end, read the card and go back.
  for (;;) {
    const next = await until(() => {
      const nr = [...document.querySelectorAll('#result [data-copy="results.next_round.label"]')].find(visible);
      if (nr) return nr;
      if (window.vagrancy.phase() === 'match_over') return 'over';
      return null;
    });
    if (next === 'over') break;
    await beat(1500);
    await click(next);
    await until(() => window.vagrancy.phase() === 'fight', 20000).catch(() => {});
  }
  await beat(5200); // the match's card
  const back = await until(() => [...document.querySelectorAll('#result [data-copy="results.to_road.label"], [data-copy="results.to_road.label"]')].find(visible));
  await click(back);
  await until(() => $('#road-tree .node'));
  await beat(1200);
}

export async function direct() {
  makeCursor();
  await until(() => document.body.dataset.ready === '1');
  await beat(3000);
  await click(await until(() => [...document.querySelectorAll('[data-copy="menu.road.label"]')].find(visible)));
  await until(() => $('#road-tree .node'));
  await beat(1500);
  await lookAround(4);
  for (;;) {
    const { stop } = window.vagrancy.agentNext();
    if (!stop) break;
    if (Math.random() < 0.35) await lookAround(1 + Math.floor(Math.random() * 2));
    await fight(stop);
    note(`fought-${window.vagrancy.agentPlayed()}`);
  }
  // The run is over: a last look along the chart.
  await lookAround(3);
  document.body.dataset.directorDone = '1';
  note('done');
}
