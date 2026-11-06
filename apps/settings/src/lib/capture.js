// Direct key recording: while the key picker is open, the key you press is the key picked.
// Prefers physical keys sent by the background service (Fn, Caps Lock and left/right modifiers are told apart), and falls back to window keyboard events when the service is not running.
// Remapping is paused and menu shortcuts are removed meanwhile (recording on the Rust side); both are restored when the window closes.
import { invoke } from "@tauri-apps/api/core";
import { onKey } from "./live.js";

const WORDS = {
  left_command: "cmd", right_command: "cmd", left_control: "ctrl", right_control: "ctrl",
  left_option: "opt", right_option: "opt", left_shift: "shift", right_shift: "shift", fn: "fn",
};

// Browser KeyboardEvent.code → kanami key name
const CODES = {
  Escape: "escape", Enter: "enter", Tab: "tab", Space: "space", Backspace: "backspace", Delete: "delete",
  Backquote: "grave", Minus: "minus", Equal: "equal", BracketLeft: "left_bracket", BracketRight: "right_bracket",
  Backslash: "backslash", Semicolon: "semicolon", Quote: "quote", Comma: "comma", Period: "period", Slash: "slash",
  ArrowLeft: "left", ArrowRight: "right", ArrowUp: "up", ArrowDown: "down",
  Home: "home", End: "end", PageUp: "page_up", PageDown: "page_down",
  CapsLock: "caps_lock", ControlLeft: "left_control", ControlRight: "right_control",
  ShiftLeft: "left_shift", ShiftRight: "right_shift", AltLeft: "left_option", AltRight: "right_option",
  MetaLeft: "left_command", MetaRight: "right_command", Fn: "fn",
  NumpadEnter: "keypad_enter", NumpadAdd: "keypad_plus", NumpadSubtract: "keypad_minus",
  NumpadMultiply: "keypad_multiply", NumpadDivide: "keypad_divide", NumpadDecimal: "keypad_period",
  NumpadEqual: "keypad_equal", NumLock: "num_lock",
  Insert: "insert", PrintScreen: "print_screen", ScrollLock: "scroll_lock", Pause: "pause", ContextMenu: "menu",
};

function keyFromCode(code) {
  if (CODES[code]) return CODES[code];
  let m;
  if ((m = /^Key([A-Z])$/.exec(code))) return m[1].toLowerCase();
  if ((m = /^Digit(\d)$/.exec(code))) return m[1];
  if ((m = /^Numpad(\d)$/.exec(code))) return "keypad_" + m[1];
  if ((m = /^F(\d{1,2})$/.exec(code))) return "f" + m[1];
  return null;
}

// chord: modifiers are remembered and combined with the next key; pressing a modifier alone gives that modifier. plain: modifiers are not accepted.
// onHeld: modifiers currently held (spelled like cmd), for live display. onPressed: names of keys currently held. onDone: an expression was recorded.
// Returns a function that stops recording.
export function startCapture({ chord, plain, onHeld, onPressed, onDone }) {
  let connected = null;
  let stopped = false;
  const held = [];
  const pressed = new Set();
  invoke("recording", { on: true }).then((c) => (connected = c)).catch(() => (connected = false));

  const words = () => [...new Set(held.map((k) => WORDS[k]))];

  function press(key) {
    pressed.add(key);
    onPressed(new Set(pressed));
    if (WORDS[key]) {
      if (plain) return;
      if (!chord) return onDone(key);
      held.push(key);
      onHeld(words());
      return;
    }
    onDone([...(chord ? words() : []), key].join("+"));
  }

  function release(key) {
    pressed.delete(key);
    onPressed(new Set(pressed));
    const i = held.indexOf(key);
    if (i < 0) return;
    const alone = held.length === 1;
    held.splice(i, 1);
    onHeld(words());
    if (alone) onDone(key);
  }

  const off = onKey((e) => {
    if (e.type !== "key") return;
    if (e.input.state === "down") press(e.input.key);
    else if (e.input.state === "up") release(e.input.key);
  });

  function onDown(e) {
    e.preventDefault();
    e.stopPropagation();
    if (connected !== false || e.repeat) return;
    const key = keyFromCode(e.code);
    if (key) press(key);
  }
  function onUp(e) {
    e.preventDefault();
    e.stopPropagation();
    if (connected !== false) return;
    const key = keyFromCode(e.code);
    if (key) release(key);
  }
  window.addEventListener("keydown", onDown, true);
  window.addEventListener("keyup", onUp, true);

  return () => {
    if (stopped) return;
    stopped = true;
    off();
    window.removeEventListener("keydown", onDown, true);
    window.removeEventListener("keyup", onUp, true);
    invoke("recording", { on: false }).catch(() => {});
  };
}
