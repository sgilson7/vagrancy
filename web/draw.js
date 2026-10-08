// Draws a frame core sent. It may interpolate between two frames; it never
// integrates, predicts or detects a contact (D2). Every color comes from
// data/palette.json, through core.

// The cursed blade (Sam, 2026-10-07): no steel, two strands twisting about
// each other from the hilt to the point, where they meet. `a` and `b` are
// in pixels, `width` how far the strands swing apart; the strand behind is
// drawn first, so they cross over and under.
export function cursedStrands(ctx, a, b, width, palette, turns = 3) {
  const dx = b[0] - a[0], dy = b[1] - a[1];
  const len = Math.hypot(dx, dy) || 1;
  const nx = -dy / len, ny = dx / len;
  const N = 40;
  const at = (t, sgn) => {
    const amp = width * (1 - t * 0.85) * Math.sin(t * turns * Math.PI * 2) * sgn;
    return [a[0] + dx * t + nx * amp, a[1] + dy * t + ny * amp];
  };
  const strand = (sgn, color, lw) => {
    ctx.strokeStyle = color;
    ctx.lineWidth = lw;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    for (let k = 0; k <= N; k += 1) {
      const p = at(k / N, sgn);
      if (k === 0) ctx.moveTo(p[0], p[1]); else ctx.lineTo(p[0], p[1]);
    }
    ctx.stroke();
  };
  const lw = Math.max(1.5, width * 0.55);
  strand(-1, palette.cursed.strand_b, lw);
  strand(1, palette.cursed.strand_a, lw);
}

export function renderer(canvas, palette, numbers) {
  const ctx = canvas.getContext('2d');
  const one = 1 << numbers.frac_bits;
  const half = numbers.arena_half / one;
  const W = canvas.width;
  const H = canvas.height;
  const scale = W / (2 * half);
  const ground = H - 40;
  // The camera: centred on the arena, or, on a stage wider than a screen,
  // on the player as far as the walls allow. Set at the start of each draw.
  let camX = 0;
  const sx = (x) => (x / one - camX + half) * scale;
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
    if (guest && guest.fills && guest.fills[part.fighter]) return guest.fills[part.fighter];
    if (part.body === 1) return palette.post;
    return side(part.fighter) === 0 ? palette.fighters.left.body : stripes;
  };
  const inkColor = (seat) => (side(seat) === 0 ? palette.fighters.left.ink : palette.fighters.right.ink);

  // Costumes (data/costumes.json, through core's numbers): each picture is
  // drawn in the fighter's rest-pose centimeters and rides on one part, laid
  // on with the turn that takes the part's rest direction to where it points
  // now, about its far end; mirrored for a fighter facing -x. The pictures
  // load once and are skipped until they have.
  const dress = numbers.costumes || { art: false, slots: {}, under: [], costumes: {} };
  const under = new Set(dress.under);
  const pictures = new Map();
  const picture = (id, slot) => {
    const k = `${id}-${slot}`;
    if (!pictures.has(k)) { const im = new Image(); im.src = `art/costume/${k}.png`; pictures.set(k, im); }
    return pictures.get(k);
  };
  // The page can turn the costumes off (Sam, 2026-10-08: "a way to turn
  // off all costumes"): the fighters are then drawn plain and a plate as
  // a band of steel, as with the art switch off.
  let costumesShown = true;
  draw.costumes = (on) => { costumesShown = on !== false; };
  draw.costumesShown = () => costumesShown;
  // A clip's guests (clip.js): who each seat is dressed as, their slots,
  // their scene, and their blades' colors, over the road's.
  let guest = null;
  draw.guests = (g) => { guest = g || null; };
  // What the player's own fighter (seat 0) wears, from the wardrobe (Sam,
  // 2026-10-08): only drawn, so a fight plays the same in any outfit.
  let outfit = null;
  draw.playerOutfit = (id) => { outfit = id || null; };
  const dressedAs = (cur, seat) => (guest && guest.seats ? guest.seats[seat]
    : seat === 0 && outfit ? outfit
    : cur.fighters[seat] && cur.fighters[seat].costume);
  function wearing(cur, seat) {
    const id = dressedAs(cur, seat);
    if (!costumesShown || !dress.art || !id) return null;
    return (guest && guest.costumes && guest.costumes[id]) || dress.costumes[id] || null;
  }
  function wear(cur, pts, seat, slots) {
    const f = cur.fighters[seat];
    const c = wearing(cur, seat);
    for (const name of slots) {
      if (!c.slots.includes(name)) continue;
      const slot = dress.slots[name];
      const part = cur.parts.find((p) => p.fighter === seat && p.def === slot.part && !p.stump);
      const im = picture(dressedAs(cur, seat), name);
      if (!part || !im.complete || !im.naturalWidth) continue;
      const a = [sx(pts[part.a][0]), sy(pts[part.a][1])];
      const b = [sx(pts[part.b][0]), sy(pts[part.b][1])];
      const dx = slot.far[0] - slot.near[0], dy = slot.far[1] - slot.near[1];
      const turn = Math.atan2(b[1] - a[1], b[0] - a[0]) - Math.atan2(-dy, f.facing * dx);
      const [x0, y0, x1, y1] = slot.box;
      ctx.save();
      ctx.translate(b[0], b[1]);
      ctx.rotate(turn);
      ctx.scale(f.facing * scale, scale);
      ctx.drawImage(im, x0 - slot.far[0], slot.far[1] - y1, x1 - x0, y1 - y0);
      ctx.restore();
    }
  }
  // Backgrounds (data/backgrounds.json, through core's numbers): a fight's
  // scene is layers of pictures in the arena's centimeters. Each moves with
  // the camera by its depth, a far one hardly at all, so the scene has
  // depth as the view pans and closes in (Sam, 2026-10-08). The page can
  // turn them off.
  const scenery = numbers.backgrounds || { shown: false, scenes: {}, fights: {} };
  let sceneryShown = scenery.shown !== false;
  draw.backgrounds = (on) => { sceneryShown = on !== false; };
  draw.backgroundsShown = () => sceneryShown;
  const layerPics = new Map();
  const layerPic = (scene, layer) => {
    const k = `${scene}-${layer}`;
    if (!layerPics.has(k)) { const im = new Image(); im.src = `art/scene/${k}.png`; layerPics.set(k, im); }
    return layerPics.get(k);
  };
  function sceneOf(cur) {
    if (guest && guest.scene) return guest.scene;
    const ids = [1, 0, 2].map((s) => cur.fighters[s] && cur.fighters[s].costume).filter(Boolean);
    const id = ids.find((x) => scenery.fights[x]);
    return id ? scenery.fights[id] : null;
  }
  function backdrop(cur, M) {
    if (!sceneryShown) return;
    const name = sceneOf(cur);
    const scene = name && ((guest && guest.scenes && guest.scenes[name]) || scenery.scenes[name]);
    if (!scene) return;
    for (const layer of scene.layers) {
      const im = layerPic(name, layer.name);
      if (!im.complete || !im.naturalWidth) continue;
      const d = layer.depth;
      // The view's zoom and its sideways pan taken only part of the way: a
      // layer at depth d zooms and pans d of what the world does. Its ground
      // stays on the world's ground line, where the scene meets the floor.
      const z = 1 + (M.a - 1) * d;
      // A layer that is only for the whole fight fades as the view closes in.
      if (layer.fade) {
        const [a, b] = layer.fade;
        ctx.globalAlpha = Math.max(0, Math.min(1, (b - M.a) / (b - a)));
        if (ctx.globalAlpha === 0) continue;
      }
      const floor = M.d * ground + M.f;
      ctx.setTransform(z, 0, 0, z, M.e * d, floor - z * ground);
      const [x0, y0, x1, y1] = layer.box;
      const left = (x0 - camX * d + half) * scale;
      ctx.drawImage(im, left, ground - y1 * scale, (x1 - x0) * scale, (y1 - y0) * scale);
      ctx.globalAlpha = 1;
    }
    ctx.globalAlpha = 1;
    ctx.setTransform(M);
  }

  // Whether every picture asked for so far has loaded or failed: a clip
  // waits for it before its first frame (Sam, 2026-10-08: "cut the very
  // first like millisecond while the scene loads in").
  draw.picturesReady = () => [...pictures.values(), ...layerPics.values()].every((im) => im.complete);

  // A deity glows (Sam, 2026-10-08): soft rings of light round each part
  // still on its body, breathing slowly.
  function glow(cur, pts, seat, now) {
    const breath = 0.5 + 0.5 * Math.sin(now / 420);
    for (const [k, grow] of [[0, 9], [1, 5]]) {
      ctx.globalAlpha = (k ? 0.28 : 0.16) * (0.7 + 0.3 * breath);
      for (const part of cur.parts) {
        if (part.fighter !== seat || !part.attached) continue;
        capsule(pts[part.a], pts[part.b], part.r + one * (grow + 2 * breath), k ? palette.costume.glow_soft : palette.costume.glow);
      }
    }
    ctx.globalAlpha = 1;
  }

  // A shield that stopped a blade shimmers round its fighter for a moment,
  // brightest where the blade met it (sim BodyDef::shield, core's Shield
  // event).
  let flashes = [];
  const FLASH_MS = 320;
  function shimmer(cur, pts, now) {
    flashes = flashes.filter((f) => now - f.t < FLASH_MS);
    for (const f of flashes) {
      const k = 1 - (now - f.t) / FLASH_MS;
      ctx.globalAlpha = 0.35 * k;
      for (const part of cur.parts) {
        if (part.fighter !== f.seat || !part.attached) continue;
        capsule(pts[part.a], pts[part.b], part.r + one * 7, palette.costume.shield);
      }
      ctx.globalAlpha = 0.8 * k;
      ctx.strokeStyle = palette.costume.shield;
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.arc(sx(f.at[0]), sy(f.at[1]), (14 + 22 * (1 - k)) * scale, 0, Math.PI * 2);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
  }

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
  // For a clip rendered frame by frame (clip.js): the time the drawing
  // reads, and a camera that holds a zoom on a point of the world, which a
  // round's close-up closes in from.
  let clock = () => performance.now();
  draw.clock = (fn) => { clock = fn || (() => performance.now()); };
  let view = null;
  draw.view = (v) => { view = v; };
  // The transform the world was last drawn with, for placing the trees.
  let worldM = new DOMMatrix();
  function closeness(now) {
    if (!focus || focus.still) return 0;
    const t = (now - focus.start) / focus.dur;
    if (t <= 0 || t >= 1) return 0;
    const ease = (x) => x * x * (3 - 2 * x);
    return t < 0.2 ? ease(t / 0.2) : t > 0.8 ? ease((1 - t) / 0.2) : 1;
  }

  function draw(prev, cur, alpha) {
    const pts = lerp(prev, cur, alpha);
    const arena = (cur.arena_half || numbers.arena_half) / one;
    if (arena > half) {
      let sum = 0, n = 0;
      for (const p of cur.parts) if (p.fighter === 0 && p.attached) { sum += pts[p.a][0]; n += 1; }
      // While a round's end is looked at, the camera is on the cut: on a
      // wide ground it can be far from the left fighter, and the close-up
      // showed bare ground (the stream, 2026-10-07).
      const px = focus && focus.at ? focus.at[0] / one : n ? sum / n / one : 0;
      camX = Math.max(-(arena - half), Math.min(arena - half, px));
    } else {
      camX = 0;
    }
    sides = cur.fighters.map((f, seat) => (f ? f.side : seat === 0 ? 0 : 1));
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.fillStyle = palette.paper;
    ctx.fillRect(0, 0, W, H);
    const e = closeness(clock());
    if (view) {
      // A held camera: the view's point at its place on the canvas, zoomed;
      // a close-up blends from it toward the cut, closer still.
      const vx = sx(view.at[0]), vy = sy(view.at[1]);
      const qx = W / 2, qy = H * (view.y ?? 0.5);
      const fx = e > 0 ? sx(focus.at[0]) : vx, fy = e > 0 ? sy(focus.at[1]) : vy;
      const z = view.zoom * (1 + 0.8 * e);
      const px = vx + (fx - vx) * e, py = vy + (fy - vy) * e;
      ctx.setTransform(z, 0, 0, z, qx - z * px, qy + (H / 2 - qy) * e - z * py);
    } else if (e > 0) {
      // The cut's point moves toward the middle as the view closes in.
      const px = sx(focus.at[0]), py = sy(focus.at[1]);
      const z = 1 + 1.6 * e;
      const tx = px + (W / 2 - px) * e, ty = py + (H / 2 - py) * e;
      ctx.setTransform(z, 0, 0, z, tx - z * px, ty - z * py);
    }
    worldM = ctx.getTransform();
    backdrop(cur, worldM);
    // The ground reaches past the canvas on each side and below, so a
    // zoomed view never shows its edge. It is drawn over a scene's layers
    // too, so what the fighters stand on reads the same with or without one.
    ctx.fillStyle = (guest && guest.ground) || palette.ground;
    ctx.fillRect(-W, ground, 3 * W, 3 * H);
    ctx.strokeStyle = palette.ground_line;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(-W, ground + 0.5);
    ctx.lineTo(2 * W, ground + 0.5);
    ctx.stroke();
    // The end of a stage to cross: two posts and a lintel.
    if (cur.exit !== null && cur.exit !== undefined) {
      const ex = sx(cur.exit);
      ctx.strokeStyle = palette.line;
      ctx.lineWidth = 6;
      ctx.beginPath();
      ctx.moveTo(ex - 40, ground); ctx.lineTo(ex - 40, ground - 170);
      ctx.moveTo(ex + 40, ground); ctx.lineTo(ex + 40, ground - 170);
      ctx.moveTo(ex - 70, ground - 170); ctx.lineTo(ex + 70, ground - 170);
      ctx.moveTo(ex - 55, ground - 145); ctx.lineTo(ex + 55, ground - 145);
      ctx.stroke();
      ctx.lineWidth = 1;
      ctx.strokeStyle = palette.ground_line;
    }
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
    // A costumed fighter's trunk and legs, then its clothes, then its arms
    // over them, then what it wears on its head; anyone else as before.
    const worn = (seat) => wearing(cur, seat);
    const now = clock();
    cur.fighters.forEach((f, seat) => { if (f && worn(seat) && worn(seat).glow) glow(cur, pts, seat, now); });
    const body = (over) => {
      for (const part of cur.parts) {
        if (over !== Boolean(worn(part.fighter) && !under.has(part.def))) continue;
        ctx.globalAlpha = part.attached && dodging(part.fighter) ? 0.4 : 1;
        capsule(pts[part.a], pts[part.b], part.r, fill(part));
      }
    };
    shimmer(cur, pts, now);
    body(false);
    cur.fighters.forEach((f, seat) => { if (f && worn(seat)) { ctx.globalAlpha = dodging(seat) ? 0.4 : 1; wear(cur, pts, seat, ['waist', 'chest']); } });
    body(true);
    cur.fighters.forEach((f, seat) => { if (f && worn(seat)) { ctx.globalAlpha = dodging(seat) ? 0.4 : 1; wear(cur, pts, seat, ['head']); } });
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
      // A plate of armor: drawn by its costume's picture when there is one,
      // and otherwise as a band of steel along its edges.
      if (s.armor) {
        if (!worn(s.fighter)) {
          for (const [i, j] of s.edges) capsule(pts[i], pts[j], one * 2.2, palette.costume.armor_edge);
          for (const [i, j] of s.edges) capsule(pts[i], pts[j], one * 1.4, palette.costume.armor);
        }
        continue;
      }
      // A turned boomerang is ringed in its thrower's own ink: it is
      // coming back for them.
      if (s.turned) for (const [i, j] of s.edges) capsule(pts[i], pts[j], one * 2.4, inkColor(s.fighter));
      // Each edge cuts from `cut_from` on; the part before it is a handle
      // or a pole, in the hilt's color (a plain sword: its hilt).
      s.edges.forEach(([i, j], k) => {
        const p = pts[i], q = pts[j], g = (s.cut_from ? s.cut_from[k] : 0) / one;
        const m = [p[0] + (q[0] - p[0]) * g, p[1] + (q[1] - p[1]) * g];
        // The cursed blade, which core marks, is its two strands.
        if (s.cursed) cursedStrands(ctx, [sx(m[0]), sy(m[1])], [sx(q[0]), sy(q[1])], Math.max(3, 3.4 * scale), palette);
        else capsule(m, q, one * 1.1, guest && guest.blades && guest.blades[s.fighter] ? palette.costume[guest.blades[s.fighter]] : palette.sword);
        if (g > 0) capsule(p, m, one * 1.6, palette.hilt);
      });
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
    // The trees, over everything in the arena but the meters, each over its
    // fighter as the world was drawn (a zoomed view moves the head).
    ringsDrawn = 0;
    if (trees.length && cur.phase === 'fight') {
      const now = clock();
      const lit = new Set();
      // Each tree is a bubble core moves (content::bubbles): drawn home over
      // its fighter, pushed off the others, kept on the canvas.
      const boxes = trees.map((e) => measureTree(e, pts, cur)).filter(Boolean);
      const list = boxes.map((b) => {
        const s = bubbleAt.get(b.entry.seat) || { x: b.hx, y: b.hy, px: b.hx, py: b.hy };
        return { x: s.x, y: s.y, px: s.px, py: s.py, w: b.w, h: b.h, hx: b.hx, hy: b.hy };
      });
      const moved = layoutBubbles ? layoutBubbles(list, W, H) : list;
      boxes.forEach((b, k) => {
        bubbleAt.set(b.entry.seat, moved[k]);
        drawTree(b, moved[k], now);
        for (const id of b.entry.trace.active) lit.add(`${b.entry.seat}:${id}`);
      });
      litBefore = lit;
    }
    // Seat 0's meter at the top left, seat 1's at the top right, and on a
    // flanked stop seat 2's (the opponent on the left) under seat 0's.
    cur.fighters.forEach((f, seat) => {
      if (!f || f.ink_max <= 0) return;
      if (seat === 0) meter(16, 14, f, seat, true);
      else if (seat === 1) meter(W - 236, 14, f, seat, false);
      else meter(16, 30, f, seat, true);
    });
  }

  // --- Behavior trees over the opponents (training) -------------------------
  // Each entry is { seat, tree, trace }: the tree as core describes it, with
  // each node's text filled from the copy by the page, and what core says
  // ran on the last tick. Nothing here decides; it lays out and lights what
  // it is given (after the live views of behavior tree tools: light the path
  // that ran this tick, and keep the tree small enough to read at a glance).
  let trees = [];
  let ringsDrawn = 0;
  draw.trees = (list) => { trees = list || []; };
  // Where each seat's bubble is and was, and the function in core that
  // moves them a frame (wasm bubbles_step).
  const bubbleAt = new Map();
  let layoutBubbles = null;
  draw.bubbleLayout = (fn) => { layoutBubbles = fn; };
  // For the gate: how many trees are drawn, and where their bubbles are.
  draw.shown = () => ({ trees: trees.length, bubbles: [...bubbleAt.values()], rings: ringsDrawn });
  const iconCache = new Map();
  const icon = (name, lit) => {
    const k = lit ? `${name}-lit` : name;
    if (!iconCache.has(k)) { const im = new Image(); im.src = `icons/${k}.png`; iconCache.set(k, im); }
    return iconCache.get(k);
  };
  let NODE_R = 14, COL_W = 36, ROW_H = 34;
  // Larger trees where a page is about them (the BT Lab's lesson).
  draw.treeScale = (k) => { NODE_R = 14 * k; COL_W = 36 * k; ROW_H = 34 * k; };
  const across = (n) => n.kind === 'selector' || n.kind === 'parallel' || n.kind === 'repeat';
  // Columns and rows: a selector, parallel or repeat spreads its children
  // across; a sequence stacks its steps under itself.
  function lay(n, col, row, out) {
    if (!n.children.length) { out.set(n.id, { col, row, n }); return 1; }
    if (across(n)) {
      let c = col;
      for (const k of n.children) c += lay(k, c, row + 1, out);
      out.set(n.id, { col: col + (c - col - 1) / 2, row, n });
      return c - col;
    }
    out.set(n.id, { col, row, n });
    let r = row + 1, w = 1;
    for (const k of n.children) {
      const sub = new Map();
      w = Math.max(w, lay(k, col, r, sub));
      for (const [id, v] of sub) out.set(id, v);
      r = Math.max(...[...sub.values()].map((v) => v.row)) + 1;
    }
    return w;
  }
  const since = new Map(); // `${seat}:${id}` -> when that node lit
  let litBefore = new Set();
  // A tree's card: its layout, its size with the caption line above it
  // (CAPTION_H), and its home, centred over the fighter's highest point.
  const CAPTION_H = 24;
  function measureTree(entry, pts, cur) {
    let top = -Infinity, sum = 0, cnt = 0;
    for (const p of cur.parts) {
      if (p.fighter !== entry.seat || !p.attached) continue;
      for (const i of [p.a, p.b]) { top = Math.max(top, pts[i][1]); sum += pts[i][0]; cnt += 1; }
    }
    if (!cnt) return null;
    const pos = new Map();
    const cols = lay(entry.tree, 0, 0, pos);
    const rows = Math.max(...[...pos.values()].map((v) => v.row)) + 1;
    const w = cols * COL_W + 8, h = rows * ROW_H + 4 + CAPTION_H;
    const hx = sx(sum / cnt), hy = sy(top);
    const head = [worldM.a * hx + worldM.c * hy + worldM.e, worldM.b * hx + worldM.d * hy + worldM.f];
    return { entry, pos, cols, w, h, head, hx: head[0], hy: head[1] - 30 - h / 2 };
  }

  function drawTree(box, at0, now) {
    const { entry, pos, cols, head } = box;
    const { seat, trace } = entry;
    const cx = at0.x;
    const y0 = at0.y - box.h / 2 + CAPTION_H + 4;
    const w = box.w - 8, h = box.h - 4 - CAPTION_H;
    // The bubble's tail, down to its fighter.
    ctx.strokeStyle = palette.ground_line;
    ctx.lineWidth = 1;
    ctx.beginPath(); ctx.moveTo(cx, at0.y + box.h / 2); ctx.lineTo(head[0], head[1] - 6); ctx.stroke();
    const at = (id) => { const v = pos.get(id); return [cx + (v.col - (cols - 1) / 2) * COL_W, y0 + v.row * ROW_H + NODE_R]; };
    const active = new Set(trace.active), held = new Set(trace.held), failed = new Set(trace.failed);
    for (const id of active) {
      const k = `${seat}:${id}`;
      if (!litBefore.has(k)) since.set(k, now);
    }
    // A pale card behind, so the tree reads over the arena.
    ctx.globalAlpha = 0.82;
    ctx.fillStyle = palette.paper;
    ctx.fillRect(cx - w / 2 - 4, y0 - 4, w + 8, h + 4);
    ctx.globalAlpha = 1;
    // Edges: a selector's or parallel's straight down to each child; a
    // sequence's along a spine. The lit path flows.
    const edge = (a, b, lit, spine) => {
      ctx.strokeStyle = lit ? palette.focus : palette.ground_line;
      ctx.lineWidth = lit ? 2.5 : 1;
      ctx.setLineDash(lit ? [5, 4] : []);
      ctx.lineDashOffset = lit ? -now / 40 : 0;
      ctx.beginPath();
      if (spine) { ctx.moveTo(a[0] - NODE_R + 3, a[1] + NODE_R); ctx.lineTo(a[0] - NODE_R + 3, b[1]); ctx.lineTo(b[0] - NODE_R, b[1]); }
      else { ctx.moveTo(a[0], a[1] + NODE_R); ctx.lineTo(b[0], b[1] - NODE_R); }
      ctx.stroke();
    };
    for (const v of pos.values()) {
      for (const k of v.n.children) edge(at(v.n.id), at(k.id), active.has(v.n.id) && active.has(k.id), !across(v.n));
    }
    ctx.setLineDash([]);
    // Nodes.
    for (const v of pos.values()) {
      const n = v.n, [x, y] = at(n.id), lit = active.has(n.id);
      const t0 = since.get(`${seat}:${n.id}`);
      if (lit && t0 !== undefined && now - t0 < 350) {
        // A ring that opens out as the node lights.
        const f = (now - t0) / 350;
        ctx.globalAlpha = 1 - f;
        ctx.strokeStyle = palette.focus;
        ctx.lineWidth = 2;
        ctx.beginPath(); ctx.arc(x, y, NODE_R + 2 + f * 12, 0, Math.PI * 2); ctx.stroke();
      }
      ctx.globalAlpha = failed.has(n.id) && !lit ? 0.35 : 1;
      ctx.fillStyle = lit ? palette.focus : palette.paper;
      ctx.beginPath(); ctx.arc(x, y, NODE_R, 0, Math.PI * 2); ctx.fill();
      ctx.strokeStyle = held.has(n.id) ? palette.fighters.right.body : palette.line;
      ctx.lineWidth = held.has(n.id) ? 3 : 1;
      ctx.setLineDash(n.interrupt ? [3, 2] : []);
      ctx.stroke();
      ctx.setLineDash([]);
      // A node the page is about, ringed (the BT Lab's lesson).
      // Dark on a paper gap, so it reads apart from a lit node's color,
      // and breathing slowly so the eye finds it.
      if (entry.ring && entry.ring.has(n.id)) {
        const r = NODE_R + 6 + Math.sin(now / 300) * 1.5;
        ctx.strokeStyle = palette.paper;
        ctx.lineWidth = 6;
        ctx.beginPath(); ctx.arc(x, y, r, 0, Math.PI * 2); ctx.stroke();
        ctx.strokeStyle = palette.line;
        ctx.lineWidth = 3;
        ctx.beginPath(); ctx.arc(x, y, r, 0, Math.PI * 2); ctx.stroke();
        ringsDrawn += 1;
      }
      const im = icon(n.icon, lit);
      if (im.complete && im.naturalWidth) ctx.drawImage(im, x - NODE_R + 2, y - NODE_R + 2, 2 * NODE_R - 4, 2 * NODE_R - 4);
      ctx.globalAlpha = 1;
    }
    // What it is doing now, in words, above the tree.
    const leaf = trace.active.length ? pos.get(trace.active[trace.active.length - 1]) : null;
    if (leaf && entry.caption) {
      // The words grow with the tree (draw.treeScale).
      const k = NODE_R / 14;
      ctx.font = `${Math.round(13 * k)}px Georgia, serif`;
      const text = entry.caption(leaf.n);
      const tw = ctx.measureText(text).width;
      const bx = Math.max(4, Math.min(W - tw - 12, cx - tw / 2 - 4 * k));
      ctx.fillStyle = palette.focus;
      ctx.fillRect(bx, y0 - 24 * k, tw + 8 * k, 19 * k);
      ctx.fillStyle = palette.paper;
      ctx.fillText(text, bx + 4 * k, y0 - 10 * k);
    }
  }

  let lastPhase = 'fight';
  draw.reset = () => { marks = []; flashes = []; lastPhase = 'fight'; trees = []; since.clear(); litBefore = new Set(); bubbleAt.clear(); };
  draw.events = (frame) => {
    // A new round stands both fighters back up; its marks start clean.
    if (lastPhase !== 'fight' && frame.phase === 'fight') marks = [];
    lastPhase = frame.phase;
    for (const e of frame.events) {
      if (e.Cut && e.Cut.spilled) marks.push({ at: [e.Cut.at.x, e.Cut.at.y], seat: e.Cut.seat });
      if (e.Shield) flashes.push({ seat: e.Shield.seat, at: [e.Shield.at.x, e.Shield.at.y], t: clock() });
    }
  };
  return draw;
}
