// Draws a frame core sent. It may interpolate between two frames; it never
// integrates, predicts or detects a contact (D2). Every color comes from
// data/palette.json, through core.

export function renderer(canvas, palette, numbers) {
  const ctx = canvas.getContext('2d');
  const one = 1 << numbers.frac_bits;
  const half = numbers.arena_half / one;
  const W = canvas.width;
  const H = canvas.height;
  const scale = W / (2 * half);
  const ground = H - 40;
  const sx = (x) => (x / one + half) * scale;
  const sy = (y) => ground - (y / one) * scale;

  // The right fighter is striped as well as ochre, so the two are told apart
  // by pattern and not by hue alone (D17).
  const stripes = (() => {
    const c = document.createElement('canvas');
    c.width = c.height = 8;
    const g = c.getContext('2d');
    g.fillStyle = palette.fighters.right.body;
    g.fillRect(0, 0, 8, 8);
    g.strokeStyle = palette.fighters.right.stripe;
    g.lineWidth = 2;
    g.beginPath();
    g.moveTo(-2, 10); g.lineTo(10, -2);
    g.stroke();
    return ctx.createPattern(c, 'repeat');
  })();
  // Colors go by side: on a flanked stop both opponents wear the right
  // fighter's ochre stripes. `sides` is the latest frame's, from core.
  let sides = [0, 1, 1];
  const side = (seat) => sides[seat] ?? (seat === 0 ? 0 : 1);
  const fill = (part) => {
    if (part.body === 1) return palette.post;
    return side(part.fighter) === 0 ? palette.fighters.left.body : stripes;
  };
  const inkColor = (seat) => (side(seat) === 0 ? palette.fighters.left.ink : palette.fighters.right.ink);

  // Where cuts landed this round, as core reported them. They are drawn
  // where they happened and do not move: the page does not integrate.
  let marks = [];

  function lerp(prev, cur, alpha) {
    if (!prev || prev.points.length !== cur.points.length) return cur.points;
    return cur.points.map((p, i) => [
      prev.points[i][0] + (p[0] - prev.points[i][0]) * alpha,
      prev.points[i][1] + (p[1] - prev.points[i][1]) * alpha,
    ]);
  }

  function capsule(a, b, r, style) {
    ctx.strokeStyle = style;
    ctx.lineWidth = Math.max(1, 2 * (r / one) * scale);
    ctx.lineCap = 'round';
    ctx.beginPath();
    ctx.moveTo(sx(a[0]), sy(a[1]));
    ctx.lineTo(sx(b[0]), sy(b[1]));
    ctx.stroke();
  }

  function dot(p, r, style) {
    ctx.fillStyle = style;
    ctx.beginPath();
    ctx.arc(sx(p[0]), sy(p[1]), r, 0, Math.PI * 2);
    ctx.fill();
  }

  function meter(x, y, f, seat, toLeft) {
    const w = 220;
    const h = 10;
    ctx.fillStyle = palette.meter_back;
    ctx.fillRect(x, y, w, h);
    ctx.fillStyle = inkColor(seat);
    const filled = Math.max(0, Math.min(1, f.ink / Math.max(1, f.ink_max))) * w;
    // A meter empties toward the side of the screen its fighter started on.
    ctx.fillRect(toLeft ? x : x + w - filled, y, filled, h);
  }

  // A headshot is looked at: the page holds the clock, and the view closes
  // in on where the blade landed, then opens again. `e` runs 0 → 1 → 0.
  let focus = null;
  draw.focus = (f) => { focus = f; };
  function closeness(now) {
    if (!focus || focus.still) return 0;
    const t = (now - focus.start) / focus.dur;
    if (t <= 0 || t >= 1) return 0;
    const ease = (x) => x * x * (3 - 2 * x);
    return t < 0.2 ? ease(t / 0.2) : t > 0.8 ? ease((1 - t) / 0.2) : 1;
  }

  function draw(prev, cur, alpha) {
    const pts = lerp(prev, cur, alpha);
    sides = cur.fighters.map((f, seat) => (f ? f.side : seat === 0 ? 0 : 1));
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.fillStyle = palette.paper;
    ctx.fillRect(0, 0, W, H);
    const e = closeness(performance.now());
    if (e > 0) {
      // The cut's point moves toward the middle as the view closes in.
      const px = sx(focus.at[0]), py = sy(focus.at[1]);
      const z = 1 + 1.6 * e;
      const tx = px + (W / 2 - px) * e, ty = py + (H / 2 - py) * e;
      ctx.setTransform(z, 0, 0, z, tx - z * px, ty - z * py);
    }
    ctx.fillStyle = palette.ground;
    ctx.fillRect(0, ground, W, H - ground);
    ctx.strokeStyle = palette.ground_line;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, ground + 0.5);
    ctx.lineTo(W, ground + 0.5);
    ctx.stroke();
    // Ledges: a slab of the ground's color with the ground's line on top.
    for (const [x0, x1, y] of cur.platforms || []) {
      const top = sy(y);
      ctx.fillStyle = palette.ground;
      ctx.fillRect(sx(x0), top, sx(x1) - sx(x0), 8);
      ctx.beginPath();
      ctx.moveTo(sx(x0), top + 0.5);
      ctx.lineTo(sx(x1), top + 0.5);
      ctx.stroke();
    }
    // A dodging fighter is drawn see-through for the moment it cannot be cut.
    const dodging = (seat) => cur.fighters[seat] && cur.fighters[seat].dodging;
    for (const part of cur.parts) {
      ctx.globalAlpha = part.attached && dodging(part.fighter) ? 0.4 : 1;
      capsule(pts[part.a], pts[part.b], part.r, fill(part));
    }
    ctx.globalAlpha = 1;
    // A stump's cut end: the paper inside, ringed with that fighter's ink.
    for (const part of cur.parts) {
      if (!part.stump) continue;
      const r = Math.max(2, (part.r / one) * scale);
      dot(pts[part.b], r, inkColor(part.fighter));
      dot(pts[part.b], r * 0.55, palette.paper);
    }
    // Ink where each cut landed, on top: drawn under the bodies, every mark
    // sat inside the part it was cut from and none could be seen.
    for (const m of marks) dot(m.at, 4, inkColor(m.seat));
    for (const s of cur.swords) {
      const a = pts[s.butt];
      const b = pts[s.tip];
      const f = s.hilt / one;
      const h = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
      // Each edge of the weapon (one for a sword; a curve's bends, a fork's
      // prongs), then the hilt over them.
      for (const [i, j] of s.edges) capsule(pts[i], pts[j], one * 1.1, palette.sword);
      capsule(a, h, one * 1.6, palette.hilt);
    }
    if (e > 0) {
      // The edges darken while the view is close.
      // Rings of the line color, each a little further out, so the dark
      // deepens toward the edges (a gradient would need a color the palette
      // does not hold).
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.fillStyle = palette.line;
      for (let k = 0; k < 24; k += 1) {
        ctx.globalAlpha = 0.018 * e;
        ctx.beginPath();
        ctx.rect(0, 0, W, H);
        ctx.arc(W / 2, H / 2, H * (0.42 + k * 0.025), 0, Math.PI * 2, true);
        ctx.fill('evenodd');
      }
      ctx.globalAlpha = 1;
    }
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    // Seat 0's meter at the top left, seat 1's at the top right, and on a
    // flanked stop seat 2's (the opponent on the left) under seat 0's.
    cur.fighters.forEach((f, seat) => {
      if (!f || f.ink_max <= 0) return;
      if (seat === 0) meter(16, 14, f, seat, true);
      else if (seat === 1) meter(W - 236, 14, f, seat, false);
      else meter(16, 30, f, seat, true);
    });
  }

  let lastPhase = 'fight';
  draw.reset = () => { marks = []; lastPhase = 'fight'; };
  draw.events = (frame) => {
    // A new round stands both fighters back up; its marks start clean.
    if (lastPhase !== 'fight' && frame.phase === 'fight') marks = [];
    lastPhase = frame.phase;
    for (const e of frame.events) {
      if (e.Cut && e.Cut.spilled) marks.push({ at: [e.Cut.at.x, e.Cut.at.y], seat: e.Cut.seat });
    }
  };
  return draw;
}
