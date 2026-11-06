<script>
  import Icon from "../lib/Icon.svelte";
  import Expr from "../lib/Expr.svelte";
  import Seg from "../lib/Seg.svelte";
  import KeyField from "../lib/KeyField.svelte";
  import { config, edit } from "../lib/config.svelte.js";
  import { legend } from "../lib/keys.js";
  import { N_, t } from "../lib/i18n.svelte.js";

  let { platform } = $props();

  const combos = $derived(config.model?.combo ?? []);
  let open = $state(null); // index, or "new"
  let keys = $state([]);
  let next = $state("");
  let kind = $state("keys");
  let to = $state("");
  let run = $state("");
  let call = $state("");
  let problem = $state("");

  // Add a key to the list as soon as it is picked
  $effect(() => {
    if (next) {
      if (!keys.includes(next)) keys = [...keys, next];
      next = "";
    }
  });

  function show(index) {
    const c = index === "new" ? {} : combos[index];
    open = index;
    keys = [...(c.keys ?? [])];
    kind = c.run !== undefined ? "run" : c.call !== undefined ? "call" : "keys";
    to = c.to ?? "";
    run = c.run ?? "";
    call = c.call ?? "";
    problem = "";
  }

  async function save() {
    if (keys.length < 2) return void (problem = N_("Pick at least two keys."));
    const action = kind === "keys" ? { to } : kind === "run" ? { run: run.trim() } : { call: call.trim() };
    if (!Object.values(action)[0]) return void (problem = N_("Choose an action."));
    let done;
    if (open === "new") {
      done = await edit({ op: "insert", path: ["combo"], index: combos.length, value: { keys, ...action } });
    } else {
      const path = ["combo", open];
      const old = combos[open];
      const ops = [{ op: "set", path: [...path, "keys"], value: keys }];
      for (const field of ["to", "run", "call"]) {
        if (action[field] !== undefined) ops.push({ op: "set", path: [...path, field], value: action[field] });
        else if (old[field] !== undefined) ops.push({ op: "remove", path: [...path, field] });
      }
      done = await edit(...ops);
    }
    if (done) open = null;
  }

  const remove = async (i) => {
    if (await edit({ op: "remove", path: ["combo", i] })) open = null;
  };
</script>

{#snippet form()}
  <div class="editor">
    <div class="ke-row"><span class="lbl">{t("Together")}</span>
      {#each keys as key (key)}
        <span class="chip">{legend(key, platform)}<button aria-label={t("Remove {key}", { key })} onclick={() => (keys = keys.filter((k) => k !== key))}><Icon name="close" /></button></span>
      {/each}
      <KeyField bind:value={next} {platform} plain exclude={keys} placeholder={t("Add a key")} />
    </div>
    <div class="ke-row"><span class="lbl">{t("Action")}</span><Seg bind:value={kind} options={[{ value: "keys", label: t("Keys") }, { value: "run", label: t("Run command") }, { value: "call", label: t("Call script") }]} /></div>
    {#if kind === "keys"}
      <div class="ke-row"><span class="lbl"></span><KeyField bind:value={to} {platform} mods targets out /></div>
    {:else if kind === "run"}
      <div class="ke-row"><span class="lbl"></span><input type="text" class="mono" spellcheck="false" placeholder={t("e.g. open -a Safari")} bind:value={run} /></div>
    {:else}
      <div class="ke-row"><span class="lbl"></span><input type="text" class="mono" spellcheck="false" placeholder={t("Function name in the script")} bind:value={call} /></div>
    {/if}
    {#if problem}<div class="ke-row" style="color:var(--err)"><span class="lbl"></span>{t(problem)}</div>{/if}
    <div class="ke-actions">
      <button class="btn primary" onclick={save}>{t("Save")}</button>
      <button class="btn" onclick={() => (open = null)}>{t("Cancel")}</button>
      {#if open !== "new"}<span class="grow"></span><button class="btn" onclick={() => remove(open)}>{t("Delete")}</button>{/if}
    </div>
  </div>
{/snippet}

<section class="page">
  <h1>{t("Combos")}</h1>
  <p class="lead">{t("Press several keys at almost the same time to trigger one action. Each key still types normally on its own, and combos don't fire during fast typing.")}</p>
  <div class="toolbar">
    <span class="grow"></span>
    <button class="btn primary" onclick={() => show("new")}><Icon name="plus" />{t("Add combo")}</button>
  </div>
  {#if open === "new"}<div style="margin-bottom:12px">{@render form()}</div>{/if}

  {#if combos.length}
    <div class="rlist">
      {#each combos as combo, i}
        <div class="rule plain">
          <button class="lines" onclick={() => (open === i ? (open = null) : show(i))}>
            <span class="line">
              {#each combo.keys as key, j}{#if j}<span class="plus">+</span>{/if}<Expr value={key} {platform} />{/each}
              <span class="becomes"></span>
              {#if combo.run !== undefined}<span class="cmd">{combo.run}</span>
              {:else if combo.call !== undefined}<span class="cmd">{combo.call}()</span>
              {:else}<Expr value={combo.to} {platform} out />{/if}
            </span>
          </button>
          <button class="icon-btn" aria-label={t("Delete this combo")} title={t("Delete")} onclick={() => remove(i)}><Icon name="trash" /></button>
        </div>
        {#if open === i}{@render form()}{/if}
      {/each}
    </div>
  {:else if open === null}
    <div class="empty">
      <b>{t("No combos yet")}</b>
      <span>{t("For example, press D and F together to act as Esc.")}</span>
      <button class="btn primary" onclick={() => show("new")}>{t("Add combo")}</button>
    </div>
  {/if}
</section>

<style>
  .rule.plain { grid-template-columns: 1fr auto; }
  .plus { color: var(--ink-3); margin: 0 2px; }
</style>
