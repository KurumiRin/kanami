<script>
  import { onMount, onDestroy } from "svelte";
  import Icon from "../lib/Icon.svelte";
  import { api } from "../lib/api.js";
  import { createEditor } from "../lib/editor.js";
  import { language, t } from "../lib/i18n.svelte.js";

  let { status, configPath } = $props();

  let host;
  let editor;
  let saved = $state("");
  let current = $state("");
  let modified = 0;
  let check = $state(null);
  let external = $state(null); // version on disk changed by another program { text, modified }
  let comparing = $state(false);
  let historyIds = $state(null);
  let preview = $state(null); // history version being viewed { id, text }
  let error = $state("");
  let checkTimer;
  let pollTimer;

  const dirty = $derived(current !== saved);
  const failed = $derived(check?.tests?.filter((test) => !test.passed).length ?? 0);

  onMount(async () => {
    const file = await api.readConfig(configPath);
    saved = current = file.text;
    modified = file.modified;
    editor = createEditor(host, {
      doc: file.text,
      schema: await api.schema(),
      onChange: (text) => {
        current = text;
        clearTimeout(checkTimer);
        checkTimer = setTimeout(runCheck, 400);
      },
      onSave: save,
    });
    runCheck();
    pollTimer = setInterval(watchDisk, 1500);
  });

  onDestroy(() => {
    clearInterval(pollTimer);
    clearTimeout(checkTimer);
    editor?.destroy();
  });

  async function runCheck() {
    check = await api.checkConfig(configPath, current);
    editor?.showCheck(check);
  }

  async function save() {
    error = "";
    try {
      modified = await api.saveConfig(configPath, current, modified);
      saved = current;
    } catch (e) {
      if (String(e) === "conflict") external = await api.readConfig(configPath);
      else error = String(e);
    }
  }

  // The file was changed in another editor: refresh straight away if there are no edits here, otherwise warn
  async function watchDisk() {
    const m = await api.configModified(configPath);
    if (m === modified || (external && m === external.modified)) return;
    const disk = await api.readConfig(configPath);
    if (!dirty) {
      editor.setDoc(disk.text);
      saved = current = disk.text;
      modified = disk.modified;
    } else {
      external = disk;
    }
  }

  function useDisk() {
    editor.setDoc(external.text);
    saved = current = external.text;
    modified = external.modified;
    external = null;
    comparing = false;
  }

  function keepMine() {
    modified = external.modified; // the next save overwrites the version on disk
    external = null;
    comparing = false;
  }

  async function toggleHistory() {
    historyIds = historyIds ? null : await api.history();
  }

  async function show(id) {
    preview = { id, text: await api.historyRead(id) };
    historyIds = null;
  }

  function restore() {
    editor.setDoc(preview.text);
    preview = null;
  }

  const when = (id) =>
    new Date(id).toLocaleString(language(), { hourCycle: "h23", month: "short", day: "numeric", hour: "2-digit", minute: "2-digit", second: "2-digit" });
</script>

<section class="page source">
  <h1>{t("Config Source")}</h1>
  <p class="lead">{t("Edit config.toml directly. It is checked as you type and takes effect only after you save with ⌘S / Ctrl+S. While it has errors it is not applied, and remapping keeps using the last good config.")}</p>

  <div class="toolbar">
    <span class="path" title={configPath}>{configPath.replace(/^(\/Users|\/home)\/[^/]+/, "~")}</span>
    {#if dirty}<span class="pill">{t("Unsaved")}</span>{/if}
    <div class="menu-anchor">
      <button class="btn" onclick={toggleHistory}><Icon name="history" />{t("Version history")}</button>
      {#if historyIds}
        <div class="menu">
          {#each historyIds as id}<button onclick={() => show(id)}>{when(id)}</button>{/each}
          {#if historyIds.length === 0}<span class="none">{t("No earlier versions yet. Every save keeps a copy.")}</span>{/if}
        </div>
      {/if}
    </div>
    <button class="btn" onclick={() => api.openInEditor(configPath)}><Icon name="external" />{t("Open in external editor")}</button>
    <button class="btn primary" disabled={!dirty} onclick={save}>{t("Save")}</button>
  </div>

  {#if external}
    <div class="banner">
      <Icon name="alert" /><span>{t("config.toml was just changed by another program, and you have unsaved changes here too.")}</span>
      <button class="btn" onclick={() => (comparing = !comparing)}>{comparing ? t("Hide comparison") : t("Compare versions")}</button>
      <button class="btn" onclick={useDisk}>{t("Use the one on disk")}</button>
      <button class="btn" onclick={keepMine}>{t("Keep mine")}</button>
    </div>
  {/if}
  {#if error}<div class="banner err"><Icon name="alert" /><span>{error}</span></div>{/if}
  {#if preview}
    <div class="banner"><Icon name="history" /><span>{t("Viewing the version from before the save at {time}.", { time: when(preview.id) })}</span>
      <button class="btn" onclick={restore}>{t("Restore this version")}</button>
      <button class="btn" onclick={() => (preview = null)}>{t("Close")}</button>
    </div>
  {/if}

  <div class="panes" class:split={comparing || preview}>
    <div class="editor" bind:this={host}></div>
    {#if comparing && external}
      <pre class="other"><b>{t("Version on disk")}</b>{"\n"}{external.text}</pre>
    {:else if preview}
      <pre class="other"><b>{t("Earlier version")}</b>{"\n"}{preview.text}</pre>
    {/if}
  </div>

  <div class="status">
    {#if check?.problem}
      <span class="bad"><Icon name="alert" />{t("Line {line}: {message}", { line: check.problem.line, message: check.problem.message })}</span>
    {:else if check}
      <span class="good"><Icon name="check" />{t("Config is valid: {summary}", { summary: check.summary })}</span>
    {/if}
    {#if check?.tests?.length}
      <span class:bad={failed} class:good={!failed}>{t("{passed}/{total} tests passed", { passed: check.tests.length - failed, total: check.tests.length })}</span>
      {#each check.tests.filter((test) => !test.passed) as test}
        <button class="link" onclick={() => editor.jump(test.line)}>{t("Line {line}: {name}", { line: test.line, name: test.name })}</button>
      {/each}
    {/if}
  </div>
</section>

<style>
  .source { display: flex; flex-direction: column; height: 100%; }
  .source .toolbar { flex-wrap: nowrap; }
  .path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font: 12px var(--mono); color: var(--ink-2); -webkit-user-select: text; user-select: text; }
  .pill { font-size: 11px; padding: 1px 8px; border-radius: 999px; background: rgba(127,127,127,.12); color: var(--ink-2); }
  .panes { flex: 1; min-height: 0; display: grid; gap: 10px; }
  .panes.split { grid-template-columns: 1fr 1fr; }
  .editor, .other { min-height: 0; border-radius: var(--radius-group); background: var(--group-bg); border: 1px solid var(--line); overflow: hidden; }
  .editor { -webkit-user-select: text; user-select: text; }
  .other { margin: 0; padding: 10px 14px; overflow: auto; font: 12.5px/1.75 var(--mono); white-space: pre; -webkit-user-select: text; user-select: text; }
  .other b { font-family: var(--font); color: var(--ink-2); }
  .status { display: flex; gap: 14px; align-items: center; flex-wrap: wrap; margin-top: 10px; font-size: 12px; min-height: 20px; }
  .status span { display: inline-flex; gap: 5px; align-items: center; }
  .status :global(.i) { width: 15px; height: 15px; }
  .link { border: 0; background: none; padding: 0; color: var(--accent-ink); cursor: pointer; font-size: 12px; }
  .good { color: var(--ok); }
  .bad { color: var(--err); }
  .menu-anchor { position: relative; }
  .menu { position: absolute; z-index: 5; right: 0; top: calc(100% + 4px); min-width: 220px; max-height: 320px; overflow: auto; padding: 4px;
    border-radius: 8px; background: var(--win-bg); border: 1px solid var(--line); box-shadow: 0 8px 24px rgba(0,0,0,.18); display: grid; }
  .menu button { border: 0; background: none; text-align: left; padding: 5px 10px; border-radius: 5px; cursor: pointer; }
  .menu button:hover { background: var(--accent-soft); }
  .none { padding: 6px 10px; color: var(--ink-3); font-size: 12px; }
</style>
