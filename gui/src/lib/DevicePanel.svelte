<script>
  import { t, help } from "./i18n.js";
  import DevicePicto from "./DevicePicto.svelte";
  import DeviceSwitcher from "./DeviceSwitcher.svelte";
  import BatteryIcon from "./BatteryIcon.svelte";
  import LevelMeter from "./LevelMeter.svelte";
  import Switch from "./ui/Switch.svelte";
  import Segmented from "./ui/Segmented.svelte";
  import Stepper from "./ui/Stepper.svelte";
  import { SCENES } from "./scenes.js";

  let {
    device,
    devices = [],
    status,
    settings = [],
    values = {},
    pending = {},
    pendingTx = {},
    tab = null,
    onchange,
    onchangeTx,
    onselect,
    ontab,
    ontoast,
  } = $props();

  // ---- Labels -------------------------------------------------------------
  // Sentence-case display labels keyed by setting id (the model's own labels
  // are the fallback). Translated through $t like everything else.
  const LABELS = {
    "noise-cancel-power": "Noise cancelling",
    "noise-cancel": "Mode",
    "noise-cancel-button": "Power button toggles NC",
    "low-cut": "Low cut",
    "tx-clip-control": "Clipping control",
    "clip-limiter": "Clipping control",
    "loudness-balance": "Loudness balance",
    "safety-track": "Safety track",
    "auto-off-15m": "Auto-off receiver",
    "tx-auto-off-15m": "Auto-off transmitters",
    "camera-power": "Follow camera power",
    "plug-free": "Plug-free ext. speaker",
    "mic-leds": "Mic LEDs",
    vibration: "TX vibration",
    float32: "32-bit float",
    "loop-recording": "Loop recording",
    "startup-recording": "Record on power-on",
    "low-power-recording": "Low-battery auto record",
    "rec-stop": "Record via TX button",
  };
  const GROUP_LABELS = { "Power & Startup": "Power & startup", "Internal Recording": "Internal recording" };
  const labelOf = (s) => $t(LABELS[s.id] ?? s.label);
  const groupLabel = (g) => $t(GROUP_LABELS[g] ?? g);
  const opts = (s) => (s ? s.options.map((o) => ({ value: o.value, label: $t(o.label) })) : []);

  // ---- Setting buckets ------------------------------------------------------
  const settingsById = $derived(Object.fromEntries(settings.map((s) => [s.id, s])));
  const CARD_IDS = new Set(["stereo", "rx-gain", "monitor-gain", "lossless"]);
  const NC_IDS = new Set(["noise-cancel", "noise-cancel-power", "noise-cancel-button"]);
  const isTxTargeted = (s) => s.v2_target === "tx";
  // Everything not on a card, in declared order, grouped into tabs.
  const tabSettings = $derived(settings.filter((s) => !CARD_IDS.has(s.id) && !isTxTargeted(s)));
  const groups = $derived([...new Set(tabSettings.map((s) => s.group))]);
  const activeTab = $derived(groups.includes(tab) ? tab : groups[0]);
  const hasNc = $derived(!!settingsById["noise-cancel-power"] || !!settingsById["noise-cancel"]);
  const ncInTab = $derived(hasNc && (settingsById["noise-cancel-power"] ?? settingsById["noise-cancel"]).group === activeTab);
  // Display order within a tab; ids not listed keep the model's order after these.
  const ORDER = ["low-cut", "noise-cancel-button", "tx-clip-control", "clip-limiter", "loudness-balance", "safety-track"];
  const rank = (id) => (ORDER.includes(id) ? ORDER.indexOf(id) : ORDER.length);
  const rows = $derived(
    tabSettings
      .filter((s) => s.group === activeTab && s.id !== "noise-cancel" && s.id !== "noise-cancel-power")
      .map((s, i) => ({ s, i }))
      .sort((a, b) => rank(a.s.id) - rank(b.s.id) || a.i - b.i)
      .map(({ s }) => s),
  );

  // ---- Locks (unchanged rules) ---------------------------------------------
  function lockReason(s) {
    if (!s) return null;
    if (status.protocol_version === 1 && s.v1_command == null) {
      return $t("Requires protocol v2, please update your firmware");
    }
    const ncOn = values["noise-cancel-power"] != null ? values["noise-cancel-power"] === "on" : status.nc_enabled;
    if (s.id === "noise-cancel" && ncOn === false) {
      return settingsById["noise-cancel-power"]
        ? $t('Turn on "{name}" first', { name: $t("Noise cancelling") })
        : $t("Turn on Noise Cancel using the button on the transmitter");
    }
    for (const other of s.exclusive_with ?? []) {
      if (other === "stereo") {
        if (values.stereo && values.stereo !== "mono") {
          return $t('Switch "{name}" to Mono first', { name: $t("Channels") });
        }
      } else if (values[other] === "on") {
        return $t("Turn off {name} first", { name: labelOf(settingsById[other] ?? { id: other, label: other }) });
      }
    }
    return null;
  }

  // ---- Changes with side-effect toasts -------------------------------------
  function set(id, value) {
    const prev = values[id];
    onchange?.(id, value);
    if (id === "stereo" && (prev === "quad") !== (value === "quad")) {
      ontoast?.($t("Receiver USB audio is reconnecting…"));
    } else if (id === "plug-free") {
      ontoast?.($t("Receiver is rebooting to apply the change…"));
    }
  }
  const toggleValue = (s, on) => s.options[on ? 1 : 0].value;
  const isOn = (s, v) => v === s.options[1]?.value;

  // ---- Receiver -----------------------------------------------------------
  const rx = $derived(status.rx);
  const rxSubtitle = $derived(
    [rx?.firmware ? `FW ${rx.firmware}` : null, status.protocol_version ? `v${status.protocol_version}` : null]
      .filter(Boolean)
      .join(" · "),
  );
  const formatDial = (v) => (v > 0 ? `+${v}` : v < 0 ? `−${-v}` : "0");

  // ---- Transmitters -------------------------------------------------------
  const maxTx = $derived(device.max_tx ?? 2);
  const visibleTx = $derived(
    (status.tx ?? []).slice(0, maxTx).map((tx, i) => ({ tx, i })).filter(({ tx, i }) => i < 2 || tx),
  );
  const connectedTx = $derived(visibleTx.filter(({ tx }) => tx));
  // Raw level units: the Mic 3 reports ~0–100, the Mic Mini ~0x14–0x4f.
  const levelMax = $derived(device.model_id === "dji-mic-3" ? 100 : 80);
  const levelOf = (tx) => (tx?.level == null ? null : tx.level / levelMax);
  const txValue = (tx, id) => (id === "voice-tone" ? tx.settings?.[id] ?? tx.voice_tone : tx.settings?.[id]);
  const txPictogram = (tx) => {
    if (tx?.product_name === "DJI Mic Mini 2") return "mic-mini-2-tx";
    return `${device.pictogram_key}-tx`;
  };
  const hoursLeft = (tenths) => $t("{h} h left", { h: (tenths / 10).toFixed(1) });

  // Local recording timers: the device doesn't report elapsed time, so the
  // clock starts when "record: on" is first seen for a slot.
  let recStartedAt = $state({});
  let now = $state(Date.now());
  $effect(() => {
    const next = { ...recStartedAt };
    let changed = false;
    for (const { tx, i } of visibleTx) {
      const recording = tx && txValue(tx, "record") === "on";
      if (recording && !next[i]) {
        next[i] = Date.now();
        changed = true;
      } else if (!recording && next[i]) {
        delete next[i];
        changed = true;
      }
    }
    if (changed) recStartedAt = next;
  });
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 500);
    return () => clearInterval(timer);
  });
  const clock = (start) => {
    const s = Math.max(0, Math.floor((now - start) / 1000));
    return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
  };
  const blinkOn = $derived(Math.floor(now / 500) % 2 === 0);

  // ---- Quick actions --------------------------------------------------------
  const canMute = $derived(!!settingsById["mute"] && connectedTx.length > 0);
  const canRecord = $derived(!!settingsById["record"] && connectedTx.length > 0);
  const allMuted = $derived(canMute && connectedTx.every(({ tx }) => txValue(tx, "mute") === "on"));
  const allRecording = $derived(canRecord && connectedTx.every(({ tx }) => txValue(tx, "record") === "on"));
  const firstRecStart = $derived(
    Math.min(...connectedTx.map(({ i }) => recStartedAt[i] ?? now)),
  );

  async function muteAll() {
    const target = allMuted ? "off" : "on";
    for (const { tx, i } of connectedTx) {
      if (txValue(tx, "mute") !== target) await onchangeTx?.(i, "mute", target);
    }
  }
  async function recordAll() {
    if (allRecording) {
      for (const { i } of connectedTx) await onchangeTx?.(i, "record", "off");
      return;
    }
    for (const { tx, i } of connectedTx) {
      if (txValue(tx, "record") !== "on") await onchangeTx?.(i, "record", "on");
    }
    ontoast?.($t("Internal recording started on all transmitters"));
  }

  // ---- Scenes ---------------------------------------------------------------
  const sceneEntries = (sc) =>
    Object.entries(sc.set).filter(([id]) =>
      id === "voice-tone" ? !!settingsById[id] && connectedTx.length > 0 : !!settingsById[id],
    );
  function sceneMatches(sc) {
    const entries = sceneEntries(sc);
    if (entries.length === 0) return false;
    return entries.every(([id, v]) =>
      id === "voice-tone" ? connectedTx.every(({ tx }) => txValue(tx, id) === v) : values[id] === v,
    );
  }
  const activeScene = $derived(SCENES.find(sceneMatches)?.id ?? null);

  async function applyScene(sc) {
    let n = 0;
    for (const [id, v] of sceneEntries(sc)) {
      if (id === "voice-tone") {
        for (const { tx, i } of connectedTx) {
          if (txValue(tx, id) !== v) {
            n++;
            await onchangeTx?.(i, id, v);
          }
        }
        continue;
      }
      if (values[id] === v) continue;
      // Respect mutual exclusions (e.g. Mic Mini stereo vs safety track).
      const blocked = (settingsById[id].exclusive_with ?? []).some((o) => values[o] === "on");
      if (blocked) continue;
      n++;
      await onchange?.(id, v);
    }
    const name = $t(sc.label);
    ontoast?.(
      n === 0
        ? $t("{scene} is already active", { scene: name })
        : n === 1
          ? $t("{scene} scene applied · 1 change", { scene: name })
          : $t("{scene} scene applied · {n} changes", { scene: name, n }),
    );
  }

  // ---- Noise cancelling row ------------------------------------------------
  const ncPower = $derived(settingsById["noise-cancel-power"]);
  const ncMode = $derived(settingsById["noise-cancel"]);
  const ncModeLock = $derived(lockReason(ncMode));
  const ncPowerLock = $derived(lockReason(ncPower));
  const ncHelp = $derived([$help("noise-cancel-power"), $help("noise-cancel")].filter(Boolean).join(" "));
</script>

{#snippet micIcon(slash)}
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <rect x="8.5" y="2.5" width="7" height="12" rx="3.5" stroke="currentColor" stroke-width="1.6" />
    <path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8.5 21h7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    {#if slash}<path d="M4 3l16 18" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />{/if}
  </svg>
{/snippet}

{#snippet recDot(blink)}
  <span class="rec-dot" style:opacity={blink && !blinkOn ? 0.25 : 1}></span>
{/snippet}

{#snippet battery(b, charging, sub)}
  <div class="batt-col">
    {#if b}
      <BatteryIcon value={b} {charging} size={22} />
    {/if}
    {#if sub}<div class="mono-sub">{sub}</div>{/if}
  </div>
{/snippet}

{#snippet control(s, value)}
  {@const locked = !!lockReason(s)}
  <div class="ctl" class:locked>
    {#if s.kind === "toggle"}
      <Switch checked={isOn(s, value)} disabled={locked} label={labelOf(s)} onchange={(on) => set(s.id, toggleValue(s, on))} />
    {:else if s.kind === "select"}
      <Stepper options={opts(s)} {value} disabled={locked} label={labelOf(s)} onchange={(v) => set(s.id, v)} />
    {:else}
      <Segmented options={opts(s)} {value} disabled={locked} label={labelOf(s)} width="200px" onchange={(v) => set(s.id, v)} />
    {/if}
  </div>
{/snippet}

<div class="panel">
  <!-- Header -------------------------------------------------------------->
  <header class="head">
    <DevicePicto pictogram={`${device.pictogram_key}-rx`} width={84} height={40} />
    <DeviceSwitcher {device} {devices} connected={status.connected} serial={rx?.serial ?? device.id} {onselect} />
    <div class="quick">
      {#if canMute}
        <button class="qbtn" class:warn={allMuted} onclick={muteAll}>
          {@render micIcon(true)}
          {allMuted ? $t("Unmute all") : $t("Mute all")}
        </button>
      {/if}
      {#if canRecord}
        <button class="qbtn" class:rec={allRecording} onclick={recordAll}>
          {#if allRecording}
            {@render recDot(true)}
            <span class="mono-num">{$t("Stop all")} · {clock(firstRecStart)}</span>
          {:else}
            {@render recDot(false)}
            {$t("Record all")}
          {/if}
        </button>
      {/if}
    </div>
  </header>

  <!-- Cards --------------------------------------------------------------->
  <section class="cards">
    <div class="card">
      <div class="card-head">
        <div class="art rx-art"><DevicePicto pictogram={`${device.pictogram_key}-rx`} width={64} height={30} /></div>
        <div class="titles">
          <div class="title">{$t("Receiver")}</div>
          {#if rxSubtitle}<div class="mono-sub">{rxSubtitle}</div>{/if}
        </div>
        {@render battery(
          rx?.battery,
          !!rx?.charging,
          rx?.environment
            ? rx.environment === "outdoors" ? $t("Outdoors") : $t("Indoors")
            : status.gain_dial != null ? `${$t("Gain")} ${formatDial(status.gain_dial)} dB` : null,
        )}
      </div>

      {#if settingsById["stereo"]}
        <div class="field">
          <div class="micro">{$t("Channels")}</div>
          <Segmented
            options={opts(settingsById["stereo"])}
            value={values.stereo ?? null}
            disabled={!!lockReason(settingsById["stereo"])}
            label={$t("Channels")}
            onchange={(v) => set("stereo", v)}
          />
        </div>
      {/if}
      {#if settingsById["rx-gain"]}
        <div class="field">
          <div class="micro">{$t("Output gain · dB")}</div>
          <Segmented options={opts(settingsById["rx-gain"])} value={values["rx-gain"] ?? null} mono label={$t("Output gain · dB")} onchange={(v) => set("rx-gain", v)} />
        </div>
      {/if}
      {#if settingsById["monitor-gain"] || settingsById["lossless"]}
        <div class="bottom-row">
          {#if settingsById["monitor-gain"]}
            <div class="field">
              <div class="micro">{$t("Headphones")}</div>
              <Stepper options={opts(settingsById["monitor-gain"])} value={values["monitor-gain"] ?? null} label={$t("Headphones")} onchange={(v) => set("monitor-gain", v)} />
            </div>
          {/if}
          {#if settingsById["lossless"]}
            <div class="inline-switch">
              <span class="micro">{$t("Lossless")}</span>
              <Switch
                checked={isOn(settingsById["lossless"], values.lossless)}
                label={$t("Lossless Audio")}
                onchange={(on) => set("lossless", toggleValue(settingsById["lossless"], on))}
              />
            </div>
          {/if}
        </div>
      {/if}
    </div>

    {#each visibleTx as { tx, i } (i)}
      <div class="card" class:busy={!!pendingTx[i]}>
        <div class="card-head">
          <div class="art tx-art">
            {#if tx}<DevicePicto pictogram={txPictogram(tx)} size={40} />{/if}
          </div>
          <div class="titles">
            <div class="title">{tx?.product_name && device.model_id !== "dji-mic-3" ? `${tx.product_name} (TX${i + 1})` : `TX${i + 1}`}</div>
            {#if tx?.serial}<div class="mono-sub">{tx.serial}</div>{/if}
          </div>
          {#if tx}
            {@render battery(tx.battery, !!tx.charging, tx.rec_remaining != null ? hoursLeft(tx.rec_remaining) : null)}
          {/if}
        </div>

        {#if tx}
          {@const muted = txValue(tx, "mute") === "on"}
          {@const recording = txValue(tx, "record") === "on"}
          {@const tone = txValue(tx, "voice-tone")}
          {@const gain = txValue(tx, "tx-gain")}
          <div class="meter-box">
            <LevelMeter level={levelOf(tx)} {muted} />
          </div>

          {#if settingsById["mute"] || settingsById["record"]}
            <div class="btn-row">
              {#if settingsById["mute"]}
                <button class="sbtn" class:warn={muted} onclick={() => onchangeTx?.(i, "mute", muted ? "off" : "on")}>
                  {@render micIcon(muted)}
                  {muted ? $t("Muted") : $t("Mute")}
                </button>
              {/if}
              {#if settingsById["record"]}
                <button class="sbtn" class:rec={recording} onclick={() => onchangeTx?.(i, "record", recording ? "off" : "on")}>
                  {#if recording}
                    {@render recDot(true)}
                    <span class="mono-num">{$t("REC")} {clock(recStartedAt[i] ?? now)}</span>
                  {:else}
                    {@render recDot(false)}
                    {$t("Record")}
                  {/if}
                </button>
              {/if}
            </div>
          {/if}

          {#if (settingsById["voice-tone"] && tone != null) || (settingsById["tx-gain"] && gain != null)}
            <div class="bottom-row tight">
              {#if settingsById["voice-tone"] && tone != null}
                <div class="field grow">
                  <div class="micro">{$t("Voice tone")}</div>
                  <Segmented options={opts(settingsById["voice-tone"])} value={tone} label={$t("Voice tone")} onchange={(v) => onchangeTx?.(i, "voice-tone", v)} />
                </div>
              {/if}
              {#if settingsById["tx-gain"] && gain != null}
                <div class="field">
                  <div class="micro">{$t("Input gain")}</div>
                  <Stepper options={opts(settingsById["tx-gain"])} value={gain} minWidth={52} label={$t("Input gain")} onchange={(v) => onchangeTx?.(i, "tx-gain", v)} />
                </div>
              {/if}
            </div>
          {/if}
        {:else}
          <div class="disconnected">{$t("Disconnected")}</div>
        {/if}
      </div>
    {/each}
  </section>

  <!-- Settings -------------------------------------------------------------->
  {#if groups.length > 0}
    <section class="settings">
      <div class="tabs-row">
        <div class="tabs" role="tablist">
          {#each groups as g (g)}
            <button class="tab" class:active={g === activeTab} role="tab" aria-selected={g === activeTab} onclick={() => ontab?.(g)}>
              {groupLabel(g)}
            </button>
          {/each}
        </div>
        <div class="scenes">
          <span class="micro scene-label">{$t("Scene")}</span>
          {#each SCENES as sc (sc.id)}
            {#if sceneEntries(sc).length > 0}
              <button class="chip" class:active={activeScene === sc.id} title={$t(sc.desc)} onclick={() => applyScene(sc)}>
                {$t(sc.label)}
              </button>
            {/if}
          {/each}
        </div>
      </div>

      <div class="tab-panel">
        {#if ncInTab}
          <div class="nc-row" class:busy={!!pending["noise-cancel"] || !!pending["noise-cancel-power"]}>
            <div class="text">
              <div class="label">{$t("Noise cancelling")}</div>
              {#if ncPowerLock}
                <div class="help warn-text">{ncPowerLock}</div>
              {:else if ncModeLock && !ncPower}
                <div class="help warn-text">{ncModeLock}</div>
              {:else if ncHelp}
                <div class="help">{ncHelp}</div>
              {/if}
            </div>
            {#if ncMode}
              <div class="nc-mode" class:locked={!!ncModeLock}>
                <span class="micro">{$t("Mode")}</span>
                <Segmented
                  options={opts(ncMode)}
                  value={values["noise-cancel"] ?? null}
                  disabled={!!ncModeLock}
                  width="180px"
                  label={$t("Mode")}
                  onchange={(v) => set("noise-cancel", v)}
                />
              </div>
            {/if}
            {#if ncPower}
              <span class="vdivider"></span>
              <div class="ctl" class:locked={!!ncPowerLock}>
                <Switch
                  checked={isOn(ncPower, values["noise-cancel-power"])}
                  disabled={!!ncPowerLock}
                  label={$t("Noise cancelling")}
                  onchange={(on) => set("noise-cancel-power", toggleValue(ncPower, on))}
                />
              </div>
            {/if}
          </div>
        {/if}

        <div class="grid">
          {#each rows as s, idx (s.id)}
            {@const reason = lockReason(s)}
            <div class="row" class:first={idx < 2} class:busy={!!pending[s.id]}>
              <div class="text">
                <div class="label">{labelOf(s)}</div>
                {#if reason}
                  <div class="help warn-text">{reason}</div>
                {:else if $help(s.id)}
                  <div class="help">{$help(s.id)}</div>
                {/if}
                {#if s.note}
                  <div class="note"><span class="note-dot"></span>{$t(s.note)}</div>
                {/if}
              </div>
              {@render control(s, values[s.id] ?? null)}
            </div>
          {/each}
        </div>
      </div>
    </section>
  {/if}
</div>

<style>
  .panel {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    padding: 20px 28px 28px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  /* Header */
  .head {
    display: flex;
    align-items: center;
    gap: 18px;
  }
  .head > :global(.picto) {
    flex: 0 0 auto;
    object-fit: contain;
  }
  .quick {
    margin-left: auto;
    display: flex;
    gap: 8px;
  }
  .qbtn {
    height: 38px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 0 14px;
    border-radius: 10px;
    border: 1px solid var(--line2);
    background: transparent;
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    transition: all 0.15s;
  }
  .qbtn.warn,
  .sbtn.warn {
    background: color-mix(in srgb, var(--warn) 16%, transparent);
    border-color: var(--warn);
    color: var(--warn);
  }
  .qbtn.rec,
  .sbtn.rec {
    background: color-mix(in srgb, var(--rec) 16%, transparent);
    border-color: var(--rec);
    color: var(--rec);
  }
  .rec-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--rec);
    flex: 0 0 auto;
    transition: opacity 0.2s;
  }
  .mono-num {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
  }

  /* Cards */
  .cards {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }
  .card {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 16px;
    padding: 18px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 14px;
    min-width: 0;
  }
  .card.busy {
    cursor: progress;
  }
  .card-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .art {
    display: grid;
    place-items: center;
    flex: 0 0 auto;
  }
  .rx-art {
    width: 64px;
    height: 44px;
  }
  .tx-art {
    width: 44px;
    height: 44px;
  }
  .art :global(.picto) {
    object-fit: contain;
  }
  .titles {
    flex: 1 1 auto;
    min-width: 0;
  }
  .title {
    font-size: 15px;
    font-weight: 600;
  }
  .mono-sub {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .batt-col {
    text-align: right;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .field.grow {
    flex: 1 1 auto;
  }
  .bottom-row {
    display: flex;
    align-items: flex-end;
    gap: 16px;
  }
  .bottom-row.tight {
    gap: 10px;
  }
  .inline-switch {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 32px;
  }
  .meter-box {
    min-height: 44px;
  }
  .btn-row {
    display: flex;
    gap: 8px;
  }
  .sbtn {
    flex: 1 1 0;
    height: 40px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border-radius: 10px;
    border: 1px solid transparent;
    background: var(--raise);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    transition: all 0.15s;
  }
  .disconnected {
    color: var(--dim);
    font-size: 13px;
  }

  /* Settings */
  .settings {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .tabs-row {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    flex-wrap: wrap;
    border-bottom: 1px solid var(--line);
  }
  .tabs {
    display: flex;
    gap: 26px;
  }
  .tab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    padding: 0 0 11px;
    font-size: 14px;
    font-weight: 500;
    color: var(--dim);
    white-space: nowrap;
    transition: color 0.15s, border-color 0.15s;
  }
  .tab:hover {
    color: var(--text);
  }
  .tab.active {
    color: var(--text);
    border-bottom-color: var(--text);
  }
  .scenes {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-bottom: 8px;
  }
  .scene-label {
    margin-right: 6px;
  }
  .chip {
    height: 28px;
    padding: 0 12px;
    border-radius: 999px;
    border: 1px solid var(--line2);
    background: transparent;
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    transition: all 0.15s;
  }
  .chip:hover:not(.active) {
    background: var(--raise);
  }
  .chip.active {
    background: var(--inv);
    border-color: var(--inv);
    color: var(--invInk);
  }
  .tab-panel {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 0 22px;
  }
  .nc-row {
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 16px 0;
    border-bottom: 1px solid var(--line);
  }
  .nc-mode {
    display: flex;
    align-items: center;
    gap: 12px;
    transition: opacity 0.18s;
  }
  .vdivider {
    width: 1px;
    height: 24px;
    background: var(--line2);
    flex: 0 0 auto;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    column-gap: 44px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 16px 0;
    border-top: 1px solid var(--line);
  }
  .row.first {
    border-top: none;
  }
  .busy {
    cursor: progress;
  }
  .text {
    flex: 1 1 auto;
    min-width: 0;
  }
  .label {
    font-size: 14px;
    font-weight: 500;
  }
  .help {
    margin-top: 3px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--dim);
    text-wrap: pretty;
  }
  .warn-text {
    color: var(--warn);
  }
  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 5px;
    font-size: 12px;
    color: var(--warn);
  }
  .note-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--warn);
    flex: 0 0 auto;
  }
  .ctl {
    flex: 0 0 auto;
    transition: opacity 0.18s;
  }
  .locked {
    opacity: 0.35;
    pointer-events: none;
  }

  @media (max-width: 980px) {
    .cards {
      grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    }
    .grid {
      grid-template-columns: minmax(0, 1fr);
    }
    .row:nth-child(2) {
      border-top: 1px solid var(--line);
    }
  }
</style>
