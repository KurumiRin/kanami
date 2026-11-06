<script>
  // Replaces every page while Accessibility permission is missing; disappears once the status refreshes after it is granted
  import Icon from "../lib/Icon.svelte";
  import { api } from "../lib/api.js";
  import { t } from "../lib/i18n.svelte.js";
  let { status } = $props();
  const inputMonitoring = $derived(status?.permissions?.input_monitoring);
</script>

<section class="page gate">
  <span class="gate-ico"><Icon name="lock" /></span>
  <h1>{t("kanami can't remap keys yet")}</h1>
  <p class="lead">{t("macOS needs you to allow the permissions below. You have to allow them again after every kanami upgrade, because kanami is not notarized by Apple.")}</p>
  <div class="group">
    <div class="row">
      <span class="glyph" style="background:var(--mod-top)"><Icon name="access" /></span>
      <div class="main">{t("Accessibility")}<small>{t("Reads and rewrites key presses")}</small></div>
      <button class="btn primary" onclick={() => api.openPermission("accessibility")}>{t("Open System Settings")}</button>
    </div>
    <div class="row">
      <span class="glyph" style="background:var(--accent)"><Icon name="keyboard" /></span>
      <div class="main">{t("Input Monitoring")}<small>{t("Tells which keyboard a key came from. Only needed for per-keyboard rules")}</small></div>
      {#if inputMonitoring}
        <span class="status-ok"><Icon name="check" />{t("Allowed")}</span>
      {:else}
        <button class="btn" onclick={() => api.openPermission("input_monitoring")}>{t("Open System Settings")}</button>
      {/if}
    </div>
  </div>
  <p class="hint" style="margin-top:12px">{t("This page refreshes by itself once you allow them. You don't need to restart kanami.")}</p>
</section>

<style>
  .gate { max-width: 520px; margin: 30px auto 0; }
  .gate-ico { width: 52px; height: 52px; border-radius: 14px; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent-ink); margin-bottom: 16px; }
  .gate-ico :global(.i) { width: 26px; height: 26px; }
  .glyph :global(.i) { stroke-width: 1.8; }
</style>
