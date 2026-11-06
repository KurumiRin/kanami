<script>
  import { t } from "./i18n.svelte.js";
  import { bottomRow, shownKey } from "./keyboards.svelte.js";
  import { MAIN, NAV, PAD, PAD_TOP } from "./layout.js";

  // A full-size keyboard. How each key is drawn is decided by key (name, style)
  let { platform, mini = false, key } = $props();
  const win = $derived(platform === "win");
  const order = $derived(bottomRow(platform));
  const idOf = (k) => (win ? k.win : k.id);
  // The bottom-row modifier order follows the keyboard, not the system: a Windows-layout keyboard on a Mac still has Win first, then Alt
  const bottomId = (k) => (order === "win" ? k.win : k.id);
</script>

{#snippet rows(list)}
  {#each list as row, i}
    <div class="r" class:frow={i === 0}>
      {#each row as k}
        {#if k.gap}<div class="gap" style="flex-grow:{k.gap}"></div>
        {:else}{@render key(shownKey(list === MAIN && i === MAIN.length - 1 ? bottomId(k) : idOf(k)), `flex-grow:${k.u}`)}{/if}
      {/each}
    </div>
  {/each}
{/snippet}

<div class="board" class:mini aria-label={t("Keyboard")}>
  <div class="blk main">{@render rows(MAIN)}</div>
  <div class="blk nav">{@render rows(NAV)}</div>
  <div class="blk pad">
    {@render rows([PAD_TOP[win ? "win" : "mac"]])}
    <div class="padgrid">
      {#each PAD[win ? "win" : "mac"] as k (k.id)}
        {@render key(shownKey(k.id), `grid-column:span ${k.w}; grid-row:span ${k.h}`)}
      {/each}
    </div>
  </div>
</div>
