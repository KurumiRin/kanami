<script>
  import { onDestroy, onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { api } from "./api.js";
  import { startCapture } from "./capture.js";
  import { edit, config } from "./config.svelte.js";
  import { keyboards, markSeen, setOrderFor } from "./keyboards.svelte.js";
  import { base, deviceNames, ensureDevice, isKeyboard } from "./model.js";
  import { t } from "./i18n.svelte.js";

  // Prompt bar shown the first time a keyboard is connected. Nothing changes automatically: keys are only listened to after "Start detecting" is clicked, and remaps are only written when clicked.
  let { connected, onCalibrate } = $props();

  let list = $state([]);
  let step = $state("ask"); // ask: ask whether to detect; listen: wait for a key; win, mac, other: the result
  let pressedKey = $state("");
  let wrongKeyboard = $state("");
  let stop = null;
  let timer;

  // Apple's own keyboards need no prompt
  const isApple = (product) => /^(Apple Internal|Magic Keyboard)/i.test(product);
  const fresh = $derived(
    list.find((d) => isKeyboard(d) && !isApple(d.product) && !keyboards.seen.includes(d.product) && !keyboards.orders[d.product]),
  );
  // The keyboard being handled: its result stays readable after it is unplugged
  let current = $state(null);
  const product = $derived(current ?? fresh?.product ?? null);

  async function refresh() {
    list = await api.devices().catch(() => []);
  }

  onMount(() => {
    timer = setInterval(() => connected() && refresh(), 10000);
    if (connected()) refresh();
  });

  $effect(() => {
    if (connected() && !list.length) refresh();
  });

  function cancelListen() {
    stop?.();
    stop = null;
    window.removeEventListener("blur", backToAsk);
  }

  function backToAsk() {
    cancelListen();
    step = "ask";
    wrongKeyboard = "";
  }

  function listen() {
    current = product;
    step = "listen";
    wrongKeyboard = "";
    stop = startCapture({
      chord: false,
      plain: false,
      onHeld: () => {},
      onPressed: () => {},
      onDone: (key) => {
        // The key came from another keyboard: keep waiting
        if (keyboards.device && keyboards.device !== current) {
          wrongKeyboard = keyboards.device;
          return;
        }
        cancelListen();
        pressedKey = key;
        if (key === "left_command") {
          setOrderFor(current, "mac");
          step = "mac";
        } else if (key === "left_option") {
          setOrderFor(current, "win");
          step = "win";
        } else {
          step = "other";
        }
      },
    });
    window.addEventListener("blur", backToAsk);
  }

  function finish() {
    if (product) markSeen(product);
    current = null;
    step = "ask";
    pressedKey = "";
  }

  // Write remaps for this keyboard only: swap Win and Alt so it feels like a Mac keyboard
  async function swapModifiers() {
    const spec = config.model?.device ?? {};
    const existing = deviceNames(config.model).find((n) => spec[n].product && current.includes(spec[n].product));
    const { device, ops } = existing ? { device: existing, ops: [] } : ensureDevice(config.model, "new:" + current);
    const at = [...base(device), "remap"];
    const done = await edit(
      ...ops,
      { op: "set", path: [...at, "left_command"], value: "left_option" },
      { op: "set", path: [...at, "left_option"], value: "left_command" },
      { op: "set", path: [...at, "right_command"], value: "right_option" },
      { op: "set", path: [...at, "right_option"], value: "right_command" },
    );
    if (done) finish();
  }

  function calibrate() {
    finish();
    onCalibrate();
  }

  onDestroy(() => {
    clearInterval(timer);
    cancelListen();
  });
</script>

<!-- Translated text may wrap parts in <b>…</b>; render those as bold text nodes. -->
{#snippet rich(text)}
  {#each text.split(/<\/?b>/) as chunk, i}{#if i % 2}<b>{chunk}</b>{:else}{chunk}{/if}{/each}
{/snippet}

{#if product}
  <div class="banner info">
    <Icon name="keyboard" />
    {#if step === "ask"}
      <span>{@render rich(t("First time connecting <b>{product}</b>. Detect its modifier layout? The keyboard diagram will follow it, and no keys will be changed.", { product }))}</span>
      <button class="btn primary" onclick={listen}>{t("Start detecting")}</button>
      <button class="btn" onclick={finish}>{t("Ignore")}</button>
    {:else if step === "listen"}
      <span>{@render rich(wrongKeyboard
        ? t("That key press came from {other}. On <b>{product}</b>, press <b>the key immediately left of the space bar</b>.", { other: wrongKeyboard, product })
        : t("On <b>{product}</b>, press <b>the key immediately left of the space bar</b>.", { product }))}</span>
      <button class="btn" onclick={backToAsk}>{t("Cancel")}</button>
    {:else if step === "mac"}
      <span>{@render rich(t("<b>{product}</b> uses the Mac layout, and the keyboard diagram is drawn to match.", { product }))}</span>
      <button class="btn primary" onclick={finish}>{t("OK")}</button>
    {:else if step === "win"}
      <span>{@render rich(t("<b>{product}</b> uses the Windows layout (Alt is left of the space bar), and the keyboard diagram is drawn to match. Swap Alt and Win so it feels like a Mac keyboard? This applies to this keyboard only.", { product }))}</span>
      <button class="btn primary" onclick={swapModifiers}>{t("Swap")}</button>
      <button class="btn" onclick={finish}>{t("Skip")}</button>
    {:else}
      <span>{@render rich(t("The key left of the space bar sends <b>{key}</b>, which matches none of the usual layouts. You can calibrate keys one by one so the keyboard diagram matches the real keyboard.", { key: pressedKey }))}</span>
      <button class="btn primary" onclick={calibrate}>{t("Calibrate keys")}</button>
      <button class="btn" onclick={finish}>{t("Skip")}</button>
    {/if}
  </div>
{/if}

<style>
  .banner.info { background: var(--accent-soft); }
  .banner.info > :global(.i) { color: var(--accent-ink); }
</style>
