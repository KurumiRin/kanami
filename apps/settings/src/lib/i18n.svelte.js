// Localized UI text. English source strings double as catalog keys, so markup stays readable and a
// missing translation falls back to English. Add a language by adding a file under locales/ and an
// entry in CATALOGS; scripts/check-i18n.mjs checks that every string has a translation.
import { invoke } from "@tauri-apps/api/core";
import zhCN from "./locales/zh-CN.js";

const CATALOGS = { "zh-CN": zhCN };

export const LANGUAGES = [
  { tag: "en", name: "English" },
  { tag: "zh-CN", name: "简体中文" },
];

// `tag` is the language in use; `system` is what the OS asks for when the config doesn't choose.
const state = $state({ tag: "en", system: "en" });

const normalize = (tag) => (tag?.toLowerCase().startsWith("zh") ? "zh-CN" : "en");

function apply(tag) {
  if (state.tag === tag) return;
  state.tag = tag;
  document.documentElement.lang = tag;
  // Backend error messages follow the same language.
  invoke("set_language", { tag }).catch(() => {});
}

// Called once before mounting, so the first paint is already in the right language.
export async function initLanguage() {
  const system = await invoke("system_language").catch(() => navigator.language);
  state.system = normalize(system);
  apply(state.system);
}

// `preference` is `settings.language` from the config: a tag, or undefined to follow the system.
export function setPreference(preference) {
  apply(preference ? normalize(preference) : state.system);
}

export const language = () => state.tag;

export function t(key, params) {
  const text = CATALOGS[state.tag]?.[key] ?? key;
  if (!params) return text;
  return text.replace(/\{(\w+)\}/g, (match, name) => (name in params ? params[name] : match));
}

// Counted text: pass the singular and plural forms after the count.
export const tn = (n, one, other, params) => t(n === 1 ? one : other, { n, ...params });

// Marks a string for translation where it is defined, for lists built outside a component. Render
// it later with t(item.label).
export const N_ = (key) => key;
