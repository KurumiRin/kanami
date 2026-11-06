<script>
  import Icon from "../lib/Icon.svelte";
  import Cap from "../lib/Cap.svelte";
  import Seg from "../lib/Seg.svelte";
  import { legend } from "../lib/keys.js";
  import { language, t, tn } from "../lib/i18n.svelte.js";

  // events: records the app received (up to 5000), each with the time `at` it was received and the foreground app at that moment
  let { events, platform, connected, onClear } = $props();

  let onlyChanged = $state(true);
  let frozen = $state(null); // the list frozen while the display is paused
  let query = $state("");

  const ROW = 44;
  let box = $state();
  let height = $state(400);
  let top = $state(0);
  let follow = true;

  const changed = (e) =>
    e.text || e.run || e.waiting || e.output.length !== 1 ||
    e.output[0].key !== e.input.key || e.output[0].state !== e.input.state;

  const rows = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return (frozen ?? events).filter((e) => {
      if (e.type === "skipped") return !q;
      if (e.input.state === "repeat") return false;
      if (onlyChanged && !changed(e)) return false;
      if (!q) return true;
      const keys = [e.input.key, ...e.output.map((o) => o.key)];
      return keys.some((k) => k.includes(q) || legend(k, platform).toLowerCase().includes(q)) ||
        (e.app ?? "").toLowerCase().includes(q) || (e.device ?? "").toLowerCase().includes(q);
    });
  });

  const first = $derived(Math.max(0, Math.floor(top / ROW) - 5));
  const visible = $derived(rows.slice(first, first + Math.ceil(height / ROW) + 10));

  // When a new event arrives, follow it if scrolled to the bottom; leave the view alone when scrolled up
  $effect(() => {
    rows.length;
    if (box && follow && !frozen) queueMicrotask(() => (box.scrollTop = box.scrollHeight));
  });

  function onscroll() {
    top = box.scrollTop;
    follow = box.scrollTop + box.clientHeight >= box.scrollHeight - ROW;
  }

  const time = (at) =>
    new Date(at).toLocaleTimeString(language(), { hourCycle: "h23", hour: "2-digit", minute: "2-digit", second: "2-digit", fractionalSecondDigits: 3 });
  const arrow = (state) => (state === "down" ? "↓" : state === "up" ? "↑" : "↻");
</script>

<section class="page events-page">
  <h1>{t("Event Viewer")}</h1>
  <p class="lead">{t("See what kanami turns each key into. Only keys pressed after you open the viewer are recorded.")}</p>
  <div class="toolbar">
    <button class="btn" onclick={() => (frozen = frozen ? null : [...events])}>
      <Icon name={frozen ? "play" : "pause"} />{frozen ? t("Resume") : t("Pause")}
    </button>
    <button class="btn" onclick={() => { frozen = null; onClear(); }}>{t("Clear")}</button>
    <Seg bind:value={onlyChanged} options={[{ value: true, label: t("Changed keys only") }, { value: false, label: t("All keys") }]} />
    <input class="search" placeholder={t("Key, app or keyboard")} bind:value={query} />
  </div>

  {#if !connected}
    <div class="banner"><Icon name="alert" /><span>{t("Remapping isn't running, so no key events arrive.")}</span></div>
  {/if}

  <div class="table">
    <div class="head"><span>{t("Time")}</span><span>{t("Pressed")}</span><span>{t("Becomes")}</span><span>{t("From")}</span></div>
    <div class="body" bind:this={box} bind:clientHeight={height} {onscroll}>
      <div style="height:{rows.length * ROW}px; position:relative">
        {#each visible as e, i (first + i)}
          <div class="tr" class:skip={e.type === "skipped"} style="top:{(first + i) * ROW}px">
            {#if e.type === "skipped"}
              <span class="skipped">{tn(e.count, "Skipped {n} event (the window couldn't keep up)", "Skipped {n} events (the window couldn't keep up)")}</span>
            {:else}
              <span class="t">{time(e.at)}</span>
              <span><span class="ud">{arrow(e.input.state)}</span><Cap name={e.input.key} {platform} /></span>
              <span class="out">
                {#if e.waiting}<span class="wait">{t("Waiting for release or next key")}</span>
                {:else if e.output.length === 0 && !e.text && !e.run}<span class="wait">{t("Swallowed")}</span>
                {:else}
                  {#each e.output as o}<span class="ud">{arrow(o.state)}</span><Cap name={o.key} {platform} out={changed(e)} />{/each}
                  {#if e.more}<span class="wait">…</span>{/if}
                  {#if e.text}<span class="tag">{t("Text")}</span>{/if}
                  {#if e.run}<span class="tag">{t("Command")}</span>{/if}
                {/if}
              </span>
              <span class="src">{[e.device, e.app].filter(Boolean).join(t(", "))}</span>
            {/if}
          </div>
        {/each}
      </div>
      {#if rows.length === 0}
        <p class="empty">{connected ? t("Press a few keys to try it.") : ""}</p>
      {/if}
    </div>
  </div>
  <p class="privacy"><Icon name="lock" /><span>{t("Key records stay in memory only: the latest 5000 at most, cleared when you close the window and never written to disk. The system doesn't pass input in password fields to kanami. “All keys” shows ordinary typing, so take care when recording or sharing your screen.")}</span></p>
</section>

<style>
  .events-page { display: flex; flex-direction: column; height: 100%; }
  .table { flex: 1; min-height: 0; display: flex; flex-direction: column; border-radius: var(--radius-group); background: var(--group-bg); overflow: hidden; }
  .head, .tr { display: grid; grid-template-columns: 110px minmax(90px, 1fr) minmax(160px, 2fr) minmax(90px, 1fr); gap: 10px; align-items: center; padding: 0 14px; }
  .head { color: var(--ink-3); font-size: 12px; height: 32px; border-bottom: 1px solid var(--line); }
  .body { flex: 1; overflow: auto; position: relative; }
  .tr { position: absolute; left: 0; right: 0; height: 44px; border-bottom: 1px solid var(--line); }
  .tr.skip { display: block; text-align: center; line-height: 44px; color: var(--ink-3); font-size: 12px;
    background: repeating-linear-gradient(135deg, transparent 0 6px, rgba(127,127,127,.06) 6px 12px); }
  .t { color: var(--ink-3); font-variant-numeric: tabular-nums; font-size: 12px; }
  .ud { font-size: 10px; color: var(--ink-3); margin: 0 3px 0 6px; }
  .ud:first-child { margin-left: 0; }
  .out { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .wait { color: var(--ink-3); font-size: 12px; }
  .tag { font-size: 11px; padding: 1px 8px; border-radius: 999px; background: var(--accent-soft); color: var(--accent-ink); margin-left: 6px; }
  .src { color: var(--ink-2); font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .empty { position: absolute; inset: 0; display: grid; place-items: center; color: var(--ink-3); margin: 0; pointer-events: none; }
  .privacy { display: flex; gap: 8px; align-items: flex-start; color: var(--ink-3); font-size: 12px; margin: 12px 2px 0; }
  .privacy :global(.i) { width: 15px; height: 15px; margin-top: 1px; }
</style>
