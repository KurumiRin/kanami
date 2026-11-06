// One-to-one with the commands on the Rust side (src-tauri/src/main.rs)
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const api = {
  platform: () => invoke("platform"),
  initialPage: () => invoke("initial_page"),
  status: () => invoke("status"),
  control: (command) => invoke("control", { command }),
  subscribe: () => invoke("subscribe"),
  configPath: () => invoke("config_path"),
  readConfig: (path) => invoke("read_config", { path }),
  configModified: (path) => invoke("config_modified", { path }),
  checkConfig: (path, text) => invoke("check_config", { path, text }),
  saveConfig: (path, text, expected) => invoke("save_config", { path, text, expected }),
  configModel: (path) => invoke("config_model", { path }),
  editConfig: (path, ops, expected) => invoke("edit_config", { path, ops, expected }),
  runningApps: () => invoke("running_apps"),
  devices: () => invoke("devices"),
  history: () => invoke("history"),
  historyRead: (id) => invoke("history_read", { id }),
  schema: () => invoke("schema"),
  appName: (id) => invoke("app_name", { id }),
  openInEditor: (path) => invoke("open_in_editor", { path }),
  openPermission: (kind) => invoke("open_permission", { kind }),
  loginItem: () => invoke("login_item"),
  setLoginItem: (enabled, config) => invoke("set_login_item", { enabled, config }),
  onEvent: (f) => listen("kanami-event", (e) => f(e.payload)),
  onDisconnected: (f) => listen("kanami-disconnected", () => f()),
  onPage: (f) => listen("kanami-page", (e) => f(e.payload)),
};
