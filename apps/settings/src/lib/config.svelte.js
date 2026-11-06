// Config shared by the visual pages: read the structure, edit by path (the Rust side validates and writes back, keeping comments and formatting).
import { api } from "./api.js";
import { N_ } from "./i18n.svelte.js";

export const config = $state({ path: "", model: null, modified: 0, error: "" });

export async function load(path) {
  config.path = path;
  const m = await api.configModel(path);
  config.model = m.model;
  config.modified = m.modified;
}

// See src-tauri/src/edit.rs for ops. If the edited config has errors it is not written; the error shows at the top of the window and false is returned
export async function edit(...ops) {
  config.error = "";
  try {
    const m = await api.editConfig(config.path, ops, config.modified);
    config.model = m.model;
    config.modified = m.modified;
    return true;
  } catch (e) {
    if (String(e) === "conflict") {
      await load(config.path);
      config.error = N_("The config file was changed elsewhere and has been reloaded. Try again.");
    } else {
      config.error = String(e);
    }
    return false;
  }
}

// Reload when the file was changed elsewhere (Config Source page, an external editor)
export async function watch() {
  if (!config.path) return;
  const m = await api.configModified(config.path);
  if (m !== config.modified) await load(config.path).catch((e) => (config.error = String(e)));
}
