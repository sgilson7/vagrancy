// A YouTube video, chosen by the player, playing on a loop in a small player
// in the corner of the page. Sam asked for the game's track to play "even if
// its in a weird way like a youtube player" (DECISIONS.md, "Decided by Sam":
// his override of PLANNING-BRIEF 0.5's "no stream"). The video is the one
// Sam chose, already filled in; a player may paste another. Nothing contacts
// YouTube until they press Play, and the
// player stays visible, because YouTube's terms do not allow a hidden one.

const ID = /^[A-Za-z0-9_-]{11}$/;
const KEY = 'vagrancy.youtube';
// Sam's link (2026-10-04): "the song link should be pre-loaded in and you
// just have to hit play".
export const DEFAULT_LINK = 'https://www.youtube.com/watch?v=L7dqdw2i5JM';

// The video's id from a link a person copies, or null.
export function videoId(text) {
  const s = String(text || '').trim();
  if (ID.test(s)) return s;
  let u;
  try { u = new URL(s); } catch { return null; }
  const host = u.hostname.replace(/^(www|m|music)\./, '');
  let id = null;
  if (host === 'youtu.be') id = u.pathname.split('/')[1];
  else if (host === 'youtube.com' || host === 'youtube-nocookie.com') {
    if (u.pathname === '/watch') id = u.searchParams.get('v');
    else {
      const m = u.pathname.match(/^\/(embed|shorts|live)\/([^/]+)/);
      if (m) id = m[2];
    }
  }
  return id && ID.test(id) ? id : null;
}

// The privacy-enhanced embed, looping the one video.
export function embedUrl(id) {
  return `https://www.youtube-nocookie.com/embed/${id}?autoplay=1&loop=1&playlist=${id}&playsinline=1&rel=0`;
}

// The last link played on this device, a convenience only; Sam's otherwise.
export function remembered() {
  try { return localStorage.getItem(KEY) || DEFAULT_LINK; } catch { return DEFAULT_LINK; }
}

let dock = null;

export function playing() {
  return dock !== null;
}

// `stopButton` is built by the page, which owns every string.
export function play(id, stopButton) {
  stop();
  try { localStorage.setItem(KEY, id); } catch { /* the link is not kept */ }
  const frame = document.createElement('iframe');
  frame.src = embedUrl(id);
  frame.width = '200';
  frame.height = '200';
  frame.allow = 'autoplay; encrypted-media';
  frame.referrerPolicy = 'strict-origin-when-cross-origin';
  frame.title = 'YouTube';
  dock = document.createElement('div');
  dock.id = 'youtube-dock';
  dock.append(frame, stopButton);
  document.body.append(dock);
}

export function stop() {
  if (dock) dock.remove();
  dock = null;
}
