<script>
  import { api } from "../lib/api.js";
  import { config, edit } from "../lib/config.svelte.js";
  import { deviceNames, isKeyboard, ruleCount, remapEntries, slug } from "../lib/model.js";
  import { t, tn } from "../lib/i18n.svelte.js";

  let { status } = $props();

  let connected = $state([]);
  let confirming = $state(null);
  let renaming = $state(null);
  let newName = $state("");

  $effect(() => {
    status;
    api.devices().then((d) => (connected = d.filter(isKeyboard))).catch(() => (connected = []));
  });

  const hex = (n) => "0x" + n.toString(16).padStart(4, "0");
  const matches = (spec, d) =>
    (spec.vendor_id === undefined || spec.vendor_id === d.vendor_id) &&
    (spec.product_id === undefined || spec.product_id === d.product_id) &&
    (spec.product === undefined || d.product.includes(spec.product));

  const named = $derived(deviceNames(config.model).map((name) => {
    const spec = config.model.device[name];
    return { name, spec, on: connected.some((d) => matches(spec, d)) };
  }));
  const others = $derived.by(() => {
    const seen = new Set();
    return connected.filter((d) => !named.some((n) => matches(n.spec, d)) && !seen.has(d.product) && seen.add(d.product));
  });

  const add = (d) =>
    edit({ op: "set", path: ["device", slug(d.product, deviceNames(config.model))], value: { product: d.product }, table: true });

  async function remove(name) {
    if (await edit({ op: "remove", path: ["device", name] })) confirming = null;
  }

  async function rename(name) {
    const to = newName.trim();
    if (to && to !== name && !(await edit({ op: "rename", path: ["device", name], to }))) return;
    renaming = null;
  }

  const how = (spec) =>
    [spec.product && t("product name contains “{name}”", { name: spec.product }), spec.vendor_id !== undefined && t("vendor {id}", { id: hex(spec.vendor_id) }), spec.product_id !== undefined && t("product {id}", { id: hex(spec.product_id) })]
      .filter(Boolean).join(t(", "));
  const usage = (name) => {
    const r = ruleCount(config.model, name);
    const m = remapEntries(config.model, name).length;
    return r || m ? t("{rules}, {remaps}", { rules: tn(r, "{n} rule", "{n} rules"), remaps: tn(m, "{n} remap", "{n} remaps") }) : t("No rules of its own yet");
  };
  const detail = (name, spec, on) => {
    const status = on ? t("Connected") : t("Disconnected");
    const matching = how(spec);
    return matching
      ? t("{status}. Match: {how}. {usage}", { status, how: matching, usage: usage(name) })
      : t("{status}. {usage}", { status, usage: usage(name) });
  };
</script>

<section class="page">
  <h1>{t("Keyboards")}</h1>
  <p class="lead">{t("Different keyboards can have different rules and remaps. Connected keyboards appear as tabs on the Rules and Remaps pages and are added here the first time you add a rule for them. Rules for disconnected keyboards are kept and apply again when they reconnect.")}</p>

  {#if !status}
    <div class="banner"><span>{t("Remapping isn't running, so connected keyboards can't be listed.")}</span></div>
  {/if}

  {#if named.length}
    <h2 class="group-title">{t("Added")}</h2>
    <div class="group">
      {#each named as { name, spec, on } (name)}
        <div class="row">
          <span class="led" class:on title={on ? t("Connected") : t("Disconnected")}></span>
          <div class="main">{spec.product ?? name}<small>{detail(name, spec, on)}</small></div>
          {#if renaming === name}
            <input class="name" spellcheck="false" aria-label={t("Name in the config")} bind:value={newName} onkeydown={(e) => e.key === "Enter" && rename(name)} />
            <button class="btn primary" onclick={() => rename(name)}>{t("Save")}</button>
            <button class="btn" onclick={() => (renaming = null)}>{t("Cancel")}</button>
          {:else if confirming === name}
            <span class="warn">{t("Remove it along with its rules and remaps?")}</span>
            <button class="btn" onclick={() => remove(name)}>{t("Remove")}</button>
            <button class="btn" onclick={() => (confirming = null)}>{t("Cancel")}</button>
          {:else}
            <button class="link" title={t("Name used in the config file")} onclick={() => { renaming = name; newName = name; }}>{name}</button>
            <button class="btn" onclick={() => (confirming = name)}>{t("Remove")}</button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if others.length}
    <h2 class="group-title">{t("Other connected keyboards")}</h2>
    <div class="group">
      {#each others as d (d.product)}
        <div class="row">
          <span class="led on"></span>
          <div class="main">{d.product}{#if d.vendor_id !== undefined}<small>{t("Vendor {vendor}, product {product}", { vendor: hex(d.vendor_id), product: hex(d.product_id) })}</small>{/if}</div>
          <button class="btn" onclick={() => add(d)}>{t("Add")}</button>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .led { width: 8px; height: 8px; border-radius: 50%; background: var(--ink-3); flex: none; }
  .led.on { background: var(--ok); }
  .warn { color: var(--err); font-size: 12px; }
  .name { border: 1px solid var(--line); background: var(--win-bg); color: var(--ink); border-radius: var(--radius-ctl); padding: 4px 10px; font: 12px var(--mono); width: 140px; -webkit-user-select: text; user-select: text; }
</style>
