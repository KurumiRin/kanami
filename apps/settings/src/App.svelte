<script>
  import { onMount } from "svelte";
  import Icon from "./lib/Icon.svelte";
  import { N_, setPreference, t } from "./lib/i18n.svelte.js";
  import { api } from "./lib/api.js";
  import { config, load, watch } from "./lib/config.svelte.js";
  import { emitKey } from "./lib/live.js";
  import { noteDevice } from "./lib/keyboards.svelte.js";
  import Overview from "./pages/Overview.svelte";
  import Remap from "./pages/Remap.svelte";
  import Rules from "./pages/Rules.svelte";
  import Layers from "./pages/Layers.svelte";
  import Combos from "./pages/Combos.svelte";
  import Keyboards from "./pages/Keyboards.svelte";
  import Events from "./pages/Events.svelte";
  import Source from "./pages/Source.svelte";
  import Gate from "./pages/Gate.svelte";
  import NewKeyboard from "./lib/NewKeyboard.svelte";

  const MAX_EVENTS = 5000;

  let platform = $state("mac");
  let page = $state("overview");
  let status = $state(null); // background service status; null when it isn't running
  let configPath = $state("");
  let events = $state.raw([]);
  let app = $state(null); // Bundle ID of the frontmost app
  let appLabel = null; // its display name (fetched asynchronously)
  let calibrateRequest = $state(0); // incremented when the new-keyboard prompt asks to start calibration

  const NAV = [
    { id: "overview", label: N_("Overview"), icon: "overview" },
    { id: "remap", label: N_("Remaps"), icon: "remap" },
    { id: "rules", label: N_("Rules"), icon: "rules" },
    { id: "layers", label: N_("Layers"), icon: "layers" },
    { id: "combos", label: N_("Combos"), icon: "combo" },
    { id: "keyboards", label: N_("Keyboards"), icon: "keyboard" },
    { id: "events", label: N_("Event Viewer"), icon: "events" },
  ];
  const ADVANCED = [{ id: "source", label: N_("Config Source"), icon: "code" }];

  // Without Accessibility permission, the whole window is replaced by the authorization page
  const gated = $derived(status?.permissions?.accessibility === false);

  async function refresh() {
    try {
      status = await api.status();
      if (app === null && status.app) setApp(status.app);
      await api.subscribe();
    } catch {
      status = null;
    }
    const path = await api.configPath();
    if (path !== configPath) {
      configPath = path;
      await load(path).catch((e) => (config.error = String(e)));
    } else {
      await watch();
    }
  }

  // Events are queued and merged once per frame, so fast typing doesn't redraw per event
  let pending = [];
  let scheduled = false;
  function receive(event) {
    if (event.type === "app") {
      setApp(event.app);
      return;
    }
    if (event.type === "paused") {
      if (status) status = { ...status, paused: event.paused };
      return;
    }
    if (event.type !== "key" && event.type !== "skipped") return;
    if (event.type === "key") noteDevice(event.device);
    emitKey(event);
    const record = { ...event, at: Date.now(), app: appLabel ?? app };
    pending.push(record);
    if (!scheduled) {
      scheduled = true;
      requestAnimationFrame(() => {
        scheduled = false;
        const next = events.concat(pending);
        pending = [];
        events = next.length > MAX_EVENTS ? next.slice(next.length - MAX_EVENTS) : next;
      });
    }
  }

  function setApp(id) {
    app = id;
    appLabel = null;
    if (id) api.appName(id).then((name) => app === id && (appLabel = name));
  }

  onMount(() => {
    api.platform().then((p) => (platform = p));
    api.initialPage().then((p) => (page = p));
    api.onPage((p) => (page = p));
    api.onEvent(receive);
    api.onDisconnected(() => (status = null));
    refresh();
    const timer = setInterval(refresh, 2000);
    return () => clearInterval(timer);
  });

  $effect(() => {
    document.documentElement.className = platform;
  });

  $effect(() => {
    setPreference(config.model?.settings?.language);
  });
</script>

<div class="shell">
  <nav class="side" aria-label={t("Settings")}>
    <div class="drag" data-tauri-drag-region></div>
    <ul class="nav">
      {#each NAV as item}
        <li><button aria-current={page === item.id ? "page" : undefined} disabled={gated} onclick={() => (page = item.id)}><span class="ico"><Icon name={item.icon} /></span>{t(item.label)}</button></li>
      {/each}
    </ul>
    <div class="nav-sep"></div>
    <p class="nav-label">{t("Advanced")}</p>
    <ul class="nav">
      {#each ADVANCED as item}
        <li><button aria-current={page === item.id ? "page" : undefined} disabled={gated} onclick={() => (page = item.id)}><span class="ico"><Icon name={item.icon} /></span>{t(item.label)}</button></li>
      {/each}
    </ul>
    <p class="state">
      {#if !status}<span class="dot off"></span>{t("Not running")}
      {:else if status.paused}<span class="dot paused"></span>{t("Paused")}
      {:else}<span class="dot on"></span>{t("Running")}{/if}
    </p>
  </nav>

  <main class="content">
    <div class="drag top" data-tauri-drag-region></div>
    {#if config.error}
      <div class="banner err"><Icon name="alert" /><span>{t(config.error)}</span><button class="btn" onclick={() => (config.error = "")}>{t("Dismiss")}</button></div>
    {/if}
    {#if !gated}
      <NewKeyboard connected={() => !!status} onCalibrate={() => (calibrateRequest++, (page = "layers"))} />
    {/if}
    {#if gated}
      <Gate {status} />
    {:else if page === "remap"}
      <Remap {status} {platform} />
    {:else if page === "rules"}
      <Rules {status} {platform} />
    {:else if page === "layers"}
      <Layers {platform} {calibrateRequest} onGoto={(p) => (page = p)} />
    {:else if page === "combos"}
      <Combos {platform} />
    {:else if page === "keyboards"}
      <Keyboards {status} />
    {:else if page === "events"}
      <Events {events} {platform} connected={!!status} onClear={() => (events = [])} />
    {:else if page === "source"}
      {#key configPath}
        {#if configPath}<Source {status} {configPath} />{/if}
      {/key}
    {:else}
      <Overview {status} {configPath} {refresh} />
    {/if}
  </main>
</div>

<style>
  .shell { display: grid; grid-template-columns: 210px 1fr; height: 100vh; }
  .side { background: var(--side-bg); padding: 52px 10px 14px; display: flex; flex-direction: column; position: relative; }
  :global(.mac) .side { border-right: 1px solid var(--line); }
  :global(.win) .side { padding-top: 14px; }
  .drag { position: absolute; left: 0; right: 0; top: 0; height: 40px; }
  .nav { list-style: none; margin: 0; padding: 0; }
  .nav li { margin: 1px 0; }
  .nav button { width: 100%; text-align: left; border: 0; background: none; padding: 6px 10px; border-radius: var(--radius-ctl); cursor: pointer; display: flex; gap: 10px; align-items: center; position: relative; }
  .nav button:hover:not(:disabled) { background: rgba(127,127,127,.1); }
  .nav button:disabled { opacity: .4; cursor: default; }
  .nav button[aria-current="page"] { background: var(--select-bg); }
  :global(.mac) .nav button[aria-current="page"] { color: var(--accent-ink); font-weight: 600; }
  :global(.win) .nav button[aria-current="page"]::before { content: ""; position: absolute; left: 0; top: 9px; bottom: 9px; width: 3px; border-radius: 2px; background: var(--accent); }
  .ico { width: 18px; display: grid; place-items: center; opacity: .8; }
  .ico :global(.i) { width: 17px; height: 17px; }
  .nav button[aria-current="page"] .ico { opacity: 1; }
  :global(.mac) .nav button[aria-current="page"] .ico { color: var(--accent); }
  .nav-sep { height: 1px; background: var(--line); margin: 10px 8px; }
  .nav-label { font-size: 11px; color: var(--ink-3); margin: 0 10px 4px; }
  .state { margin: auto 10px 0; font-size: 12px; color: var(--ink-2); display: flex; align-items: center; gap: 7px; }
  .dot { width: 8px; height: 8px; border-radius: 50%; }
  .dot.on { background: var(--ok); box-shadow: 0 0 0 4px color-mix(in srgb, var(--ok) 20%, transparent); }
  .dot.paused { background: var(--warn); }
  .dot.off { background: var(--ink-3); }
  .content { padding: 30px 34px 36px; overflow: auto; position: relative; min-width: 0; }
  :global(.mac) .content { padding-top: 26px; }
  :global(.win) .content { background: var(--layer-bg); border-top-left-radius: 8px; border-left: 1px solid var(--layer-line); border-top: 1px solid var(--layer-line); }
  .drag.top { height: 26px; }
</style>
