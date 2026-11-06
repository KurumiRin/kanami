<script>
  import { untrack } from "svelte";
  import Seg from "./Seg.svelte";
  import KeyField from "./KeyField.svelte";
  import WhenPicker from "./WhenPicker.svelte";
  import { ruleKind } from "./model.js";
  import { plainWhen, whenWith, fieldOp } from "./apps.svelte.js";
  import { N_, t } from "./i18n.svelte.js";

  // A null rule means a new one. onSave receives the full fields (new) or the changes relative to the original rule (edit)
  let { rule = null, platform, onSave, onCancel, onDelete = null } = $props();

  // The rule as opened; the form starts editing from it
  const old = untrack(() => rule) ?? {};
  let from = $state(old.from ?? "");
  let kind = $state(old.from ? ruleKind(old) : "keys");
  let to = $state(old.to && old.to !== "none" ? old.to : "");
  let tap = $state(old.tap ?? "");
  let hold = $state(old.hold ?? "");
  let strategy = $state(old.strategy ?? "permissive");
  let run = $state(old.run ?? "");
  let call = $state(old.call ?? "");
  let mode = $state(old.when?.app?.length ? "only" : old.when?.app_not?.length ? "except" : "all");
  let apps = $state([...(old.when?.app ?? old.when?.app_not ?? [])]);
  let problem = $state("");

  const KINDS = $derived([
    { value: "keys", label: t("Send keys") },
    { value: "taphold", label: t("Tap / Hold") },
    { value: "run", label: t("Run command") },
    { value: "call", label: t("Call script") },
    { value: "off", label: t("Disabled") },
  ]);

  // The fields the form currently stands for
  function fields() {
    const f = { from, to: undefined, tap: undefined, hold: undefined, strategy: undefined, run: undefined, call: undefined };
    if (kind === "keys") f.to = to;
    if (kind === "off") f.to = "none";
    if (kind === "taphold") {
      f.tap = tap || undefined;
      f.hold = hold;
      // The default strategy is not written to the file unless it was there already
      f.strategy = strategy === "permissive" && old.strategy === undefined ? undefined : strategy;
    }
    if (kind === "run") f.run = run.trim();
    if (kind === "call") f.call = call.trim();
    return f;
  }

  function check() {
    if (!from) return N_("Choose the key to remap.");
    if (kind === "keys" && !to) return N_("Choose the key it becomes.");
    if (kind === "taphold" && !hold) return N_("Choose what the key becomes when held.");
    if (kind === "run" && !run.trim()) return N_("Enter the command to run.");
    if (kind === "call" && !call.trim()) return N_("Enter the script function name.");
    return "";
  }

  function save() {
    problem = check();
    if (problem) return;
    const f = fields();
    const when = whenWith(old.when, mode, apps);
    if (!rule) {
      const value = Object.fromEntries(Object.entries(f).filter(([, v]) => v !== undefined));
      if (when) value.when = when;
      onSave(value);
      return;
    }
    const ops = (path) => [
      ...Object.entries(f).flatMap(([key, value]) => fieldOp(path, key, old[key], value)),
      ...fieldOp(path, "when", plainWhen(old.when), when),
    ];
    onSave(ops);
  }

</script>

<div class="editor">
  <div class="ke-row"><span class="lbl">{t("Press")}</span><KeyField bind:value={from} {platform} mods /></div>
  <div class="ke-row"><span class="lbl">{t("Becomes")}</span><Seg bind:value={kind} options={KINDS} /></div>
  {#if kind === "keys"}
    <div class="ke-row"><span class="lbl"></span><KeyField bind:value={to} {platform} mods targets out /></div>
  {:else if kind === "taphold"}
    <div class="ke-row"><span class="lbl">{t("Tap")}</span><KeyField bind:value={tap} {platform} mods targets none clearable out placeholder={t("Same as original key")} /></div>
    <div class="ke-row"><span class="lbl">{t("Hold")}</span><KeyField bind:value={hold} {platform} mods targets out /></div>
    <div class="ke-row"><span class="lbl">{t("Detection")}</span>
      <select aria-label={t("Detection method")} bind:value={strategy}>
        <option value="permissive">{t("Hold when another key is tapped during it (recommended)")}</option>
        <option value="eager">{t("Hold as soon as another key is pressed (use as a modifier)")}</option>
        <option value="timeout">{t("Hold based on duration only")}</option>
      </select>
    </div>
  {:else if kind === "run"}
    <div class="ke-row"><span class="lbl"></span><input type="text" class="mono" spellcheck="false" placeholder={t("e.g. open -a Terminal")} bind:value={run} /></div>
  {:else if kind === "call"}
    <div class="ke-row"><span class="lbl"></span><input type="text" class="mono" spellcheck="false" placeholder={t("Function name in the script")} bind:value={call} /></div>
  {/if}
  <WhenPicker bind:mode bind:apps />
  {#if problem}<div class="ke-row" style="color:var(--err)"><span class="lbl"></span>{t(problem)}</div>{/if}
  <div class="ke-actions">
    <button class="btn primary" onclick={save}>{t("Save")}</button>
    <button class="btn" onclick={onCancel}>{t("Cancel")}</button>
    {#if onDelete}<span class="grow"></span><button class="btn" onclick={onDelete}>{t("Delete this rule")}</button>{/if}
  </div>
</div>
