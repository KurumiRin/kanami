// Keys of the keyboard diagram. With no layout chosen, draws every key kanami can remap (full size).
// Per key: id is the kanami key name, win is the key name at the same position on a Windows keyboard (the bottom-row modifier order differs), u is the width.
const K = (id, u = 1, win = id) => ({ id, u, win });
const GAP = (u) => ({ gap: u });
const keys = (...ids) => ids.map((id) => K(id));
const chars = (s) => keys(...s);

export const MAIN = [
  [K("escape"), GAP(1), ...keys("f1", "f2", "f3", "f4"), GAP(0.5), ...keys("f5", "f6", "f7", "f8"), GAP(0.5), ...keys("f9", "f10", "f11", "f12")],
  [K("grave"), ...chars("1234567890"), K("minus"), K("equal"), K("backspace", 2)],
  [K("tab", 1.5), ...chars("qwertyuiop"), K("left_bracket"), K("right_bracket"), K("backslash", 1.5)],
  [K("caps_lock", 1.75), ...chars("asdfghjkl"), K("semicolon"), K("quote"), K("enter", 2.25)],
  [K("left_shift", 2.25), ...chars("zxcvbnm"), K("comma"), K("period"), K("slash"), K("right_shift", 2.75)],
  [K("left_control", 1.25), K("left_option", 1.25, "left_command"), K("left_command", 1.5, "left_option"), K("space", 6),
    K("right_command", 1.5, "right_option"), K("right_option", 1.25, "right_command"), K("fn", 1, "menu"), K("right_control", 1.25)],
];

export const NAV = [
  [K("f13", 1, "print_screen"), K("f14", 1, "scroll_lock"), K("f15", 1, "pause")],
  keys("insert", "home", "page_up"),
  keys("delete", "end", "page_down"),
  [GAP(3)],
  [GAP(1), K("up"), GAP(1)],
  keys("left", "down", "right"),
];

// Total number of keys on the keyboard diagram
export const boardKeyCount = (platform) => {
  const win = platform === "win";
  const count = (rows) => rows.flat().filter((k) => !k.gap).length;
  return count(MAIN) + count(NAV) + count([PAD_TOP[win ? "win" : "mac"]]) + PAD[win ? "win" : "mac"].length;
};

// Keypad: the top row lines up with the F-key row; the five rows below use a grid (Enter and plus span two rows, 0 spans two columns)
const P = (id, w = 1, h = 1) => ({ id: "keypad_" + id, w, h });
export const PAD_TOP = { mac: keys("f16", "f17", "f18", "f19"), win: [GAP(4)] };
export const PAD = {
  mac: [{ id: "num_lock", w: 1, h: 1 }, P("equal"), P("divide"), P("multiply"), P(7), P(8), P(9), P("minus"), P(4), P(5), P(6), P("plus"), P(1), P(2), P(3), P("enter", 1, 2), P(0, 2), P("period")],
  win: [{ id: "num_lock", w: 1, h: 1 }, P("divide"), P("multiply"), P("minus"), P(7), P(8), P(9), P("plus", 1, 2), P(4), P(5), P(6), P(1), P(2), P(3), P("enter", 1, 2), P(0, 2), P("period")],
};
