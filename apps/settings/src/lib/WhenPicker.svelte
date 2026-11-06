<script>
  import Seg from "./Seg.svelte";
  import Icon from "./Icon.svelte";
  import { api } from "./api.js";
  import { appName } from "./apps.svelte.js";
  import { t } from "./i18n.svelte.js";

  // Which apps a rule applies in. mode: "all", "only", "except"; apps: list of Bundle IDs
  let { mode = $bindable("all"), apps = $bindable([]) } = $props();

  let running = $state([]);
  let picking = $state("");
  let typed = $state("");
  $effect(() => {
    api.runningApps().then((list) => (running = list)).catch(() => {});
  });

  const add = (id) => {
    id = id.trim();
    if (id && !apps.includes(id)) apps = [...apps, id];
  };
  const pick = (event) => {
    add(event.currentTarget.value);
    picking = "";
  };
</script>

<div class="ke-row">
  <span class="lbl">{t("Applies to")}</span>
  <Seg bind:value={mode} options={[{ value: "all", label: t("All apps") }, { value: "only", label: t("Only these apps") }, { value: "except", label: t("Except these apps") }]} />
</div>
{#if mode !== "all"}
  <div class="ke-row">
    <span class="lbl"></span>
    {#each apps as id (id)}
      <span class="chip" title={id}>{appName(id)}<button aria-label={t("Remove {name}", { name: appName(id) })} onclick={() => (apps = apps.filter((a) => a !== id))}><Icon name="close" /></button></span>
    {/each}
    <select aria-label={t("Choose from running apps")} bind:value={picking} onchange={pick}>
      <option value="">{t("Add a running app…")}</option>
      {#each running.filter((a) => !apps.includes(a.id)) as a (a.id)}<option value={a.id}>{a.name}</option>{/each}
    </select>
    <input type="text" class="mono" style="max-width:220px" spellcheck="false" placeholder={t("Or enter a Bundle ID and press Enter to add it")} bind:value={typed}
      onkeydown={(e) => { if (e.key === "Enter") { add(typed); typed = ""; } }} />
  </div>
{/if}
