// Keyboard diagram display settings, remembered per keyboard. The system only reports key codes, not where a key sits, so positions on the
// diagram assume a standard keyboard and are corrected by modifier order and calibration when they don't match. This only affects the UI, is stored by the UI itself and is not written to the config file.
import { baseOfShown, swapShown } from "./keys.js";

const STORE = "kanami.keyboards";

function read() {
  try {
    return JSON.parse(localStorage.getItem(STORE)) ?? {};
  } catch {
    return {};
  }
}

const saved = read();
// device: the keyboard typed on most recently; orders: product name → "mac" or "win"; swaps: product name → { key at the standard position → key actually sent };
// confirmed: product name → standard positions confirmed against the real keyboard (including ones that were already right); seen: keyboards already prompted about
export const keyboards = $state({
  device: saved.device ?? "",
  orders: saved.orders ?? {},
  swaps: saved.swaps ?? {},
  confirmed: saved.confirmed ?? {},
  seen: saved.seen ?? [],
});

function save() {
  try {
    localStorage.setItem(STORE, JSON.stringify($state.snapshot(keyboards)));
  } catch {}
}

export function noteDevice(device) {
  if (device && device !== keyboards.device) {
    keyboards.device = device;
    save();
  }
}

export const bottomRow = (platform) => keyboards.orders[keyboards.device] ?? (platform === "win" ? "win" : "mac");

// Changing the modifier order invalidates this keyboard's earlier calibration
export function setBottomRow(order) {
  keyboards.orders[keyboards.device] = order;
  clearCalibration();
}

// Layout detected by the new-keyboard prompt
export function setOrderFor(product, order) {
  keyboards.orders[product] = order;
  delete keyboards.swaps[product];
  delete keyboards.confirmed[product];
  save();
}

export function markSeen(product) {
  if (!keyboards.seen.includes(product)) keyboards.seen.push(product);
  save();
}

// What the key at a standard position is actually shown as after calibration
export const shownKey = (id) => keyboards.swaps[keyboards.device]?.[id] ?? id;

const swapsNow = () => keyboards.swaps[keyboards.device] ?? {};
const confirmedNow = () => keyboards.confirmed[keyboards.device] ?? [];

// Whether the position showing this key on the diagram has been confirmed against the real keyboard
export const isConfirmed = (shown) => confirmedNow().includes(baseOfShown(swapsNow(), shown));

export const confirmedCount = () => confirmedNow().length;

// The position showing target sent `pressed` on the real keyboard: that position counts as confirmed;
// if the keys differ the two keys are swapped, and the position that got swapped away must be confirmed again
export function confirmKey(target, pressed) {
  const swaps = swapsNow();
  const here = baseOfShown(swaps, target);
  const there = baseOfShown(swaps, pressed);
  const rest = confirmedNow().filter((k) => k !== here && k !== there);
  keyboards.confirmed[keyboards.device] = [...rest, here];
  keyboards.swaps[keyboards.device] = swapShown(swaps, target, pressed);
  save();
}

function clearCalibration() {
  delete keyboards.swaps[keyboards.device];
  delete keyboards.confirmed[keyboards.device];
  save();
}

export const resetSwaps = clearCalibration;
