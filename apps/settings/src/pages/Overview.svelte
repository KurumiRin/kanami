<script>
  import Icon from "../lib/Icon.svelte";
  import Seg from "../lib/Seg.svelte";
  import { api } from "../lib/api.js";
  import { config, edit } from "../lib/config.svelte.js";
  import { DEFAULTS } from "../lib/model.js";
  import { LANGUAGES, N_, t } from "../lib/i18n.svelte.js";

  // status: state of the background service (null when it is not running)
  let { status, configPath, refresh } = $props();

  let login = $state(false);
  let busy = $state(false);
  let error = $state("");

  $effect(() => {
    api.loginItem().then((v) => (login = v));
  });

  async function run(task) {
    busy = true;
    error = "";
    try {
      await task();
      await refresh();
      login = await api.loginItem();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const togglePause = () => run(() => api.control(status.paused ? "resume" : "pause"));
  const toggleLogin = () => run(() => api.setLoginItem(!login, configPath));
  const start = () => run(() => api.setLoginItem(true, configPath));

  // Timing settings: dragging only changes the display, the config is written on release
  const TIMING = [
    { key: "tap_timeout", label: N_("Tap/hold threshold"), hint: N_("Holding a key longer than this counts as a hold"), min: 100, max: 500, step: 10 },
    { key: "typing_streak", label: N_("Typing streak window"), hint: N_("Within this time of the previous key, tap/hold keys act as normal keys right away and combos don't wait. 0 turns it off"), min: 0, max: 400, step: 10 },
    { key: "combo_timeout", label: N_("Combo window"), hint: N_("Keys pressed within this time of each other count as pressed together"), min: 10, max: 150, step: 5 },
  ];
  let draft = $state({});
  const timing = (key) => draft[key] ?? config.model?.settings?.[key] ?? DEFAULTS[key];
  async function commit(key, value) {
    if (value === DEFAULTS[key] && config.model?.settings?.[key] === undefined) {
      delete draft[key];
      return;
    }
    await edit({ op: "set", path: ["settings", key], value });
    delete draft[key];
  }

  // "" follows the system language
  const languageOptions = $derived([{ value: "", label: t("Follow system") }, ...LANGUAGES.map((l) => ({ value: l.tag, label: l.name }))]);
  const language = () => config.model?.settings?.language ?? "";
  const setLanguage = (tag) => {
    if (tag === language()) return;
    return edit(tag ? { op: "set", path: ["settings", "language"], value: tag } : { op: "remove", path: ["settings", "language"] });
  };
</script>

<section class="page">
  <h1>{t("Overview")}</h1>
  {#if !status}
    <p class="lead">{t("Remapping isn't running.")}</p>
    <div class="group">
      <div class="row">
        <div class="main">{t("Start remapping")}<small>{t("Installs kanami as a background service that starts at login and restarts if it quits unexpectedly")}</small></div>
        <button class="btn primary" disabled={busy} onclick={start}>{t("Start")}</button>
      </div>
    </div>
  {:else}
    <p class="lead">
      {#if status.paused}{t("Remapping is paused. All keys pass through unchanged.")}{:else}{t("kanami is remapping keys with your config. Saved changes apply right away.")}{/if}
    </p>
  {/if}

  {#if error}<div class="banner err" style="margin-top:16px"><Icon name="alert" /><span>{error}</span></div>{/if}

  {#if status}
    <h2 class="group-title">{t("Permissions")}</h2>
    <div class="group">
      <div class="row">
        <span class="glyph" style="background:var(--mod-top)"><Icon name="access" /></span>
        <div class="main">{t("Accessibility")}<small>{t("Needed for remapping")}</small></div>
        {#if status.permissions?.accessibility}<span class="status-ok"><Icon name="check" />{t("Allowed")}</span>
        {:else}<button class="btn" onclick={() => api.openPermission("accessibility")}>{t("Open System Settings")}</button>{/if}
      </div>
      <div class="row">
        <span class="glyph" style="background:var(--accent)"><Icon name="keyboard" /></span>
        <div class="main">{t("Input Monitoring")}<small>{t("Needed for per-keyboard rules")}</small></div>
        {#if status.permissions?.input_monitoring}<span class="status-ok"><Icon name="check" />{t("Allowed")}</span>
        {:else}<span class="status-warn">{t("Not allowed")}</span><button class="btn" onclick={() => api.openPermission("input_monitoring")}>{t("Open System Settings")}</button>{/if}
      </div>
    </div>
  {/if}

  <h2 class="group-title">{t("General")}</h2>
  <div class="group">
    {#if status}
      <div class="row">
        <div class="main">{t("Pause remapping")}<small>{t("While paused, all keys pass through unchanged. You can also press left Control + Space + Esc together")}</small></div>
        <button class="switch" role="switch" aria-checked={status.paused} aria-label={t("Pause remapping")} disabled={busy} onclick={togglePause}></button>
      </div>
    {/if}
    <div class="row">
      <div class="main">{t("Layer HUD")}<small>{t("Hold a layer key for about 0.4 seconds to show that layer's keys at the bottom of the screen")}</small></div>
      <button class="switch" role="switch" aria-checked={config.model?.settings?.layer_hud !== false} aria-label={t("Layer HUD")}
        onclick={() => edit(config.model?.settings?.layer_hud === false
          ? { op: "remove", path: ["settings", "layer_hud"] }
          : { op: "set", path: ["settings", "layer_hud"], value: false })}></button>
    </div>
    <div class="row">
      <div class="main">{t("Language")}</div>
      <Seg bind:value={language, setLanguage} options={languageOptions} />
    </div>
    <div class="row">
      <div class="main">{t("Start at login")}<small>{login ? t("Installed as a background service. Turning it off also stops remapping") : t("Runs in the background and restarts if it quits unexpectedly")}</small></div>
      <button class="switch" role="switch" aria-checked={login} aria-label={t("Start at login")} disabled={busy} onclick={toggleLogin}></button>
    </div>
    <div class="row">
      <div class="main">{t("Config file")}<small>{configPath}{#if status?.error}<span class="cfg-err">{t("Your config has an error. kanami is using the last good config")}</span>{:else if status?.summary}<span>{status.summary}</span>{/if}</small></div>
      <button class="btn" onclick={() => api.openInEditor(configPath)}><Icon name="external" />{t("Open in editor")}</button>
    </div>
  </div>

  <h2 class="group-title">{t("Timing")}</h2>
  <div class="group">
    {#each TIMING as item (item.key)}
      <div class="row">
        <div class="main">{t(item.label)}<small>{t(item.hint)}</small></div>
        <div class="slider">
          <input type="range" aria-label={t(item.label)} min={item.min} max={item.max} step={item.step} value={timing(item.key)}
            oninput={(e) => (draft[item.key] = +e.currentTarget.value)} onchange={(e) => commit(item.key, +e.currentTarget.value)} />
          <output>{t("{n} ms", { n: timing(item.key) })}</output>
        </div>
      </div>
    {/each}
  </div>
</section>

<style>
  .glyph :global(.i) { stroke-width: 1.8; }
  .status-ok :global(.i) { width: 15px; height: 15px; stroke-width: 2.2; }
  small span { display: block; }
  .cfg-err { color: var(--err); }
</style>
