<script>
  import Icon from "../lib/Icon.svelte";
  import KeyField from "../lib/KeyField.svelte";
  import KbTabs from "../lib/KbTabs.svelte";
  import { config, edit } from "../lib/config.svelte.js";
  import { base, ensureDevice, remapEntries } from "../lib/model.js";
  import { t } from "../lib/i18n.svelte.js";

  let { status, platform } = $props();

  let tab = $state("");
  let draft = $state(null); // the row being added, saved once both sides are picked

  const fresh = $derived(tab.startsWith("new:"));
  const device = $derived(fresh ? "" : tab);
  const entries = $derived(fresh ? [] : remapEntries(config.model, device));
  const used = $derived(entries.map(([key]) => key));

  $effect(() => {
    tab;
    draft = null;
  });

  const path = (key) => [...base(device), "remap", key];

  function changeFrom(key, next) {
    if (next && next !== key) edit({ op: "rename", path: path(key), to: next });
  }

  function changeTo(key, next) {
    if (next) edit({ op: "set", path: path(key), value: next });
  }

  $effect(() => {
    if (draft?.from && draft?.to) addDraft(draft.from, draft.to);
  });

  async function addDraft(from, to) {
    draft = null;
    const { device: name, ops } = ensureDevice(config.model, tab);
    if ((await edit(...ops, { op: "set", path: [...base(name), "remap", from], value: to })) && fresh) tab = name;
  }
</script>

<section class="page">
  <h1>{t("Remaps")}</h1>
  <p class="lead">{t("Always replace one key with another, for example to swap two modifier keys. Changes apply as soon as you pick both keys, and rules and layers see the remapped keys.")}</p>
  <KbTabs bind:value={tab} {status} count={(d) => remapEntries(config.model, d).length} />

  <div class="toolbar">
    <span class="grow"></span>
    <button class="btn primary" onclick={() => (draft = { from: "", to: "" })}><Icon name="plus" />{t("Add remap")}</button>
  </div>

  {#if entries.length || draft}
    <div class="rlist">
      {#each entries as [key, target] (key)}
        <div class="rule plain">
          <span class="line">
            <KeyField value={key} {platform} exclude={used} onpick={(next) => changeFrom(key, next)} />
            <span class="becomes"></span>
            <KeyField value={target} {platform} targets none out onpick={(next) => changeTo(key, next)} />
          </span>
          <button class="icon-btn" aria-label={t("Reset {key}", { key })} title={t("Reset")} onclick={() => edit({ op: "remove", path: path(key) })}><Icon name="trash" /></button>
        </div>
      {/each}
      {#if draft}
        <div class="rule plain">
          <span class="line">
            <KeyField bind:value={draft.from} {platform} exclude={used} placeholder={t("This key")} />
            <span class="becomes"></span>
            <KeyField bind:value={draft.to} {platform} targets none out placeholder={t("Becomes")} />
          </span>
          <button class="icon-btn" aria-label={t("Cancel adding remap")} title={t("Cancel")} onclick={() => (draft = null)}><Icon name="close" /></button>
        </div>
      {/if}
    </div>
  {:else}
    <div class="empty">
      <b>{tab ? t("This keyboard has no remaps of its own yet") : t("No remaps yet")}</b>
      <span>{t("All keys are unchanged for now.")}</span>
      <button class="btn primary" onclick={() => (draft = { from: "", to: "" })}>{t("Add remap")}</button>
    </div>
  {/if}
</section>

<style>
  .rule.plain { grid-template-columns: 1fr auto; }
  .line { display: flex; align-items: center; flex-wrap: wrap; gap: 2px; }
</style>
