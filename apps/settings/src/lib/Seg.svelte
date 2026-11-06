<script>
  // Segmented control: the selected thumb slides under the chosen option
  let { options, value = $bindable() } = $props();

  let buttons = $state([]);
  let thumb = $state(null);
  // Measured after the DOM updates, since a language switch changes the labels' widths.
  $effect(() => {
    const el = buttons[options.findIndex((o) => o.value === value)];
    thumb = el && options.length ? { left: el.offsetLeft, width: el.offsetWidth } : null;
  });
</script>

<div class="seg">
  {#if thumb}<span class="thumb" style="transform:translateX({thumb.left}px); width:{thumb.width}px"></span>{/if}
  {#each options as option, i}
    <button bind:this={buttons[i]} aria-pressed={option.value === value} onclick={() => (value = option.value)}>{option.label}</button>
  {/each}
</div>

<style>
  .seg { position: relative; display: inline-flex; padding: 2px; border-radius: 8px; background: rgba(127,127,127,.12); }
  .thumb { position: absolute; top: 2px; bottom: 2px; left: 0; border-radius: 6px; background: var(--win-bg); box-shadow: 0 1px 2px rgba(0,0,0,.15);
    transition: transform .22s cubic-bezier(.3, .7, .3, 1), width .22s cubic-bezier(.3, .7, .3, 1); }
  button { position: relative; border: 0; background: none; padding: 4px 12px; border-radius: 6px; cursor: pointer; transition: color .15s; }
  button[aria-pressed="false"] { color: var(--ink-2); }
  @media (prefers-reduced-motion: reduce) { .thumb { transition: none; } }
</style>
