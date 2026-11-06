// Display name of an app: Bundle ID → "WeChat". Fetched asynchronously on first use.
import { api } from "./api.js";
import { t } from "./i18n.svelte.js";

const names = $state({});
const asked = new Set();

export function appName(id) {
  if (!asked.has(id)) {
    asked.add(id);
    api.appName(id).then((name) => (names[id] = name)).catch(() => {});
  }
  return names[id] ?? id.split(".").pop();
}

// One-line description of a when condition; an empty string when there is none
export function whenText(when) {
  if (!when) return "";
  const parts = [];
  const list = (items) => items.join(t(", "));
  const source = (s) => s.split(".").pop();
  if (when.app?.length) parts.push(t("Only in {apps}", { apps: list(when.app.map(appName)) }));
  if (when.app_not?.length) parts.push(t("Except in {apps}", { apps: list(when.app_not.map(appName)) }));
  if (when.input_source?.length) parts.push(t("Only with input source {sources}", { sources: list(when.input_source.map(source)) }));
  if (when.input_source_not?.length) parts.push(t("Except with input source {sources}", { sources: list(when.input_source_not.map(source)) }));
  if (when.var && Object.keys(when.var).length) parts.push(t("Script variable condition met"));
  if (when.device?.length) parts.push(t("Only on keyboard {keyboards}", { keyboards: list(when.device) }));
  if (when.device_not?.length) parts.push(t("Except on keyboard {keyboards}", { keyboards: list(when.device_not) }));
  return parts.join(t("; "));
}

// Write the picker result back into when, keeping conditions the UI doesn't manage (input source, script variables, etc.). Returns undefined when there are no conditions
export function whenWith(when, mode, apps) {
  const next = { ...(when ?? {}) };
  delete next.$line;
  delete next.app;
  delete next.app_not;
  if (mode === "only" && apps.length) next.app = apps;
  if (mode === "except" && apps.length) next.app_not = apps;
  return Object.keys(next).length ? next : undefined;
}

// Strip the line number the UI uses, to make change detection easier
export function plainWhen(when) {
  if (!when) return undefined;
  const copy = { ...when };
  delete copy.$line;
  return copy;
}

// Set a field; undefined means remove it; do nothing if unchanged
export function fieldOp(path, key, before, after) {
  if (JSON.stringify(before) === JSON.stringify(after)) return [];
  return after === undefined
    ? [{ op: "remove", path: [...path, key] }]
    : [{ op: "set", path: [...path, key], value: after }];
}
