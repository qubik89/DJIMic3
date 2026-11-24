<script>
  // Guided wireless connection: an illustrated, live walkthrough of pairing
  // a unit over Bluetooth and joining its Wi-Fi. `unit` "tx" downloads a
  // transmitter's recordings (the recordings panel); "rx" connects a
  // receiver for control (the main screen). The steps follow the backend's
  // step events, kept in `store` ({ phase, step, ssid, error }).
  import { t, tError } from "./i18n.js";
  import TxIllustration from "./TxIllustration.svelte";
  import RxIllustration from "./RxIllustration.svelte";
  import { wireless, startWireless, cancelWireless } from "./recordings.js";

  let { onclose, unit = "tx", store = wireless, onstart = startWireless, oncancel = cancelWireless } = $props();
  const rx = $derived(unit === "rx");

  const step = $derived($store.step?.step ?? null);
  const phase = $derived($store.phase);

  // Where we are in the four-stage strip.
  const stage = $derived(
    phase === "idle"
      ? -1
      : ["scanning", "found", "retry"].includes(step) || step === null
        ? 0
        : step === "confirm"
          ? 1
          : ["approved", "wifi-on", "joining", "connecting", "listing"].includes(step)
            ? 2
            : 3,
  );
  const stages = $derived(rx ? [$t("Find"), $t("Confirm"), $t("Wi-Fi"), $t("Ready")] : [$t("Link"), $t("Confirm"), $t("Wi-Fi"), $t("Download")]);

  const mode = $derived(
    phase === "idle" || phase === "error" ? "power" : stage === 0 ? "hold" : stage === 1 ? "tap" : stage === 2 ? "wifi" : "done",
  );

  // The transmitter gives up on an unconfirmed pairing after ~14 s.
  let confirmLeft = $state(14);
  $effect(() => {
    if (step !== "confirm") return;
    confirmLeft = 14;
    const timer = setInterval(() => (confirmLeft = Math.max(0, confirmLeft - 1)), 1000);
    return () => clearInterval(timer);
  });

  // Searching for a while: show what usually gets in the way.
  let searching = $state(0);
  $effect(() => {
    if (stage !== 0 || phase !== "connecting") {
      searching = 0;
      return;
    }
    const t0 = Date.now();
    const timer = setInterval(() => (searching = Math.floor((Date.now() - t0) / 1000)), 1000);
    return () => clearInterval(timer);
  });

  // The receiver's own label for "Connect to App", as its menu shows it in
  // each language (from DJI's manuals).
  const appLabel = $derived($t("Connect to App"));
  const howto = $derived(
    rx
      ? [
          { stage: 0, text: $t("Swipe down on the receiver's home screen to open the control menu.") },
          { stage: 0, text: $t("Swipe the icons until «App» is in the middle, with «{label}» above it, then tap it.", { label: appLabel }) },
          { stage: 0, text: $t("It warns that the current devices will be disconnected: confirm to connect.") },
          { stage: 0, text: $t("When it shows it's ready to connect, this app finds it on its own.") },
          { stage: 1, text: $t("When the app's connection request appears, check the code matches the one here and accept.") },
        ]
      : [
          { stage: 0, text: $t("Take the transmitter out of the case.") },
          { stage: 0, text: $t("Hold the link button (the lower one, with the chain icon) for 2 seconds, until the light blinks blue and green.") },
          { stage: 1, text: $t("When it buzzes twice, press the link button once.") },
        ],
  );

  const wifiDone = (s) => {
    const order = ["approved", "wifi-on", "joining", "connecting", "listing", "ready"];
    return order.indexOf(step) >= order.indexOf(s);
  };

  function errorText(e) {
    switch (e?.kind) {
      case "bluetooth-off":
        return $t("Bluetooth is off. Turn it on and try again.");
      case "no-bluetooth":
        return $t("This computer has no Bluetooth adapter.");
      case "refused":
        return rx
          ? $t("The receiver refused the connection. Try again and accept on its screen.")
          : $t("The transmitter refused the connection. Try again and press the link button once when it buzzes.");
      case "wifi":
        return rx
          ? $t("Couldn't join the receiver's Wi-Fi: {detail}", { detail: $tError(e.detail) })
          : $t("Couldn't join the transmitter's Wi-Fi: {detail}", { detail: $tError(e.detail) });
      case "wifi-busy":
        return $t("The Wi-Fi is already in use by another wireless connection. Disconnect it first.");
      case "bluetooth":
        return $t("Bluetooth error: {detail}. On a Mac, check that DJI Mic Control is allowed in System Settings › Privacy & Security › Bluetooth.", { detail: $tError(e.detail) });
      default:
        return $tError(e?.detail ?? e ?? "");
    }
  }

  function stop() {
    if (phase === "connecting") oncancel();
    if (phase === "error") store.update((w) => ({ ...w, phase: "idle", error: null }));
    onclose?.();
  }
</script>

<div class="wizard">
  <ol class="stages" aria-label={$t("Steps")}>
    {#each stages as label, i}
      <li class:done={stage > i} class:now={stage === i}>
        <span class="num">
          {#if stage > i}
            <svg viewBox="0 0 24 24" width="12" height="12"><path d="M5 12.5l4.2 4.2L19 7" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" /></svg>
          {:else}{i + 1}{/if}
        </span>
        <span class="label">{label}</span>
      </li>
    {/each}
  </ol>

  <div class="body">
    <div class="art">
      {#if rx}
        <RxIllustration
          {mode}
          option={appLabel}
          pin={$store.step?.pin ?? ""}
          labels={{
            down: $t("Swipe down"),
            swipe: $t("Swipe to «App»"),
            app: $t("Tap «App»"),
            confirm: $t("Confirm the connection"),
            ready: $t("Ready: this app finds it"),
            accept: $t("Check the code and accept"),
          }}
        />
      {:else}
        <TxIllustration {mode} label={mode === "hold" ? $t("Hold 2 s") : mode === "tap" ? $t("Press once") : ""} />
      {/if}
    </div>

    <div class="copy" aria-live="polite">
      {#if phase === "idle"}
        <h3>{rx ? $t("Connect the receiver without a cable") : $t("Download without the case")}</h3>
        <p>
          {rx
            ? $t("Your computer pairs with the receiver over Bluetooth and then controls it through the receiver's own Wi-Fi.")
            : $t("Your computer pairs with the transmitter over Bluetooth, joins its own Wi-Fi for a moment and copies the recordings. It takes about a minute to connect.")}
        </p>
        <ul class="facts">
          <li>
            <svg viewBox="0 0 24 24" width="16" height="16"><path d="M7 7l10 10-5 5V2l5 5L7 17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
            {$t("Bluetooth and Wi-Fi on")}
          </li>
          <li>
            <svg viewBox="0 0 24 24" width="16" height="16"><path d="M4 9a12 12 0 0 1 16 0M7 12.5a7.5 7.5 0 0 1 10 0M10 16a3 3 0 0 1 4 0" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /><circle cx="12" cy="19" r="1.3" fill="currentColor" /></svg>
            {rx
              ? $t("While connected, this computer has no Internet over Wi-Fi; a cable connection keeps working")
              : $t("Your Wi-Fi comes back on its own when you're done; a cable connection keeps working")}
          </li>
          <li>
            <svg viewBox="0 0 24 24" width="16" height="16"><path d="M12 3v12m0 0l-4-4m4 4l4-4M5 17v1.5A2.5 2.5 0 0 0 7.5 21h9a2.5 2.5 0 0 0 2.5-2.5V17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
            {rx
              ? $t("In this mode the receiver disconnects the transmitters: use it to change the receiver's settings, not to record or watch levels")
              : $t("About 4 MB/s: a 30-minute take in about a minute")}
          </li>
        </ul>
        <div class="first">
          <span class="badge">1</span>
          <span>{rx ? $t("Switch the receiver on and unplug it from USB.") : $t("Take one transmitter out of the case. It switches on by itself.")}</span>
        </div>
        <div class="actions">
          <button class="primary" onclick={onstart}>{$t("Start")}</button>
          <button class="ghost" onclick={onclose}>{$t("Cancel")}</button>
        </div>
      {:else if phase === "error"}
        <h3>{$t("Couldn't connect")}</h3>
        <p class="error">{errorText($store.error)}</p>
        <div class="actions">
          <button class="primary" onclick={onstart}>{$t("Try again")}</button>
          <button class="ghost" onclick={stop}>{$t("Cancel")}</button>
        </div>
      {:else if stage === 0 || stage === 1}
        <h3>
          {#if stage === 0}
            {rx ? $t("Put the receiver in app connection mode") : $t("Hold the link button for 2 seconds")}
          {:else}
            {rx ? $t("Accept on the receiver's screen") : $t("It buzzes twice: press the link button once")}
          {/if}
        </h3>
        <!-- Every step at once, the current ones highlighted. -->
        <ol class="howto">
          {#each howto as item, i}
            <li class:now={item.stage === stage} class:past={item.stage < stage}>
              <span class="n">{i + 1}</span><span>{item.text}</span>
            </li>
          {/each}
        </ol>
        {#if stage === 1 && rx && $store.step?.pin}
          <div class="pin" aria-label={$t("Verification code")}>
            {#each $store.step.pin.split("") as d}<span>{d}</span>{/each}
          </div>
        {/if}
        {#if step === "retry"}
          <p class="note">
            {rx ? $t("It wasn't accepted in time. Trying again…") : $t("It wasn't confirmed in time. Hold the link button for 2 seconds again.")}
          </p>
        {/if}
        <div class="status">
          {#if stage === 1}
            <span class="count" style:--p={confirmLeft / 14}>{confirmLeft}</span>
            {rx ? $t("Waiting for you to accept…") : $t("Waiting for your press…")}
          {:else if step === "found"}
            <span class="ok-dot"></span>{$t("Found {name}", { name: $store.step.name })}
          {:else}
            <span class="radar"></span>{rx ? $t("Looking for the receiver…") : $t("Looking for the transmitter…")}
          {/if}
        </div>
        {#if stage === 0 && searching >= 15 && step !== "found"}
          <div class="tips">
            <strong>{rx ? $t("The receiver doesn't show up?") : $t("The transmitter doesn't show up?")}</strong>
            <ul>
              {#if rx}
                <li>{$t("After tapping «{label}», the receiver's screen should be waiting for the app.", { label: appLabel })}</li>
                <li>{$t("If it was already connected to an app (DJI Mimo or this one), leave that screen or switch the receiver off and on, then tap «{label}» again.", { label: appLabel })}</li>
                <li>{$t("Close DJI Mimo on your phone and bring the receiver close to the computer.")}</li>
              {:else}
                <li>{$t("Take it out of the case: inside the case it doesn't advertise.")}</li>
                <li>{$t("If it's connected to a phone or a camera over Bluetooth, disconnect it there first.")}</li>
                <li>{$t("Hold the link button for 2 seconds again; it only advertises for a short while.")}</li>
              {/if}
            </ul>
          </div>
        {/if}
        <div class="actions"><button class="ghost" onclick={stop}>{$t("Cancel")}</button></div>
      {:else}
        <h3>{$t("Connecting over Wi-Fi")}</h3>
        <p>{rx ? $t("Keep the receiver near the computer.") : $t("You can put the transmitter down. Keep it near the computer.")}</p>
        <ul class="checks">
          <li class:ok={wifiDone("approved")}>{$t("Paired")}</li>
          <li class:ok={wifiDone("wifi-on")}>{rx ? $t("Receiver Wi-Fi on") : $t("Transmitter Wi-Fi on")}</li>
          <li class:ok={wifiDone("connecting")}>
            {#if wifiDone("connecting")}
              {$t("Joined {ssid}", { ssid: $store.ssid ?? "" })}
            {:else if $store.ssid}
              {$t("Joining {ssid}", { ssid: $store.ssid })}
            {:else}
              {$t("Joining its Wi-Fi")}
            {/if}
          </li>
          <li class:ok={wifiDone("ready")}>{rx ? $t("Opening the control session") : $t("Reading the recordings")}</li>
        </ul>
        <div class="actions"><button class="ghost" onclick={stop}>{$t("Cancel")}</button></div>
      {/if}
    </div>
  </div>
</div>

<style>
  .wizard {
    padding: 4px 0 8px;
  }
  .stages {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
    margin: 4px 0 6px;
    padding: 0;
    list-style: none;
  }
  .stages li {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 10px;
    color: var(--faint);
    font-size: 12px;
    border-top: 2px solid var(--line2);
    transition: color 0.2s, border-color 0.2s;
  }
  .stages li.now {
    color: var(--text);
    border-color: var(--inv);
  }
  .stages li.done {
    color: var(--dim);
    border-color: var(--good);
  }
  .num {
    width: 20px;
    height: 20px;
    flex: 0 0 auto;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1.5px solid currentColor;
    font-family: var(--mono);
    font-size: 10.5px;
    font-weight: 600;
  }
  .now .num {
    background: var(--inv);
    border-color: var(--inv);
    color: var(--invInk);
  }
  .done .num {
    background: var(--good);
    border-color: var(--good);
    color: #fff;
  }
  .body {
    display: grid;
    grid-template-columns: minmax(250px, 1fr) minmax(0, 1.15fr);
    align-items: center;
    gap: 20px;
  }
  .art {
    border-radius: 14px;
    background: radial-gradient(120% 90% at 40% 30%, var(--raise), transparent 70%);
  }
  h3 {
    margin: 0 0 8px;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.01em;
    line-height: 1.25;
  }
  p {
    margin: 0 0 12px;
    color: var(--dim);
    line-height: 1.55;
  }
  .note {
    padding: 8px 12px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--warn) 14%, transparent);
    color: var(--text);
  }
  .error {
    padding: 10px 12px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--rec) 12%, transparent);
    color: var(--text);
  }
  .facts {
    margin: 0 0 14px;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 8px;
  }
  .facts li {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    font-size: 12.5px;
    color: var(--dim);
    line-height: 1.4;
  }
  .facts svg {
    flex: 0 0 auto;
    margin-top: 1px;
    color: var(--text);
  }
  .first {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    margin-bottom: 14px;
    border-radius: 10px;
    background: var(--raise);
    font-weight: 500;
  }
  .badge {
    width: 22px;
    height: 22px;
    flex: 0 0 auto;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--inv);
    color: var(--invInk);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 600;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
    font-weight: 500;
  }
  .radar {
    position: relative;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--good);
  }
  .radar::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 2px solid var(--good);
    animation: radar 1.4s ease-out infinite;
  }
  .ok-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--good);
  }
  .count {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 600;
    background: conic-gradient(var(--warn) calc(var(--p) * 360deg), var(--track) 0);
    box-shadow: inset 0 0 0 4px var(--panel);
  }
  .tips {
    margin: 0 0 14px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--raise);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .tips ul {
    margin: 6px 0 0;
    padding-left: 18px;
    color: var(--dim);
  }
  .tips li + li {
    margin-top: 4px;
  }
  .howto {
    margin: 0 0 14px;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 8px;
  }
  .howto li {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    color: var(--faint);
    line-height: 1.45;
    transition: color 0.2s;
  }
  .howto li.now {
    color: var(--text);
  }
  .howto li.past {
    color: var(--dim);
  }
  .howto .n {
    width: 20px;
    height: 20px;
    flex: 0 0 auto;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1.5px solid currentColor;
    font-family: var(--mono);
    font-size: 10.5px;
    font-weight: 600;
  }
  .howto li.now .n {
    background: var(--inv);
    border-color: var(--inv);
    color: var(--invInk);
  }
  .howto li.past .n {
    background: var(--good);
    border-color: var(--good);
    color: #fff;
  }
  .pin {
    display: flex;
    gap: 6px;
    margin: 0 0 14px;
  }
  .pin span {
    width: 38px;
    height: 46px;
    display: grid;
    place-items: center;
    border-radius: 10px;
    background: var(--raise);
    border: 1px solid var(--line2);
    font-family: var(--mono);
    font-size: 22px;
    font-weight: 600;
  }
  .checks {
    margin: 0 0 14px;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 8px;
  }
  .checks li {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--faint);
    transition: color 0.2s;
  }
  .checks li::before {
    content: "";
    width: 16px;
    height: 16px;
    flex: 0 0 auto;
    border-radius: 50%;
    border: 2px solid var(--line2);
    border-top-color: var(--dim);
    animation: spin 0.9s linear infinite;
  }
  .checks li.ok {
    color: var(--text);
  }
  .checks li.ok::before {
    border: none;
    animation: none;
    background: var(--good)
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cpath d='M6 12.5l4 4 8-9' fill='none' stroke='white' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E")
      center / 12px no-repeat;
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .actions button {
    height: 34px;
    padding: 0 16px;
    border: none;
    border-radius: 10px;
    font-weight: 500;
  }
  .primary {
    background: var(--inv);
    color: var(--invInk);
  }
  .ghost {
    background: transparent;
    color: var(--dim);
  }
  .ghost:hover {
    background: var(--raise);
    color: var(--text);
  }
  @media (max-width: 640px) {
    .body {
      grid-template-columns: 1fr;
    }
    .stages .label {
      display: none;
    }
  }
  @keyframes radar {
    to {
      transform: scale(2.4);
      opacity: 0;
    }
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .radar::after,
    .checks li::before {
      animation: none;
    }
  }
</style>
