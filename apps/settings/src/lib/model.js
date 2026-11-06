// Extracts what each page shows from the config structure (TOML converted to JSON). `$line` is the line of the table header.

export const DEFAULTS = { tap_timeout: 200, combo_timeout: 50, typing_streak: 150 };

// "All keyboards" is "", a given keyboard is the name in [device.<name>]
export const base = (device) => (device ? ["device", device] : []);
const section = (model, device) => (device ? model?.device?.[device] : model);

export const deviceNames = (model) => Object.keys(model?.device ?? {}).filter((k) => k !== "$line");

// Rules and rule groups, in file order (which is also match order)
export function ruleItems(model, device) {
  const root = section(model, device) ?? {};
  const at = base(device);
  const rules = (root.rule ?? []).map((rule, i) => ({ kind: "rule", rule, path: [...at, "rule", i], line: rule.$line }));
  const groups = (root.group ?? []).map((group, i) => ({
    kind: "group",
    group,
    path: [...at, "group", i],
    line: group.$line,
    rules: (group.rule ?? []).map((rule, j) => ({ rule, path: [...at, "group", i, "rule", j] })),
  }));
  return [...rules, ...groups].sort((a, b) => a.line - b.line);
}

export function ruleCount(model, device) {
  return ruleItems(model, device).reduce((n, item) => n + (item.kind === "group" ? item.rules.length : 1), 0);
}

export function remapEntries(model, device) {
  return Object.entries(section(model, device)?.remap ?? {}).filter(([k]) => k !== "$line");
}

export const ruleKind = (rule) =>
  rule.run !== undefined ? "run" : rule.call !== undefined ? "call"
  : rule.hold !== undefined || rule.tap !== undefined ? "taphold"
  : rule.to === "none" ? "off" : "keys";

export const layerNames = (model) => Object.keys(model?.layer ?? {}).filter((k) => k !== "$line");

// Virtual devices among the enumerated devices that are not keyboards are left out
export const isKeyboard = (d) => !/touchbar/i.test(d.product);

// Product name → name in the config: `NIZ BT5.2` → `niz_bt5_2`
export function slug(product, taken) {
  const name = product.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "") || "keyboard";
  let result = name;
  for (let i = 2; taken.includes(result); i++) result = `${name}_${i}`;
  return result;
}

// When a keyboard not yet in the config is first used, write down how to recognize it
export function ensureDevice(model, tab) {
  if (!tab.startsWith("new:")) return { device: tab, ops: [] };
  const product = tab.slice(4);
  const device = slug(product, deviceNames(model));
  return { device, ops: [{ op: "set", path: ["device", device], value: { product }, table: true }] };
}
