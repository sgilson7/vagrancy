// The music slot (PLANNING-BRIEF 0.5; D18). The player picks an audio file
// from their own device and it loops during fights. The file is read by the
// browser and goes nowhere: it becomes a blob: URL, and the optional
// remembered copy lives in this browser's IndexedDB. No audio ships with the
// game.

const DB = 'vagrancy';
const STORE = 'music';

let audio = null;
let url = null;
let name = null;
let volume = 0.7;
let wanted = false; // a fight is on, so the track should be playing
let onChange = () => {};
let lastFile = null;

function db() {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function storeOp(mode, fn) {
  try {
    const d = await db();
    return await new Promise((resolve, reject) => {
      const tx = d.transaction(STORE, mode);
      const req = fn(tx.objectStore(STORE));
      tx.oncomplete = () => resolve(req && req.result);
      tx.onerror = () => reject(tx.error);
    });
  } catch (e) {
    // Storage can be off (a private window); the slot still works without it.
    console.warn(e);
    return null;
  }
}

export function subscribe(fn) {
  onChange = fn;
}

export function current() {
  return { name, volume, playing: !!(audio && !audio.paused) };
}

export function load(file, remember) {
  clear();
  lastFile = file;
  name = file.name;
  url = URL.createObjectURL(new Blob([file.bytes], { type: file.type || 'audio/mpeg' }));
  audio = new Audio(url);
  audio.loop = true;
  audio.volume = volume;
  audio.addEventListener('error', () => {
    const msg = audio && audio.error ? (audio.error.message || `code ${audio.error.code}`) : 'unknown';
    onChange({ error: msg });
  });
  if (remember) storeOp('readwrite', (s) => s.put({ name: file.name, type: file.type, bytes: file.bytes }, 'track'));
  if (wanted) play();
  onChange({});
}

function clear() {
  lastFile = null;
  if (audio) audio.pause();
  if (url) URL.revokeObjectURL(url);
  audio = null;
  url = null;
  name = null;
}

export function remove() {
  clear();
  storeOp('readwrite', (s) => s.delete('track'));
  onChange({});
}

export async function setRemember(on) {
  if (on && lastFile) await storeOp('readwrite', (s) => s.put({ name: lastFile.name, type: lastFile.type, bytes: lastFile.bytes }, 'track'));
  if (!on) await storeOp('readwrite', (s) => s.delete('track'));
}

export async function remembered() {
  return storeOp('readonly', (s) => s.get('track'));
}

export function setVolume(v) {
  volume = v;
  if (audio) audio.volume = v;
}

function play() {
  if (!audio) return;
  const p = audio.play();
  if (p && p.catch) p.catch((e) => onChange({ error: e.message || String(e) }));
}

// Called when a fight starts and stops.
export function fight(on) {
  wanted = on;
  if (on) play();
  else if (audio) audio.pause();
}
