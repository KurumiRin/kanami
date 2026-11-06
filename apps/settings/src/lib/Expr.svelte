<script>
  import { chordCaps } from "./keys.js";
  // Draws a key expression (or a sequence) as keycaps. out: the key produced by the remap
  let { value, platform, out = false } = $props();
  const list = $derived(Array.isArray(value) ? value : [value]);
</script>

<span class="expr">
  {#each list as expr, i}
    {#if i}<span class="seq"></span>{/if}
    {#each chordCaps(String(expr), platform) as cap, j}
      {#if j}<span class="plus">+</span>{/if}<span class="cap" class:mod={cap.mod} class:out>{cap.label}</span>
    {/each}
  {/each}
</span>

<style>
  .expr { display: inline-flex; align-items: center; flex-wrap: wrap; gap: 2px; }
  .expr .cap + .cap { margin-left: 0; }
  .plus { color: var(--ink-3); margin: 0 1px; }
  .seq { width: 8px; }
</style>
