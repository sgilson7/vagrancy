// Keys to input bits. The bit for each action comes from core
// (sim::Input::ACTIONS through numbers()); the bindings from
// data/controls.json or the player's own. The page keeps no copy of either.

const held = new Set();

export function listen(isBound) {
  window.addEventListener('keydown', (e) => {
    if (isBound(e.code)) e.preventDefault();
    held.add(e.code);
  });
  window.addEventListener('keyup', (e) => held.delete(e.code));
  window.addEventListener('blur', () => held.clear());
}

// The input byte for one seat: the OR of every held action's bit.
export function bits(binding, actionBits) {
  let b = 0;
  for (const [action, code] of Object.entries(binding)) {
    if (held.has(code) && action in actionBits) b |= actionBits[action];
  }
  return b;
}

// What a key code shows as in a sentence: "KeyQ" reads as "Q".
export function keyName(code) {
  return code.replace(/^Key/, '').replace(/^Digit/, '').replace(/^Arrow/, '');
}
