<script>
  import Icon from "../lib/Icon.svelte";
  import KbTabs from "../lib/KbTabs.svelte";
  import RuleRow from "../lib/RuleRow.svelte";
  import RuleEditor from "../lib/RuleEditor.svelte";
  import WhenPicker from "../lib/WhenPicker.svelte";
  import { config, edit } from "../lib/config.svelte.js";
  import { base, ensureDevice, ruleCount, ruleItems } from "../lib/model.js";
  import { fieldOp, plainWhen, whenText, whenWith } from "../lib/apps.svelte.js";
  import { t, tn } from "../lib/i18n.svelte.js";

  let { status, platform } = $props();

  let tab = $state("");
  let query = $state("");
  // What is being edited: { path } edits a rule; { add: container path } creates a rule; { group: path } edits a group; { newGroup: true }
  let open = $state(null);
  let drag = $state(null);
  let drop = $state(null);

  const fresh = $derived(tab.startsWith("new:"));
  const device = $derived(fresh ? "" : tab);
  const items = $derived(fresh ? [] : ruleItems(config.model, device));
  const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
  const isOpen = (path) => open?.path && same(open.path, path);

  $effect(() => {
    tab;
    open = null;
  });

  const matches = (rule) => {
    const q = query.trim().toLowerCase();
    if (!q) return true;
    return JSON.stringify(rule).toLowerCase().includes(q) || whenText(rule.when).toLowerCase().includes(q);
  };

  // Consecutive rules are drawn in one list, each group in its own section
  const chunks = $derived.by(() => {
    const out = [];
    for (const item of items) {
      if (item.kind === "group") {
        const rules = item.rules.filter((r) => matches(r.rule));
        if (rules.length || !query.trim()) out.push({ ...item, rules });
      } else if (matches(item.rule)) {
        const last = out.at(-1);
        if (last?.kind === "rules") last.rules.push(item);
        else out.push({ kind: "rules", rules: [item] });
      }
    }
    return out;
  });

  async function addRule(container, value) {
    const { device: name, ops } = ensureDevice(config.model, tab);
    const path = container ?? [...base(name), "rule"];
    const len = container ? container.reduce((node, seg) => node?.[seg], config.model)?.length ?? 0 : ruleItems(config.model, name).filter((i) => i.kind === "rule").length;
    if (await edit(...ops, { op: "insert", path, index: len, value })) {
      open = null;
      if (fresh) tab = name;
    }
  }

  async function saveRule(path, build) {
    const ops = build(path);
    if (!ops.length || (await edit(...ops))) open = null;
  }

  const removeRule = async (path) => {
    if (await edit({ op: "remove", path })) open = null;
  };
  const toggle = (path, item) =>
    edit(item.enabled === false ? { op: "remove", path: [...path, "enabled"] } : { op: "set", path: [...path, "enabled"], value: false });

  // Rule groups
  let groupName = $state("");
  let groupMode = $state("all");
  let groupApps = $state([]);
  function openGroup(item) {
    groupName = item?.group.name ?? "";
    groupMode = item?.group.when?.app?.length ? "only" : item?.group.when?.app_not?.length ? "except" : "all";
    groupApps = [...(item?.group.when?.app ?? item?.group.when?.app_not ?? [])];
    open = item ? { group: item.path } : { newGroup: true };
  }
  async function saveGroup(item) {
    const name = groupName.trim();
    if (!item) {
      const { device: dev, ops } = ensureDevice(config.model, tab);
      const value = { name: name || t("New group") };
      const when = whenWith(null, groupMode, groupApps);
      if (when) value.when = when;
      const count = items.filter((i) => i.kind === "group").length;
      if (await edit(...ops, { op: "insert", path: [...base(dev), "group"], index: count, value })) {
        open = null;
        if (fresh) tab = dev;
      }
      return;
    }
    const ops = [
      ...fieldOp(item.path, "name", item.group.name, name || undefined),
      ...fieldOp(item.path, "when", plainWhen(item.group.when), whenWith(item.group.when, groupMode, groupApps)),
    ];
    if (!ops.length || (await edit(...ops))) open = null;
  }

  // Drag to reorder: only within the same level (among top-level rules and groups, or inside one group)
  function grip(event, path, container) {
    event.preventDefault();
    drag = { path, container };
    window.addEventListener("pointermove", moved);
    window.addEventListener("pointerup", dropped, { once: true });
  }
  function moved(event) {
    const el = document.elementFromPoint(event.clientX, event.clientY)?.closest("[data-drop]");
    if (!el || el.dataset.container !== drag.container || el.dataset.drop === JSON.stringify(drag.path)) {
      drop = null;
      return;
    }
    const r = el.getBoundingClientRect();
    drop = { path: JSON.parse(el.dataset.drop), after: event.clientY > r.top + r.height / 2 };
  }
  async function dropped() {
    window.removeEventListener("pointermove", moved);
    const from = drag;
    const to = drop;
    drag = drop = null;
    if (to) await edit({ op: "place", path: from.path, anchor: to.path, after: to.after });
  }
  const dropClass = (path) =>
    (drag && same(drag.path, path) ? "dragging " : "") + (drop && same(drop.path, path) ? (drop.after ? "drop-after" : "drop-before") : "");

  const removeGroup = async (item) => {
    if (await edit({ op: "remove", path: item.path })) open = null;
  };
  const addOpenFor = (path) => open?.add && same(open.add, path);
</script>

<section class="page">
  <h1>{t("Rules")}</h1>
  <p class="lead">{t("Pick a keyboard, then edit its rules. Rules are checked from top to bottom and the first match wins. A small heading is a rule group, and the rules in a group share its condition and switch.")}</p>
  <KbTabs bind:value={tab} {status} count={(d) => ruleCount(config.model, d)} />

  <div class="toolbar">
    <input class="search" placeholder={t("Search rules, keys or apps")} bind:value={query} />
    <button class="btn" onclick={() => openGroup(null)}>{t("New rule group")}</button>
    <button class="btn primary" onclick={() => (open = { add: true })}><Icon name="plus" />{t("New rule")}</button>
  </div>

  {#if device}<p class="kb-note">{t("Applies to this keyboard only, and is checked before the “All keyboards” rules.")}</p>{/if}

  {#if open?.add === true}
    <div class="group-gap"><RuleEditor {platform} onSave={(value) => addRule(null, value)} onCancel={() => (open = null)} /></div>
  {/if}
  {#if open?.newGroup}
    <div class="editor group-gap">
      <div class="ke-row"><span class="lbl">{t("Group name")}</span><input type="text" placeholder={t("e.g. In the terminal")} bind:value={groupName} /></div>
      <WhenPicker bind:mode={groupMode} bind:apps={groupApps} />
      <div class="ke-actions"><button class="btn primary" onclick={() => saveGroup(null)}>{t("Create")}</button><button class="btn" onclick={() => (open = null)}>{t("Cancel")}</button></div>
    </div>
  {/if}

  {#if !items.length && !open}
    <div class="empty">
      <b>{fresh || device ? t("This keyboard has no rules of its own yet") : t("No rules yet")}</b>
      <span>{device || fresh ? t("Rules for “All keyboards” apply to it. To change only this keyboard, create a rule here.") : t("Rules can turn a key or combo into other keys, run a command, or apply only in certain apps.")}</span>
      <button class="btn primary" onclick={() => (open = { add: true })}>{t("New rule")}</button>
    </div>
  {/if}

  {#each chunks as chunk, c (chunk.kind === "group" ? JSON.stringify(chunk.path) : "rules" + c)}
    {#if chunk.kind === "rules"}
      <div class="rlist top-gap">
        {#each chunk.rules as item (JSON.stringify(item.path))}
          <RuleRow rule={item.rule} {platform} container="top" dropAttr={JSON.stringify(item.path)} classes={dropClass(item.path)}
            onGrip={(e) => grip(e, item.path, "top")} onOpen={() => (open = isOpen(item.path) ? null : { path: item.path })} onToggle={() => toggle(item.path, item.rule)} />
          {#if isOpen(item.path)}
            <RuleEditor rule={item.rule} {platform} onSave={(build) => saveRule(item.path, build)} onCancel={() => (open = null)} onDelete={() => removeRule(item.path)} />
          {/if}
        {/each}
      </div>
    {:else}
      {@const on = chunk.group.enabled !== false}
      <div class="rsec rule-head {dropClass(chunk.path)}" data-drop={JSON.stringify(chunk.path)} data-container="top">
        <span class="grip" role="presentation" title={t("Drag to reorder")} onpointerdown={(e) => grip(e, chunk.path, "top")}><Icon name="grip" /></span>
        <span class="title">{chunk.group.name ?? t("Rule group")}</span>
        {#if whenText(chunk.group.when)}<span class="badge">{whenText(chunk.group.when)}</span>{/if}
        <span class="grow"></span>
        <button class="link" onclick={() => (open = { add: chunk.path })}>{t("Add rule")}</button>
        <button class="link" onclick={() => openGroup(chunk)}>{t("Edit")}</button>
        <button class="switch" role="switch" aria-checked={on} aria-label={t("Enable {name}", { name: chunk.group.name ?? t("Rule group") })} onclick={() => toggle(chunk.path, chunk.group)}></button>
      </div>
      {#if open?.group && same(open.group, chunk.path)}
        <div class="editor group-gap">
          <div class="ke-row"><span class="lbl">{t("Group name")}</span><input type="text" bind:value={groupName} /></div>
          <WhenPicker bind:mode={groupMode} bind:apps={groupApps} />
          <div class="ke-actions">
            <button class="btn primary" onclick={() => saveGroup(chunk)}>{t("Save")}</button>
            <button class="btn" onclick={() => (open = null)}>{t("Cancel")}</button>
            <span class="grow"></span>
            <button class="btn" onclick={() => removeGroup(chunk)}>{t("Delete this group and its rules")}</button>
          </div>
        </div>
      {/if}
      {#if addOpenFor(chunk.path)}
        <div class="group-gap"><RuleEditor {platform} onSave={(value) => addRule([...chunk.path, "rule"], value)} onCancel={() => (open = null)} /></div>
      {/if}
      {#if chunk.rules.length}
        <div class="rlist" class:group-off={!on}>
          {#each chunk.rules as item (JSON.stringify(item.path))}
            <RuleRow rule={item.rule} {platform} container={JSON.stringify(chunk.path)} dropAttr={JSON.stringify(item.path)} classes={dropClass(item.path)}
              onGrip={(e) => grip(e, item.path, JSON.stringify(chunk.path))} onOpen={() => (open = isOpen(item.path) ? null : { path: item.path })} onToggle={() => toggle(item.path, item.rule)} />
            {#if isOpen(item.path)}
              <RuleEditor rule={item.rule} {platform} onSave={(build) => saveRule(item.path, build)} onCancel={() => (open = null)} onDelete={() => removeRule(item.path)} />
            {/if}
          {/each}
        </div>
      {/if}
    {/if}
  {/each}

  {#if device}
    {@const inherit = config.model?.device?.[device]?.inherit !== false}
    <div class="kb-inherit">
      <span>{inherit ? tn(ruleCount(config.model, ""), "{n} rule from “All keyboards” also applies to this keyboard.", "{n} rules from “All keyboards” also apply to this keyboard.") : t("This keyboard uses only its own rules and remaps.")}</span>
      <span class="grow"></span>
      <button class="switch" role="switch" aria-checked={inherit} aria-label={t("Also use the “All keyboards” rules on this keyboard")}
        onclick={() => edit(inherit ? { op: "set", path: ["device", device, "inherit"], value: false } : { op: "remove", path: ["device", device, "inherit"] })}></button>
    </div>
  {/if}
  {#if items.length}<p class="order-hint">{t("Drag the handle on the left to reorder. Click a rule to change it.")}</p>{/if}
</section>

<style>
  .top-gap { margin-top: 12px; }
  .group-gap { margin-bottom: 12px; }
  .rlist.group-off { opacity: .5; }
  .rule-head { position: relative; }
  .rule-head .grip { color: var(--ink-3); cursor: grab; display: grid; place-items: center; touch-action: none; }
  .rule-head .grip :global(.i) { width: 16px; height: 16px; }
  .rule-head.dragging { opacity: .4; }
  .rule-head.drop-before::before, .rule-head.drop-after::after { content: ""; position: absolute; left: 0; right: 0; height: 2px; background: var(--accent); border-radius: 2px; }
  .rule-head.drop-before::before { top: -3px; }
  .rule-head.drop-after::after { bottom: -3px; }
</style>
