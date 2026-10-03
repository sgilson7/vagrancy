// Deployment settings for online play, after Floodline's web/config.js.
// Edited here and copied as-is into the build.
export const CONFIG = {
  // The vendored Trystero bundle, pinned by sha256 in web/vendor/SHA256SUMS
  // and loaded only when somebody hosts or joins by room code.
  bundle: 'vendor/trystero-nostr-0.25.4.js',
  // Keeps this game's rooms apart from every other Trystero app on the same
  // public relays. The room name adds the build hash and the room code.
  appId: 'vagrancy-p2p',
  // Encrypts the session descriptions as they cross a relay this project
  // does not run. Not a secret: it ships in this file. Game traffic is
  // DTLS-encrypted end to end regardless.
  password: 'vagrancy',
  // STUN tells each browser its public address. When both players are behind
  // strict NATs only a TURN relay can carry the traffic, and there is no
  // serverless substitute (online.error.no_path says so).
  rtcConfig: {
    iceServers: [
      { urls: 'stun:stun.l.google.com:19302' },
      { urls: 'stun:global.stun.twilio.com:3478' },
    ],
  },
  // How long to wait for a relay to introduce anybody before saying so.
  relayTimeoutMs: 15000,
};
