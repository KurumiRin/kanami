<script>
  import { onDestroy, tick } from "svelte";
  import Expr from "./Expr.svelte";
  import { startCapture } from "./capture.js";
  import Board from "./Board.svelte";
  import { t } from "./i18n.svelte.js";
  import { boardLegend, joinExpr, legend, MOD_WORDS, modLabel, MORE_KEYS, splitExpr } from "./keys.js";

  // Pick a key: a keycap button that opens a small keyboard; click a key or just press it.
  // mods: modifiers can be added; targets: Hyper and Meh can be picked; none: "Disabled" can be picked; plain: modifiers are not accepted;
  // clearable: can be left unset; exclude: keys that can't be picked. With onpick, the caller saves; otherwise value is changed directly.
  let {
    value = $bindable(""),
    platform,
    mods = false,
    targets = false,
    none = false,
    plain = false,
    clearable = false,
    exclude = [],
    placeholder = undefined,
    out = false,
    onpick = null,
  } = $props();

  let open = $state(false);
  let view = $state("board");
  let held = $state([]);
  let pressed = $state(new Set());
  let trigger = $state();
  let panel = $state();
  let top = $state(0);
  let left = $state(0);
  let stop = null;

  const hint = $derived(placeholder ?? t("Choose a key"));
  const sequence = $derived(Array.isArray(value));
  const parts = $derived(sequence ? { mods: [], key: "" } : splitExpr(value ?? ""));
  const shownMods = $derived(held.length ? held : parts.mods);
  const toggles = $derived([...new Set([...MOD_WORDS, ...parts.mods])]);
  const preview = $derived(joinExpr(shownMods, parts.key));
  const isSet = $derived(sequence || !!value);
  const isModKey = (key) => /^(left|right)_(command|control|option|shift)$/.test(key) || key === "fn";
  const blocked = (key) => exclude.includes(key) || (plain && isModKey(key));

  function change(next) {
    if (onpick) onpick(next);
    else value = next;
  }

  async function show() {
    open = true;
    view = "board";
    held = [];
    stop = startCapture({
      chord: mods,
      plain,
      onHeld: (list) => (held = list),
      onPressed: (set) => (pressed = set),
      onDone: (expr) => {
        if (!blocked(expr)) commit(expr);
      },
    });
    window.addEventListener("blur", close);
    await tick();
    place();
  }

  function place() {
    const r = trigger.getBoundingClientRect();
    const p = panel.getBoundingClientRect();
    left = Math.max(8, Math.min(r.left, innerWidth - p.width - 8));
    top = r.bottom + 6 + p.height > innerHeight - 8 ? Math.max(8, r.top - p.height - 6) : r.bottom + 6;
  }

  function close() {
    open = false;
    pressed = new Set();
    stop?.();
    stop = null;
    window.removeEventListener("blur", close);
  }

  function commit(expr) {
    change(expr);
    close();
  }

  // Click a key on the small keyboard
  function pick(key) {
    if (!blocked(key)) commit(joinExpr(mods ? parts.mods : [], key));
  }

  // Modifier toggles don't close the panel; it ends once a key is picked or "Done" is clicked
  function toggle(word) {
    const next = parts.mods.includes(word) ? parts.mods.filter((m) => m !== word) : [...parts.mods, word];
    change(joinExpr(next, parts.key));
  }

  onDestroy(close);
</script>

<button class="key-btn" class:empty={!isSet} bind:this={trigger} aria-expanded={open} onclick={show}>
  {#if isSet && value === "none"}<span class="badge">{t("Disabled")}</span>
  {:else if isSet}<Expr {value} {platform} {out} />
  {:else}{hint}{/if}
</button>

{#if open}
  <div class="scrim" role="presentation" onpointerdown={close}></div>
  <div class="panel" role="dialog" aria-label={t("Choose a key")} bind:this={panel} style="top:{top}px; left:{left}px">
    <div class="head">
      {#if preview && preview !== "none"}<Expr value={preview} {platform} {out} />{/if}
      <span class="hint-live">{t("Press a key, or click one below")}</span>
      <span class="grow"></span>
      <div class="views" role="group" aria-label={t("Key range")}>
        <button aria-pressed={view === "board"} onclick={() => (view = "board")}>{t("Keyboard")}</button>
        <button aria-pressed={view === "more"} onclick={() => (view = "more")}>{t("More keys")}</button>
      </div>
    </div>

    {#if view === "board"}
      <Board {platform} mini>
        {#snippet key(id, style)}
          <button class="k" class:sel={parts.key === id} class:down={pressed.has(id)} class:na={blocked(id)} disabled={blocked(id)} {style} onclick={() => pick(id)}>{boardLegend(id, platform)}</button>
        {/snippet}
      </Board>
    {:else}
      <div class="more">
        {#each MORE_KEYS(platform) as group (group.name)}
          <div class="more-row">
            <span class="more-name">{group.name}</span>
            {#each group.keys as key (key)}
              <button class="cap pick" class:picked={parts.key === key} disabled={blocked(key)} onclick={() => pick(key)}>{legend(key, platform)}</button>
            {/each}
          </div>
        {/each}
      </div>
    {/if}

    <div class="foot">
      {#if mods}
        <div class="views" role="group" aria-label={t("Modifiers")}>
          {#each toggles as word (word)}
            <button aria-pressed={shownMods.includes(word)} onclick={() => toggle(word)}>{modLabel(word, platform)}</button>
          {/each}
        </div>
      {/if}
      {#if targets && !mods}<button class="btn" onclick={() => commit("hyper")}>Hyper</button>{/if}
      {#if targets}<button class="btn" onclick={() => commit("meh")}>Meh</button>{/if}
      {#if none}<button class="btn" onclick={() => commit("none")}>{t("Disabled")}</button>{/if}
      <span class="grow"></span>
      {#if clearable && isSet}<button class="btn" onclick={() => commit("")}>{hint}</button>{/if}
      <button class="btn" onclick={close}>{mods && isSet ? t("Done") : t("Cancel")}</button>
    </div>
  </div>
{/if}

<style>
  .key-btn { border: 1px solid transparent; background: none; padding: 2px; border-radius: 8px; cursor: pointer; display: inline-flex; align-items: center; min-height: 34px; }
  .key-btn:hover, .key-btn[aria-expanded="true"] { border-color: var(--accent); background: var(--accent-soft); }
  .key-btn.empty { border: 1px dashed var(--ink-3); padding: 2px 12px; color: var(--ink-2); }
  .scrim { position: fixed; inset: 0; z-index: 40; }
  .panel { position: fixed; z-index: 41; width: min(880px, calc(100vw - 16px)); padding: 12px; display: grid; gap: 10px; border-radius: 14px; background: var(--win-bg); border: 1px solid var(--line); box-shadow: 0 16px 44px rgba(40, 20, 40, .28); animation: panel-in .12s ease-out; }
  @keyframes panel-in { from { opacity: 0; transform: translateY(-3px); } }
  @media (prefers-reduced-motion: reduce) { .panel { animation: none; } .hint-live { animation: none; } }
  .head, .foot { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; min-height: 30px; }
  .grow { flex: 1; }
  .hint-live { color: var(--accent-ink); font-size: 12px; animation: breathe 1.8s ease-in-out infinite; }
  @keyframes breathe { 50% { opacity: .55; } }
  .views { display: inline-flex; gap: 2px; padding: 2px; border-radius: 8px; background: rgba(127,127,127,.12); }
  .views button { border: 0; background: none; border-radius: 6px; padding: 3px 10px; cursor: pointer; color: var(--ink-2); font-size: 12px; }
  .views button[aria-pressed="true"] { background: var(--win-bg); color: var(--ink); box-shadow: 0 1px 2px rgba(0,0,0,.15); }
  .more { display: grid; gap: 10px; padding: 4px 2px; min-height: 189px; align-content: start; }
  .more-row { display: flex; flex-wrap: wrap; gap: 4px; align-items: center; }
  .more-name { width: 88px; color: var(--ink-2); font-size: 12px; }
:global(:lang(zh-CN)) .more-name { width: 56px; }
  .pick { border: 0; cursor: pointer; margin: 0; }
  .pick + .pick { margin-left: 0; }
  .pick:hover:not(:disabled) { filter: brightness(.95); box-shadow: 0 0 0 2px var(--accent); }
  .pick.picked { --top: var(--accent-top); --side: var(--accent-side); --legend: #fff; }
  .pick:disabled { opacity: .35; cursor: default; }
</style>
