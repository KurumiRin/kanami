<script>
  import { api } from "./api.js";
  import { config } from "./config.svelte.js";
  import { deviceNames, isKeyboard } from "./model.js";
  import { t } from "./i18n.svelte.js";

  // Keyboard picker tabs. value: "" (all keyboards), the name in [device.*], or "new:<product name>" (connected but not yet in the config)
  let { value = $bindable(""), count, status } = $props();

  let connected = $state([]);
  $effect(() => {
    status; // refresh when the background service starts or stops
    api.devices().then((d) => (connected = d.filter(isKeyboard))).catch(() => (connected = []));
  });

  const matches = (spec, d) =>
    (spec.vendor_id === undefined || spec.vendor_id === d.vendor_id) &&
    (spec.product_id === undefined || spec.product_id === d.product_id) &&
    (spec.product === undefined || d.product.includes(spec.product));

  const tabs = $derived.by(() => {
    const devices = config.model?.device ?? {};
    const named = deviceNames(config.model).map((name) => ({
      id: name,
      label: devices[name].product ?? name,
      on: connected.some((d) => matches(devices[name], d)),
    }));
    const fresh = connected
      .filter((d) => !deviceNames(config.model).some((n) => matches(devices[n], d)))
      .map((d) => ({ id: "new:" + d.product, label: d.product, on: true }));
    const seen = new Set();
    return [...named, ...fresh].filter((t) => !seen.has(t.id) && seen.add(t.id));
  });

  // When the selected keyboard is removed, go back to "All keyboards"
  $effect(() => {
    if (value && !tabs.some((t) => t.id === value)) value = "";
  });
</script>

<div class="kb-tabs" role="tablist" aria-label={t("Keyboards")}>
  <button class="ltab" role="tab" aria-selected={value === ""} onclick={() => (value = "")}>{t("All keyboards")}<small>{count("")}</small></button>
  {#each tabs as tab (tab.id)}
    <button class="ltab" class:off={!tab.on} role="tab" aria-selected={value === tab.id} onclick={() => (value = tab.id)}>
      {#if tab.on}<span class="conn"></span>{/if}{#if tab.on}{tab.label}{:else}{t("{name} (not connected)", { name: tab.label })}{/if}<small>{tab.id.startsWith("new:") ? 0 : count(tab.id)}</small>
    </button>
  {/each}
</div>
