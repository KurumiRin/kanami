import { t } from "./i18n.svelte.js";

// Keycap legends: kanami key name → display text. Modifiers use each platform's own names.

const MAC = {
  left_control: "control", right_control: "r control",
  left_option: "option", right_option: "r option",
  left_command: "command", right_command: "r command",
  left_shift: "shift", right_shift: "r shift",
};
const WIN = {
  left_control: "Ctrl", right_control: "r Ctrl",
  left_option: "Alt", right_option: "r Alt",
  left_command: "Win", right_command: "r Win",
  left_shift: "Shift", right_shift: "r Shift",
};
const COMMON = {
  caps_lock: "caps", escape: "esc", backspace: "⌫", delete: "del", enter: "enter", tab: "tab", space: "space",
  left: "←", right: "→", up: "↑", down: "↓", page_up: "pgup", page_down: "pgdn",
  grave: "`", minus: "-", equal: "=", left_bracket: "[", right_bracket: "]", backslash: "\\",
  semicolon: ";", quote: "'", comma: ",", period: ".", slash: "/",
  keypad_period: "kp .", keypad_plus: "kp +", keypad_minus: "kp -", keypad_multiply: "kp *",
  keypad_divide: "kp /", keypad_enter: "kp enter", keypad_equal: "kp =", num_lock: "num",
  print_screen: "prtsc", scroll_lock: "scrlk", insert: "ins",
};

export function legend(key, platform) {
  if (key === "none") return t("Disabled");
  const side = (platform === "win" ? WIN : MAC)[key];
  if (side) return side;
  if (COMMON[key]) return COMMON[key];
  if (key.startsWith("keypad_")) return "kp " + key.slice(7);
  return key;
}

const MODIFIERS = new Set([
  "left_control", "right_control", "left_option", "right_option",
  "left_command", "right_command", "left_shift", "right_shift", "caps_lock", "fn",
]);

export const isModifier = (key) => MODIFIERS.has(key);

// Modifier spellings in chords → keycap text
const CHORD_MAC = { cmd: "⌘", command: "⌘", ctrl: "⌃", control: "⌃", opt: "⌥", option: "⌥", alt: "⌥", shift: "⇧" };
const CHORD_WIN = { cmd: "Win", command: "Win", ctrl: "Ctrl", control: "Ctrl", opt: "Alt", option: "Alt", alt: "Alt", shift: "Shift" };

// Split a key expression into keycaps: `cmd+k` → [{ label: "⌘", mod: true }, { label: "k", mod: false }]
export function chordCaps(expr, platform) {
  const table = platform === "win" ? CHORD_WIN : CHORD_MAC;
  const parts = expr.split("+");
  return parts.map((part) => {
    if (part === "hyper" || part === "meh" || part === "fn") return { label: part, mod: true };
    const m = /^(left|right)_(.+)$/.exec(part);
    if (parts.length > 1 && m && table[m[2]]) return { label: (m[1] === "right" ? "r " : "l ") + table[m[2]], mod: true };
    if (parts.length > 1 && table[part]) return { label: table[part], mod: true };
    if (table[part]) return { label: part, mod: true };
    return { label: legend(part, platform), mod: isModifier(part) };
  });
}

// Calibration: swaps is { key at the standard position → key actually shown }
// Which standard position currently shows `shown`
export const baseOfShown = (swaps, shown) => Object.keys(swaps).find((k) => swaps[k] === shown) ?? shown;

// Swap the two positions showing a and b, dropping entries that end up unchanged
export function swapShown(swaps, a, b) {
  if (a === b) return swaps;
  const next = { ...swaps, [baseOfShown(swaps, a)]: b, [baseOfShown(swaps, b)]: a };
  for (const k of Object.keys(next)) if (next[k] === k) delete next[k];
  return next;
}

// Keys not drawn on the keyboard diagram
const range = (from, to, f) => Array.from({ length: to - from + 1 }, (_, i) => f(from + i));
export const MORE_KEYS = (platform) =>
  platform === "win"
    ? [{ name: t("Function keys"), keys: range(13, 20, (i) => "f" + i) }]
    : [{ name: t("Function keys"), keys: ["f20"] }, { name: t("Other"), keys: ["fn", "menu"] }];

// Legends on the keyboard diagram: space is tight, so modifiers and keypad keys are shortened
const BOARD_MAC = { left_command: "cmd", right_command: "r cmd", left_option: "opt", right_option: "r opt", left_control: "ctrl", right_control: "r ctrl" };
export function boardLegend(key, platform) {
  if (platform !== "win" && BOARD_MAC[key]) return BOARD_MAC[key];
  if (key === "keypad_enter") return "↵";
  if (key.startsWith("keypad_")) return legend(key, platform).replace(/^kp /, "");
  return legend(key, platform);
}

// Key expression ↔ modifier toggles + key
export const MOD_WORDS = ["cmd", "ctrl", "opt", "shift", "hyper"];
const ALIASES = { command: "cmd", control: "ctrl", option: "opt", alt: "opt" };
const GENERIC = new Set([...MOD_WORDS, ...Object.keys(ALIASES)]);

export function splitExpr(expr) {
  if (!expr) return { mods: [], key: "" };
  const parts = expr.split("+").map((p) => ALIASES[p] ?? p);
  const last = parts.at(-1);
  // With only modifiers there is no "key": `cmd+shift`; a bare `left_command` counts as a key
  if (GENERIC.has(last)) return { mods: parts, key: "" };
  return { mods: parts.slice(0, -1), key: last };
}

export const joinExpr = (mods, key) => [...mods, ...(key ? [key] : [])].join("+");

export const modLabel = (word, platform) =>
  word === "hyper" ? "Hyper" : word === "meh" ? "Meh" : word === "fn" ? "fn" : chordCaps(`${word}+a`, platform)[0].label;
