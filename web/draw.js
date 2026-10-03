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
  const fill = (part) => {
    if (part.body === 1) return palette.post;
    return part.fighter === 0 ? palette.fighters.left.body : stripes;
  };

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

  return function draw(prev, cur, alpha) {
    const pts = lerp(prev, cur, alpha);
    ctx.fillStyle = palette.paper;
    ctx.fillRect(0, 0, W, H);
    ctx.fillStyle = palette.ground;
    ctx.fillRect(0, ground, W, H - ground);
    ctx.strokeStyle = palette.ground_line;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, ground + 0.5);
    ctx.lineTo(W, ground + 0.5);
    ctx.stroke();
    for (const part of cur.parts) capsule(pts[part.a], pts[part.b], part.r, fill(part));
    for (const s of cur.swords) {
      const a = pts[s.butt];
      const b = pts[s.tip];
      const f = s.hilt / one;
      const h = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
      capsule(a, h, one * 1.6, palette.hilt);
      capsule(h, b, one * 1.1, palette.sword);
    }
  };
}
