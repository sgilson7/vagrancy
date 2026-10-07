// Sounds the game makes itself, synthesized in the browser: no audio file
// ships (PLANNING-BRIEF 0.5; LICENSES.md), and nothing is copied from a
// recording or from the show the mood is borrowed from (CLAUDE.md).

let ctx = null;
function audio() {
  if (!ctx) {
    const AC = window.AudioContext || window.webkitAudioContext;
    if (!AC) return null;
    ctx = new AC();
  }
  // Firefox rejects this when the page goes away while the context wakes
  // (CI's gate reloads straight after pressing arcade mode); a sound that
  // cannot play is silence, not an error.
  if (ctx.state === 'suspended') ctx.resume().catch(() => {});
  return ctx;
}

// A plucked string: a bright tone that falls a little in pitch and dies
// away fast, the attack of a lute or shamisen-like instrument.
function pluck(c, out, at, freq, len) {
  const o = c.createOscillator();
  const g = c.createGain();
  const f = c.createBiquadFilter();
  o.type = 'sawtooth';
  o.frequency.setValueAtTime(freq * 1.02, at);
  o.frequency.exponentialRampToValueAtTime(freq, at + 0.05);
  f.type = 'lowpass';
  f.frequency.setValueAtTime(4000, at);
  f.frequency.exponentialRampToValueAtTime(600, at + len);
  g.gain.setValueAtTime(0.0001, at);
  g.gain.exponentialRampToValueAtTime(0.5, at + 0.005);
  g.gain.exponentialRampToValueAtTime(0.0001, at + len);
  o.connect(f).connect(g).connect(out);
  o.start(at);
  o.stop(at + len + 0.05);
}

// A record scratched back and forth: noise through a narrow band that
// sweeps up and down.
function scratch(c, out, at, len) {
  const n = Math.floor(c.sampleRate * len);
  const buf = c.createBuffer(1, n, c.sampleRate);
  const d = buf.getChannelData(0);
  for (let i = 0; i < n; i += 1) d[i] = Math.random() * 2 - 1;
  const src = c.createBufferSource();
  src.buffer = buf;
  const f = c.createBiquadFilter();
  f.type = 'bandpass';
  f.Q.value = 6;
  f.frequency.setValueAtTime(400, at);
  f.frequency.exponentialRampToValueAtTime(2600, at + len * 0.45);
  f.frequency.exponentialRampToValueAtTime(500, at + len);
  const g = c.createGain();
  g.gain.setValueAtTime(0.0001, at);
  g.gain.exponentialRampToValueAtTime(0.35, at + 0.02);
  g.gain.setValueAtTime(0.35, at + len * 0.8);
  g.gain.exponentialRampToValueAtTime(0.0001, at + len);
  src.connect(f).connect(g).connect(out);
  src.start(at);
  src.stop(at + len);
}

// A soft low drum.
function drum(c, out, at) {
  const o = c.createOscillator();
  const g = c.createGain();
  o.type = 'sine';
  o.frequency.setValueAtTime(130, at);
  o.frequency.exponentialRampToValueAtTime(45, at + 0.25);
  g.gain.setValueAtTime(0.0001, at);
  g.gain.exponentialRampToValueAtTime(0.7, at + 0.01);
  g.gain.exponentialRampToValueAtTime(0.0001, at + 0.4);
  o.connect(g).connect(out);
  o.start(at);
  o.stop(at + 0.45);
}

// Arcade mode's door: a drum, a quick scratch, then two plucked notes a
// fourth apart. `volume` is the music volume, 0 to 1.
export function arcade(volume = 0.7) {
  try {
    play(volume);
  } catch {
    // No sound: an old browser, or audio switched off.
  }
}

function play(volume) {
  const c = audio();
  if (!c || volume <= 0) return;
  const out = c.createGain();
  out.gain.value = volume;
  out.connect(c.destination);
  const t = c.currentTime + 0.02;
  drum(c, out, t);
  scratch(c, out, t + 0.06, 0.22);
  pluck(c, out, t + 0.3, 196, 0.6);
  pluck(c, out, t + 0.42, 261.6, 0.9);
}
