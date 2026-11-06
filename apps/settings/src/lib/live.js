// Key events sent by the background service, used by the Layers page live keyboard diagram and key recording.
const listeners = new Set();

export function onKey(f) {
  listeners.add(f);
  return () => listeners.delete(f);
}

export function emitKey(event) {
  for (const f of listeners) f(event);
}
