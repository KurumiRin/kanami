// Checks the settings window's text: every t(), tn() and N_() string needs a translation with the
// same placeholders, and Chinese may only appear in the locale files.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, dirname, relative, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "../src");
const catalog = (await import(pathToFileURL(join(root, "lib/locales/zh-CN.js")))).default;

const files = (dir) =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });

// A quoted string, in any of JS's quote styles, starting at a call's first argument.
const STRING = /(?:"((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'|`((?:[^`\\$]|\\.)*)`)/.source;
const CALL = new RegExp(`\\b(?:t|N_)\\(\\s*${STRING}`, "g");
const COUNTED = new RegExp(`\\btn\\([^,]+,\\s*${STRING}\\s*,\\s*${STRING}`, "g");
const unescape = (s) => s.replace(/\\(.)/g, (_, c) => (c === "n" ? "\n" : c));
const names = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join();

const problems = [];
let used = 0;
for (const file of files(root)) {
  const path = relative(root, file).split(sep).join("/");
  if (path.startsWith("lib/locales/")) continue;
  const source = readFileSync(file, "utf8");
  const keys = [];
  for (const m of source.matchAll(CALL)) keys.push(m[1] ?? m[2] ?? m[3]);
  for (const m of source.matchAll(COUNTED)) {
    keys.push(m[1] ?? m[2] ?? m[3], m[4] ?? m[5] ?? m[6]);
  }
  for (const raw of keys) {
    used++;
    const key = unescape(raw);
    if (!(key in catalog)) problems.push(`${path}: no translation for ${JSON.stringify(key)}`);
    else if (names(key) !== names(catalog[key])) problems.push(`${path}: placeholders differ in ${JSON.stringify(key)}`);
  }
  source.split("\n").forEach((line, i) => {
    if (/\p{Script=Han}/u.test(line) && !/name: "简体中文"/.test(line)) {
      problems.push(`${path}:${i + 1}: Chinese outside the locale files: ${line.trim().slice(0, 80)}`);
    }
  });
}

if (problems.length) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log(`i18n ok: ${used} strings, ${Object.keys(catalog).length} translations`);
