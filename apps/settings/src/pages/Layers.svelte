<script>
  import { onDestroy, onMount, untrack } from "svelte";
  import Icon from "../lib/Icon.svelte";
  import Expr from "../lib/Expr.svelte";
  import Seg from "../lib/Seg.svelte";
  import KeyField from "../lib/KeyField.svelte";
  import { config, edit } from "../lib/config.svelte.js";
  import Board from "../lib/Board.svelte";
  import { startCapture } from "../lib/capture.js";
  import { bottomRow, confirmedCount, confirmKey, isConfirmed, keyboards, resetSwaps, setBottomRow } from "../lib/keyboards.svelte.js";
  import { boardKeyCount } from "../lib/layout.js";
  import { boardLegend, chordCaps, legend } from "../lib/keys.js";
  import { onKey } from "../lib/live.js";
  import { layerNames } from "../lib/model.js";
  import { N_, t } from "../lib/i18n.svelte.js";

  let { platform, calibrateRequest = 0, onGoto } = $props();

  let current = $state(""); // "" is the base layer
  let selected = $state(null);
  let pressed = $state(new Set());
  let adding = $state(false);

  onMount(() =>
    onKey((e) => {
      if (e.type !== "key") return;
      const next = new Set(pressed);
      if (e.input.state === "down") next.add(e.input.key);
      else if (e.input.state === "up") next.delete(e.input.key);
      else return;
      pressed = next;
    }),
  );

  const names = $derived(layerNames(config.model));
  const layers = $derived(config.model?.layer ?? {});
  const layer = $derived(current ? layers[current] : null);

  $effect(() => {
    if (current && !layer) current = "";
  });

  const short = (expr) => {
    if (Array.isArray(expr)) return "…";
    const caps = chordCaps(String(expr), platform);
    return caps.length === 1 ? boardLegend(String(expr), platform) : caps.map((c) => c.label).join("");
  };
  const has = (obj) => Object.keys(obj ?? {}).some((k) => k !== "$line");

  // What a key looks like in the base layer now: a remap, a rule with no modifiers or conditions, or a layer trigger
  function baseOf(id) {
    const trigger = names.find((n) => layers[n].hold === id);
    if (trigger) return { type: "layer", layer: trigger, tap: layers[trigger].tap };
    const remap = config.model?.remap?.[id];
    if (remap !== undefined) return { type: remap === "none" ? "off" : "keys", to: remap, source: "remap" };
    const rules = config.model?.rule ?? [];
    const index = rules.findIndex((r) => r.from === id && r.enabled !== false && !has(r.when));
    if (index < 0) return { type: "same" };
    const rule = rules[index];
    const path = ["rule", index];
    if (rule.hold !== undefined) return { type: "taphold", tap: rule.tap, hold: rule.hold, path };
    if (rule.run !== undefined || rule.call !== undefined) return { type: "other", path };
    return { type: rule.to === "none" ? "off" : "keys", to: rule.to, path };
  }

  function view(id) {
    if (!current) {
      const b = baseOf(id);
      if (b.type === "layer") return { cls: "trig", main: boardLegend(id, platform), small: b.layer };
      if (b.type === "taphold") return { cls: "set th", main: boardLegend(id, platform), bold: short(b.hold), tap: short(b.tap ?? id) };
      if (b.type === "keys") return { cls: "set", main: boardLegend(id, platform), bold: short(b.to) };
      if (b.type === "off") return { cls: "set", main: boardLegend(id, platform), bold: t("Disabled") };
      if (b.type === "other") return { cls: "set", main: boardLegend(id, platform), bold: t("Rule") };
      return { cls: "", main: boardLegend(id, platform) };
    }
    if (layer.hold === id) return { cls: "trig", main: boardLegend(id, platform), small: t("Holding") };
    const m = layer.map?.[id];
    if (m === "none") return { cls: "set", main: boardLegend(id, platform), bold: t("Disabled") };
    if (m !== undefined) return { cls: "set", main: boardLegend(id, platform), bold: short(m) };
    const b = baseOf(id);
    return { cls: "thru", through: b.to && b.to !== "none" ? short(b.to) : b.hold ? short(b.hold) : boardLegend(id, platform) };
  }

  // ---- Calibration: positions on the diagram follow a standard keyboard. If one is off, click it, then press the key at that position on your keyboard, and the two keys swap ----
  const orders = $derived([{ value: "mac", label: t("Mac layout") }, { value: "win", label: t("Windows layout") }]);
  let calibrating = $state(false);
  let target = $state(null); // the key being calibrated
  let stopCapture = null;
  let note = $state(""); // result of the last calibration
  let flashed = $state([]); // the two keys just swapped, flashed briefly
  let flashTimer;

  const swaps = $derived(keyboards.swaps[keyboards.device] ?? {});
  // Calibrated keys; a swapped pair is listed once
  const fixes = $derived(
    Object.entries(swaps)
      .filter(([base, shown]) => swaps[shown] !== base || base < shown)
      .map(([base, shown]) => ({ base, shown, both: swaps[shown] === base })),
  );

  // A key pressed in calibration mode: with no position selected, the key lights up on the diagram and counts as calibrated; with a position selected, that position is this key
  function pressedKey(key) {
    const id = target;
    flashed = [key];
    clearTimeout(flashTimer);
    flashTimer = setTimeout(() => (flashed = []), 1400);
    if (!id) {
      confirmKey(key, key);
      note = "";
      return;
    }
    const a = boardLegend(id, platform);
    const b = boardLegend(key, platform);
    note = key === id ? t("This position is correct: {key}.", { key: a }) : t("Swapped: this position is now {b}, and {a} moved to where {b} was.", { a, b });
    confirmKey(id, key);
    target = null;
  }

  function endCapture() {
    target = null;
    stopCapture?.();
    stopCapture = null;
    window.removeEventListener("blur", leaveCalibrating);
  }

  // Leave when switching to another window, so remapping is not left paused
  function leaveCalibrating() {
    endCapture();
    calibrating = false;
    note = "";
  }

  function toggleCalibrating() {
    selected = null;
    note = "";
    if (calibrating) return leaveCalibrating();
    calibrating = true;
    stopCapture = startCapture({ chord: false, plain: false, onHeld: () => {}, onPressed: () => {}, onDone: pressedKey });
    window.addEventListener("blur", leaveCalibrating);
  }

  function aim(id) {
    target = target === id ? null : id;
    note = "";
  }

  onDestroy(endCapture);

  // Go straight into calibration when the new keyboard notice says "Calibrate"
  let handledRequest = untrack(() => calibrateRequest);
  onMount(() => {
    if (calibrateRequest && !calibrating) toggleCalibrating();
    handledRequest = calibrateRequest;
  });
  $effect(() => {
    if (calibrateRequest !== handledRequest) {
      handledRequest = calibrateRequest;
      if (!calibrating) toggleCalibrating();
    }
  });

  // ---- Key editing ----
  let kind = $state("same");
  let to = $state("");
  let tap = $state("");
  let hold = $state("");
  let targetLayer = $state("");

  function select(id) {
    selected = selected === id ? null : id;
    if (!selected) return;
    if (current) {
      const m = layer.map?.[id];
      kind = m === undefined ? "same" : m === "none" ? "off" : "keys";
      to = m && m !== "none" ? m : "";
    } else {
      const b = baseOf(id);
      kind = b.type;
      to = b.to && b.to !== "none" ? b.to : "";
      tap = b.tap ?? "";
      hold = b.hold ?? "";
      targetLayer = b.layer ?? names[0] ?? "";
    }
  }

  const selectedBase = $derived(selected && !current ? baseOf(selected) : null);
  const kinds = $derived(
    current
      ? [{ value: "same", label: t("▽ Transparent") }, { value: "keys", label: t("Keys") }, { value: "off", label: t("Disabled") }]
      : [{ value: "same", label: t("Keep as is") }, { value: "keys", label: t("Keys") }, { value: "taphold", label: t("Tap / hold") },
         ...(names.length ? [{ value: "layer", label: t("Enter layer") }] : []), { value: "off", label: t("Disabled") }],
  );

  // Clear the key's current setting in the base layer
  function clearBase(id) {
    const b = baseOf(id);
    if (b.source === "remap") return [{ op: "remove", path: ["remap", id] }];
    if (b.path) return [{ op: "remove", path: b.path }];
    return [];
  }

  async function saveKey() {
    const id = selected;
    let ops;
    if (current) {
      const path = ["layer", current, "map", id];
      if (kind === "same") ops = layer.map?.[id] !== undefined ? [{ op: "remove", path }] : [];
      else if (kind === "off") ops = [{ op: "set", path, value: "none" }];
      else {
        if (!to) return;
        ops = [{ op: "set", path, value: to }];
      }
    } else {
      ops = clearBase(id);
      if (kind === "keys") {
        if (!to) return;
        ops.push({ op: "set", path: ["remap", id], value: to });
      } else if (kind === "off") {
        ops.push({ op: "set", path: ["remap", id], value: "none" });
      } else if (kind === "taphold") {
        if (!hold) return;
        const value = { from: id, hold };
        if (tap) value.tap = tap;
        const count = (config.model?.rule?.length ?? 0) - ops.filter((o) => o.path[0] === "rule").length;
        ops.push({ op: "insert", path: ["rule"], index: count, value });
      } else if (kind === "layer") {
        if (!targetLayer) return;
        ops.push({ op: "set", path: ["layer", targetLayer, "hold"], value: id });
        if (layers[targetLayer].toggle !== undefined) ops.push({ op: "remove", path: ["layer", targetLayer, "toggle"] });
      }
    }
    if (!ops.length || (await edit(...ops))) selected = null;
  }

  // ---- The layers themselves ----
  let name = $state("");
  let mode = $state("hold");
  let trigger = $state("");
  let problem = $state("");

  async function addLayer() {
    const n = name.trim();
    if (!n) return void (problem = N_("Name the layer."));
    if (names.includes(n)) return void (problem = N_("That name is already taken."));
    if (!trigger) return void (problem = mode === "hold" ? N_("Choose the key to hold to enter the layer.") : N_("Choose the key that toggles the layer."));
    if (await edit({ op: "set", path: ["layer", n], value: { [mode]: trigger, map: {} }, table: true })) {
      adding = false;
      current = n;
      name = trigger = problem = "";
    }
  }

  // Change the current layer's trigger key
  let triggerDraft = $state("");
  $effect(() => {
    triggerDraft = layer ? (layer.hold ?? layer.toggle ?? "") : "";
  });
  $effect(() => {
    const value = triggerDraft;
    if (!layer || !value) return;
    const key = layer.hold !== undefined ? "hold" : "toggle";
    if (value !== layer[key]) edit({ op: "set", path: ["layer", current, key], value });
  });

  let confirmDelete = $state(false);
  async function removeLayer() {
    if (await edit({ op: "remove", path: ["layer", current] })) {
      current = "";
      confirmDelete = false;
    }
  }

  const how = (l) => (l.hold !== undefined ? t("Hold {key} to enter", { key: short(l.hold) }) : t("Press {key} to toggle", { key: short(l.toggle) }));
</script>

<section class="page">
  <h1>{t("Layers")}</h1>
  <p class="lead">{t("The keyboard shows what each key becomes right now. Click a key to change it; keys you press light up live.")}</p>

  <div class="layer-tabs" role="tablist" aria-label={t("Layers")}>
    <button class="ltab" role="tab" aria-selected={current === ""} onclick={() => { current = ""; selected = null; }}>{t("Base layer")}</button>
    {#each names as n (n)}
      <button class="ltab" role="tab" aria-selected={current === n} title={how(layers[n])} onclick={() => { current = n; selected = null; confirmDelete = false; }}>
        {#if has(layers[n].map)}<span class="dot"></span>{/if}{n}
      </button>
    {/each}
    <button class="ltab add" onclick={() => (adding = !adding)}><Icon name="plus" />{t("New layer")}</button>
  </div>

  {#if adding}
    <div class="editor" style="margin-bottom:12px">
      <div class="ke-row"><span class="lbl">{t("Name")}</span><input type="text" spellcheck="false" placeholder={t("e.g. nav")} bind:value={name} /></div>
      <div class="ke-row"><span class="lbl">{t("Trigger")}</span><Seg bind:value={mode} options={[{ value: "hold", label: t("Active while held") }, { value: "toggle", label: t("Toggle on press") }]} /></div>
      <div class="ke-row"><span class="lbl"></span><KeyField bind:value={trigger} {platform} mods={mode === "toggle"} /></div>
      {#if mode === "hold"}<div class="ke-row"><span class="lbl"></span><span class="hint">{t("A tap on this key is only sent on release, and holding it no longer repeats. Choose a key you rarely tap, such as Caps Lock.")}</span></div>{/if}
      {#if problem}<div class="ke-row" style="color:var(--err)"><span class="lbl"></span>{t(problem)}</div>{/if}
      <div class="ke-actions"><button class="btn primary" onclick={addLayer}>{t("Create")}</button><button class="btn" onclick={() => (adding = false)}>{t("Cancel")}</button></div>
    </div>
  {/if}

  {#if layer}
    <div class="ke-row layer-how">
      <span>{layer.hold !== undefined ? t("Hold") : t("Toggle key")}</span>
      <KeyField bind:value={triggerDraft} {platform} mods={layer.hold === undefined} />
      <span class="grow"></span>
      {#if confirmDelete}
        <span style="color:var(--err);font-size:12px">{t("Delete this layer?")}</span>
        <button class="btn" onclick={removeLayer}>{t("Delete")}</button>
        <button class="btn" onclick={() => (confirmDelete = false)}>{t("Cancel")}</button>
      {:else}
        <button class="btn" onclick={() => (confirmDelete = true)}>{t("Delete layer")}</button>
      {/if}
    </div>
  {/if}

  <div class="board-bar">
    {#if calibrating}
      <span class="cal-hint">{target ? t("Now press the key at this position on your keyboard…") : note || t("Press every key on your keyboard once and the keys you hit turn green. If a lit position is wrong, click the correct position on the diagram, then press that key.")}</span>
      <span class="grow"></span>
      <button class="btn primary" onclick={toggleCalibrating}>{t("Done")}</button>
    {:else}
      <span>{keyboards.device || t("This keyboard")}</span>
      <span class="grow"></span>
      <Seg bind:value={() => bottomRow(platform), setBottomRow} options={orders} />
      <button class="btn" onclick={toggleCalibrating}>{t("Calibrate")}</button>
    {/if}
  </div>
  <Board {platform}>
    {#snippet key(id, style)}
      {@const v = view(id)}
      <button class="k {v.cls}" class:sel={selected === id || target === id} class:down={pressed.has(id)} class:ok={calibrating && isConfirmed(id)} class:okmark={!calibrating && isConfirmed(id)} class:todo={calibrating && !isConfirmed(id) && target !== id} class:flash={flashed.includes(id)} {style} onclick={() => (calibrating ? aim(id) : select(id))} aria-label={legend(id, platform)}>
        {#if v.through}<i>▽</i>{v.through}
        {:else}
          {#if v.tap}<span class="tap">{v.tap}</span>{/if}{v.main}
          {#if v.bold}<b>{v.bold}</b>{/if}
          {#if v.small}<small>{v.small}</small>{/if}
        {/if}
      </button>
    {/snippet}
  </Board>

  {#if confirmedCount() || fixes.length}
    {@const keyboard = keyboards.device ? `${keyboards.device} ` : t("This keyboard")}
    <div class="cal-list">
      <span>{fixes.length
        ? t("{keyboard} has {done} of {total} keys calibrated. Moved keys:", { keyboard, done: confirmedCount(), total: boardKeyCount(platform) })
        : t("{keyboard} has {done} of {total} keys calibrated.", { keyboard, done: confirmedCount(), total: boardKeyCount(platform) })}</span>
      {#if fixes.length}
        {#each fixes as fix (fix.base)}
          <span class="fix">{boardLegend(fix.base, platform)}{fix.both ? " ⇄ " : " → "}{boardLegend(fix.shown, platform)}</span>
        {/each}
      {/if}
      <button class="link" onclick={() => { target = null; note = ""; resetSwaps(); }}>{t("Restore defaults")}</button>
    </div>
  {/if}

  <div class="key-legend">
    <span><span class="sw" style="background:var(--accent-top)"></span>{t("Changed")}</span>
    {#if current}<span><span class="sw" style="box-shadow:inset 0 0 0 1px var(--ink-3)"></span>{t("▽ Transparent, uses the base layer")}</span>{/if}
    <span><span class="sw" style="background:#FFFDFC;box-shadow:inset 0 0 0 2px var(--accent)"></span>{t("Layer key")}</span>
    {#if calibrating || confirmedCount()}<span><span class="sw" style="background:#4E9A6A"></span>{t("Calibrated")}</span>{/if}
    {#if calibrating}<span><span class="sw" style="background:#5A4766;opacity:.45"></span>{t("Not calibrated")}</span>{/if}
  </div>

  {#if selected}
    {@const [before, after] = t("In {layer}", { layer: "\u0001" }).split("\u0001")}
    <div class="editor">
      <div class="ke-row"><Expr value={selected} {platform} /><span>{before}<b>{current || t("Base layer")}</b>{after}</span></div>
      {#if selectedBase?.type === "layer"}
        <div class="ke-row"><span class="hint">{t("This key is the trigger for layer {layer}.", { layer: selectedBase.layer })}</span><button class="link" onclick={() => { current = selectedBase.layer; selected = null; }}>{t("Go to that layer to change it")}</button></div>
      {:else if selectedBase?.type === "other"}
        <div class="ke-row"><span class="hint">{t("This key is controlled by a rule.")}</span><button class="link" onclick={() => onGoto("rules")}>{t("Go to Rules to change it")}</button></div>
      {:else}
        <div class="ke-row"><Seg bind:value={kind} options={kinds} /></div>
        {#if kind === "keys"}
          <div class="ke-row"><span class="lbl">{t("Becomes")}</span><KeyField bind:value={to} {platform} mods={!!current} targets out /></div>
        {:else if kind === "taphold"}
          <div class="ke-row"><span class="lbl">{t("Tap")}</span><KeyField bind:value={tap} {platform} mods targets none clearable out placeholder={t("Same as the original key")} /></div>
          <div class="ke-row"><span class="lbl">{t("Hold")}</span><KeyField bind:value={hold} {platform} mods targets out /></div>
        {:else if kind === "layer"}
          <div class="ke-row"><span class="lbl">{t("Layer")}</span>
            <select aria-label={t("Layer to enter")} bind:value={targetLayer}>{#each names as n (n)}<option value={n}>{n}</option>{/each}</select>
          </div>
        {/if}
        <div class="ke-actions"><button class="btn primary" onclick={saveKey}>{t("Save")}</button><button class="btn" onclick={() => (selected = null)}>{t("Cancel")}</button></div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .layer-how { margin-bottom: 12px; color: var(--ink-2); }
  .cal-hint { color: var(--accent-ink); font-weight: 600; }
  .cal-list { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; color: var(--ink-2); font-size: 12px; }
  .fix { padding: 2px 9px; border-radius: 999px; background: color-mix(in srgb, var(--warn) 15%, transparent); color: var(--ink); font: 600 12px var(--legend-font); }
</style>
