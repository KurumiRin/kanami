<script>
  import { legend, isModifier } from "./keys.js";
  // One keycap. out: the key produced by the remap (cherry pink)
  let { name, platform, out = false, lg = false, sub = "" } = $props();
</script>

<span class="cap" class:mod={isModifier(name)} class:out class:lg>{legend(name, platform)}{#if sub}<small>{sub}</small>{/if}</span>
