// The transport: two browsers, one reliable channel, and no server of ours.
//
// Ported from Floodline's web/quad_rtc.js (its `window.FLOODLINE_RTC` API,
// :683-710), with the miniquad plugin glue removed and cut down to two seats.
// Two introductions reach the same object, one RTCPeerConnection with one
// negotiated, ordered data channel:
//
//   * a room code: Trystero, over public Nostr relays this project does not
//     run, introduces the two browsers; the room name carries the build hash
//     so two builds cannot meet;
//   * a pasted code: the host's invitation and the joiner's reply carry the
//     whole handshake, compressed, and no relay is involved.
//
// The first byte on the channel says host or joiner, which keeps the star:
// a room is a mesh, and two joiners may meet and must ignore each other.
// Everything above this file sees only `onPeer`, `onLeft`, `onMessage`,
// `onError` and `send`.

import { CONFIG } from './config.js';

const CHANNEL_ID = 40;
const ROLE_HOST = 0x48; // 'H'
const ROLE_JOINER = 0x4a; // 'J'
const GATHER_TIMEOUT_MS = 6000;

// `?ice=none` drops the STUN servers, so two tabs on one machine connect on
// host candidates alone and no request leaves the origin. testing/online.py
// uses it; a player never needs it.
function rtcConfig() {
  const ice = new URLSearchParams(location.search).get('ice');
  return ice === 'none' ? { iceServers: [] } : CONFIG.rtcConfig;
}

export function open({ isHost, mode, room, build, onPeer, onLeft, onMessage, onError }) {
  const s = { isHost, links: new Map(), nextId: 1, closed: false, trystero: null, pending: null, hostLink: null };

  function attach(pc, mine) {
    const link = { id: s.nextId++, pc, mine, ch: null, role: 0, reported: false, gone: false };
    const ch = pc.createDataChannel('vagrancy', { negotiated: true, id: CHANNEL_ID, ordered: true });
    ch.binaryType = 'arraybuffer';
    link.ch = ch;
    ch.onopen = () => {
      try { ch.send(new Uint8Array([isHost ? ROLE_HOST : ROLE_JOINER])); } catch (e) { console.warn(e); }
    };
    ch.onmessage = (e) => {
      const bytes = new Uint8Array(e.data);
      if (link.role === 0) {
        link.role = bytes[0];
        const ok = isHost ? link.role === ROLE_JOINER : (link.role === ROLE_HOST && !s.hostLink);
        if (!ok) { disown(link); return; }
        if (!isHost) s.hostLink = link;
        link.reported = true;
        onPeer(link.id);
        return;
      }
      onMessage(link.id, bytes);
    };
    ch.onclose = () => depart(link);
    pc.addEventListener('connectionstatechange', () => {
      if (pc.connectionState === 'failed' || pc.connectionState === 'closed') depart(link);
    });
    s.links.set(link.id, link);
    return link;
  }

  function hangUp(link) {
    try { link.ch.close(); if (link.mine) link.pc.close(); } catch (e) { /* already closing */ }
  }
  function disown(link) { s.links.delete(link.id); link.gone = true; hangUp(link); }
  function depart(link) {
    if (link.gone) return;
    link.gone = true;
    s.links.delete(link.id);
    if (link.reported) onLeft(link.id);
  }

  // ---- a room code, through Trystero ----------------------------------------

  async function startRoom() {
    let lib;
    try {
      lib = await import(new URL(CONFIG.bundle, document.baseURI).href);
    } catch (e) {
      console.warn('the signalling bundle did not load', e);
      onError('online.error.no_relay');
      return;
    }
    if (s.closed) return;
    const name = `${build}-${room}`;
    let r;
    try {
      r = lib.joinRoom({ appId: CONFIG.appId, password: CONFIG.password, rtcConfig: rtcConfig() }, name, {
        onJoinError: (d) => onError(d && d.peerId ? 'online.error.no_path' : 'online.error.no_relay'),
      });
    } catch (e) {
      console.warn(e);
      onError('online.error.no_relay');
      return;
    }
    s.trystero = r;
    const byRelay = new Map();
    // Trystero 0.25.4 takes these as properties, as Floodline assigns them.
    r.onPeerJoin = (pid) => {
      const pc = r.getPeers()[pid];
      if (!pc) return;
      byRelay.set(pid, attach(pc, false));
    };
    r.onPeerLeave = (pid) => {
      const link = byRelay.get(pid);
      byRelay.delete(pid);
      if (link) depart(link);
    };
    setTimeout(() => {
      if (s.closed || s.links.size > 0) return;
      let open = 0;
      try {
        const sockets = lib.getRelaySockets();
        for (const k in sockets) if (sockets[k] && sockets[k].readyState === 1) open++;
      } catch (e) { /* the strategy may not expose them */ }
      if (open === 0) onError('online.error.no_relay');
    }, CONFIG.relayTimeoutMs);
  }

  // ---- a pasted code ----------------------------------------------------------

  function gathered(pc) {
    if (pc.iceGatheringState === 'complete') return Promise.resolve();
    return new Promise((resolve) => {
      let done = false;
      const finish = () => { if (!done) { done = true; resolve(); } };
      pc.addEventListener('icegatheringstatechange', () => { if (pc.iceGatheringState === 'complete') finish(); });
      setTimeout(finish, GATHER_TIMEOUT_MS);
    });
  }

  async function invitation() {
    const pc = new RTCPeerConnection(rtcConfig());
    s.pending = attach(pc, true);
    await pc.setLocalDescription(await pc.createOffer());
    await gathered(pc);
    return pack('O', pc.localDescription.sdp);
  }

  async function reply(code) {
    const sdp = await unpack('O', code);
    const pc = new RTCPeerConnection(rtcConfig());
    s.pending = attach(pc, true);
    await pc.setRemoteDescription({ type: 'offer', sdp });
    await pc.setLocalDescription(await pc.createAnswer());
    await gathered(pc);
    return pack('A', pc.localDescription.sdp);
  }

  async function accept(code) {
    const sdp = await unpack('A', code);
    await s.pending.pc.setRemoteDescription({ type: 'answer', sdp });
  }

  if (mode === 'room') startRoom();

  return {
    // Host: the invitation to send. Joiner: given an invitation, the reply.
    invitation,
    reply,
    // Host: the joiner's reply.
    accept,
    send(id, bytes) {
      const link = s.links.get(id);
      if (!link || !link.reported || link.ch.readyState !== 'open') return false;
      try { link.ch.send(bytes); return true; } catch (e) { return false; }
    },
    close() {
      s.closed = true;
      if (s.trystero) { try { s.trystero.leave(); } catch (e) { /* gone */ } }
      s.links.forEach(hangUp);
      s.links.clear();
    },
  };
}

// ---- the blob: a data-channel SDP is nearly all boilerplate --------------------
//
// Only a handful of lines say anything the other end cannot infer, so the
// code carries those and `grow` rebuilds a valid description around them
// (Floodline quad_rtc.js `shrink`/`grow`).

function shrink(sdp) {
  const keep = [];
  for (const l of sdp.split(/\r\n|\n/)) {
    if (l.startsWith('a=ice-ufrag:')) keep.push('u' + l.slice(12));
    else if (l.startsWith('a=ice-pwd:')) keep.push('p' + l.slice(10));
    else if (l.startsWith('a=fingerprint:')) keep.push('f' + l.slice(14));
    else if (l.startsWith('a=setup:')) keep.push('s' + l.slice(8));
    else if (l.startsWith('a=mid:')) keep.push('m' + l.slice(6));
    else if (l.startsWith('a=sctp-port:')) keep.push('P' + l.slice(12));
    else if (l.startsWith('a=max-message-size:')) keep.push('M' + l.slice(19));
    else if (l.startsWith('a=candidate:') && / (udp|UDP) /.test(l)) keep.push('c' + l.slice(12));
  }
  return keep.join('\n');
}

function grow(min) {
  let ufrag = '', pwd = '', fp = '', setup = 'actpass', mid = '0', port = '5000', mms = '262144';
  const cands = [];
  for (const line of min.split('\n')) {
    const k = line[0];
    const v = line.slice(1);
    if (k === 'u') ufrag = v;
    else if (k === 'p') pwd = v;
    else if (k === 'f') fp = v;
    else if (k === 's') setup = v;
    else if (k === 'm') mid = v;
    else if (k === 'P') port = v;
    else if (k === 'M') mms = v;
    else if (k === 'c') cands.push('a=candidate:' + v);
  }
  return ['v=0', 'o=- 0 0 IN IP4 127.0.0.1', 's=-', 't=0 0', 'a=group:BUNDLE ' + mid, 'a=msid-semantic: WMS',
    'm=application 9 UDP/DTLS/SCTP webrtc-datachannel', 'c=IN IP4 0.0.0.0', ...cands,
    'a=ice-ufrag:' + ufrag, 'a=ice-pwd:' + pwd, 'a=ice-options:trickle', 'a=fingerprint:' + fp,
    'a=setup:' + setup, 'a=mid:' + mid, 'a=sctp-port:' + port, 'a=max-message-size:' + mms, ''].join('\r\n');
}

const b64url = (bytes) => btoa(String.fromCharCode(...bytes)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
function unb64url(text) {
  const s = atob(text.replace(/-/g, '+').replace(/_/g, '/'));
  return Uint8Array.from(s, (c) => c.charCodeAt(0));
}
async function through(stream, bytes) {
  const w = stream.writable.getWriter();
  w.write(bytes);
  w.close();
  return new Uint8Array(await new Response(stream.readable).arrayBuffer());
}

// "O" for an invitation, "A" for a reply, outside the compressed part, so a
// code pasted into the wrong box is refused with a sentence.
async function pack(kind, sdp) {
  return kind + b64url(await through(new CompressionStream('deflate-raw'), new TextEncoder().encode(shrink(sdp))));
}

export class BadCode extends Error {}

async function unpack(kind, code) {
  const text = (code || '').replace(/\s+/g, '');
  if (text[0] !== kind) throw new BadCode('wrong kind');
  try {
    return grow(new TextDecoder().decode(await through(new DecompressionStream('deflate-raw'), unb64url(text.slice(1)))));
  } catch (e) {
    throw new BadCode(String(e));
  }
}
