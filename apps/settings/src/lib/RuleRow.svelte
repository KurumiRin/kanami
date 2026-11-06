<script>
  import Expr from "./Expr.svelte";
  import Icon from "./Icon.svelte";
  import { whenText } from "./apps.svelte.js";
  import { ruleKind } from "./model.js";
  import { t } from "./i18n.svelte.js";

  // A rule reads like a sentence. drag: callback when the grip is pressed; line-level drop markers are passed in from outside via class
  let { rule, platform, onOpen, onToggle, onGrip, dropAttr, container, classes = "" } = $props();
  const kind = $derived(ruleKind(rule));
  const on = $derived(rule.enabled !== false);
  const scope = $derived(whenText(rule.when));
</script>

<div class="rule {classes}" class:off={!on} data-drop={dropAttr} data-container={container}>
  <span class="grip" role="presentation" title={t("Drag to reorder")} onpointerdown={onGrip}><Icon name="grip" /></span>
  <button class="lines" onclick={onOpen}>
    {#if kind === "taphold"}
      <span class="line"><span class="verb">{t("Tap")}</span><Expr value={rule.from} {platform} /><span class="becomes"></span>
        {#if rule.tap === "none"}<span class="badge">{t("Sends nothing")}</span>{:else}<Expr value={rule.tap ?? rule.from} {platform} out />{/if}</span>
      <span class="line"><span class="verb">{t("Hold")}</span><Expr value={rule.from} {platform} /><span class="becomes"></span><Expr value={rule.hold} {platform} out /></span>
    {:else}
      <span class="line"><span class="verb">{t("Press")}</span><Expr value={rule.from} {platform} /><span class="becomes"></span>
        {#if kind === "run"}<span class="cmd">{rule.run}</span>
        {:else if kind === "call"}<span class="cmd">{rule.call}()</span>
        {:else if kind === "off"}<span class="badge">{t("Disabled")}</span>
        {:else}<Expr value={rule.to} {platform} out />{/if}</span>
    {/if}
    {#if scope}<span class="scope">{scope}</span>{/if}
  </button>
  <button class="switch" role="switch" aria-checked={on} aria-label={t("Enable this rule")} onclick={onToggle}></button>
</div>
