<script>
  import { untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { t, tError, locale, selectLocale } from "./lib/i18n.js";
  import { LOCALES } from "./lib/translations.js";
  import DevicePanel from "./lib/DevicePanel.svelte";
  import UdevModal from "./lib/UdevModal.svelte";
  import DiagnosticsModal from "./lib/DiagnosticsModal.svelte";
  import SupportCard from "./lib/SupportCard.svelte";
  import RecordingsPanel from "./lib/RecordingsPanel.svelte";
  import WirelessWizard from "./lib/WirelessWizard.svelte";
  import * as recordings from "./lib/recordings.js";
  import { rxWireless, rxDevices, connectRx, cancelRx, disconnectRx, start as startRx } from "./lib/wirelessRx.js";
  import Toast from "./lib/ui/Toast.svelte";
  import { snapshot, setSetting, setTxSetting, udevHelp } from "./lib/api.js";
  import * as support from "./lib/support.js";

  // Custom window chrome. macOS keeps its native traffic lights (overlay
  // title bar); Windows/Linux draw their own controls and resize handles.
  const appWindow = getCurrentWindow();
  const ua = navigator.userAgent || navigator.platform || "";
  // Phones/tablets: no window chrome, no resize handles.
  const isMobile = /Android|iPhone|iPad/i.test(ua);
  const isMac = !isMobile && /Mac/i.test(ua);
  let maximized = $state(false);
  async function refreshMax() {
    try {
      maximized = await appWindow.isMaximized();
    } catch {
      /* ignore */
    }
  }
  const minimizeWin = () => appWindow.minimize();
  const toggleMaxWin = () => appWindow.toggleMaximize();
  const closeWin = () => appWindow.close();
  const startResize = (dir) => (e) => {
    if (e.button === 0) appWindow.startResizeDragging(dir);
  };

  // ---- Persisted UI preferences --------------------------------------------
  const THEME_KEY = "djimic.theme";
  const TAB_KEY = "djimic.tab";
  const load = (key) => {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  };
  const save = (key, value) => {
    try {
      localStorage.setItem(key, value);
    } catch {
      /* session-only */
    }
  };
  const systemTheme = () =>
    window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark";
  let theme = $state(load(THEME_KEY) === "light" || load(THEME_KEY) === "dark" ? load(THEME_KEY) : systemTheme());
  let settingsTab = $state(load(TAB_KEY));
  function toggleTheme() {
    theme = theme === "dark" ? "light" : "dark";
    save(THEME_KEY, theme);
  }
  function selectTab(tab) {
    settingsTab = tab;
    save(TAB_KEY, tab);
  }
  $effect(() => {
    document.documentElement.dataset.theme = theme;
    if (isMobile) {
      // Android: paint the status and navigation bars to match.
      const color = getComputedStyle(document.documentElement).getPropertyValue("--bg").trim();
      invoke("set_system_bars", { dark: theme === "dark", color }).catch(() => {});
    }
  });

  // ---- Device state -----------------------------------------------------------
  let snap = $state(null);
  let selected = $state(null);
  let pending = $state({});
  let optimistic = $state({}); // receiver/broadcast values shown before the device confirms
  let pendingTx = $state({});
  let optimisticTx = $state({}); // "slot:settingId" → value
  let error = $state(null);
  let toast = $state(null);
  let toastKey = 0;
  function showToast(msg, kind = "info") {
    toast = { msg, kind, key: ++toastKey };
  }

  let showUdev = $state(false);
  let showDiagnostics = $state(false);
  let udev = $state(null);

  const devices = $derived(snap?.devices ?? []);
  const selectedDevice = $derived(devices.find((d) => d.id === selected) ?? null);
  const rawStatus = $derived(selectedDevice ? snap?.status ?? null : null);
  const values = $derived({ ...(rawStatus?.settings ?? {}), ...optimistic });
  const status = $derived(
    rawStatus && {
      ...rawStatus,
      tx: rawStatus.tx.map((tx, i) => {
        if (!tx) return tx;
        const overrides = Object.entries(optimisticTx).filter(([k]) => k.startsWith(`${i}:`));
        if (overrides.length === 0) return tx;
        const settings = { ...(tx.settings ?? {}) };
        let voice_tone = tx.voice_tone;
        for (const [k, v] of overrides) {
          const id = k.slice(k.indexOf(":") + 1);
          settings[id] = v;
          if (id === "voice-tone") voice_tone = v;
        }
        return { ...tx, settings, voice_tone };
      }),
    },
  );
  const linuxAccessIssue = $derived(
    snap?.os === "linux" && snap?.probe?.permission_issue && devices.length === 0,
  );

  async function refresh() {
    try {
      const next = await snapshot(selected);
      snap = next;
      if (!selected && next.devices.length > 0) selected = next.devices[0].id;
      if (selected && !next.devices.some((d) => d.id === selected)) {
        selected = next.devices[0]?.id ?? null;
      }
      // Drop optimistic values once the device reports them.
      const confirmed = next.status?.settings ?? {};
      const remaining = Object.fromEntries(
        Object.entries(optimistic).filter(([k, v]) => confirmed[k] !== v),
      );
      if (Object.keys(remaining).length !== Object.keys(optimistic).length) optimistic = remaining;

      const txs = next.status?.tx ?? [];
      const remainingTx = Object.fromEntries(
        Object.entries(optimisticTx).filter(([k, v]) => {
          const [slot, id] = [Number(k.slice(0, k.indexOf(":"))), k.slice(k.indexOf(":") + 1)];
          const tx = txs[slot];
          const actual = id === "voice-tone" ? tx?.settings?.[id] ?? tx?.voice_tone : tx?.settings?.[id];
          return tx && actual !== v;
        }),
      );
      if (Object.keys(remainingTx).length !== Object.keys(optimisticTx).length) optimisticTx = remainingTx;
    } catch (e) {
      error = $tError(e);
    }
  }

  function select(id) {
    selected = id;
    optimistic = {};
    optimisticTx = {};
    refresh();
  }

  async function change(settingId, value) {
    if (!selected) return;
    optimistic = { ...optimistic, [settingId]: value };
    pending = { ...pending, [settingId]: true };
    error = null;
    try {
      await setSetting(selected, settingId, value);
      await refresh();
    } catch (e) {
      error = $tError(e);
      showToast(error, "error");
      const { [settingId]: _drop, ...rest } = optimistic;
      optimistic = rest;
    } finally {
      const { [settingId]: _p, ...rest } = pending;
      pending = rest;
    }
  }

  async function changeTx(tx, settingId, value) {
    if (!selected) return;
    const key = `${tx}:${settingId}`;
    optimisticTx = { ...optimisticTx, [key]: value };
    pendingTx = { ...pendingTx, [tx]: true };
    error = null;
    try {
      await setTxSetting(selected, tx, settingId, value);
      await refresh();
    } catch (e) {
      error = $tError(e);
      showToast(error, "error");
      const { [key]: _drop, ...rest } = optimisticTx;
      optimisticTx = rest;
    } finally {
      const { [tx]: _p, ...rest } = pendingTx;
      pendingTx = rest;
    }
  }

  async function openUdev() {
    if (!udev) udev = await udevHelp();
    showUdev = true;
  }

  $effect(() => {
    refresh();
    const timer = setInterval(refresh, 150);
    return () => clearInterval(timer);
  });

  $effect(() => {
    refreshMax();
    const onResize = () => refreshMax();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  });

  // ---- Ko-fi reminder (rules in support.js) --------------------------------
  let supportState = $state.raw(support.load());
  // While the card is on screen: what to restore if a recording starts, so
  // that showing it doesn't count.
  let supportCard = $state.raw(null);
  const anyRecording = $derived((status?.tx ?? []).some((tx) => tx?.settings?.record === "on"));
  function updateSupport(next) {
    supportState = next;
    support.save(next);
  }
  function openSupport() {
    support.openSupport().catch((e) => showToast($tError(e), "error"));
  }
  function supportLater() {
    supportCard = null; // already held back by markShown
  }
  function supportNever() {
    updateSupport(support.stop(supportState));
    supportCard = null;
  }

  // Bring the card up once the window has been in front for a little while
  // with a receiver and a transmitter linked: the user has seen it work.
  $effect(() => {
    let readySince = 0;
    const timer = setInterval(() => {
      const now = Date.now();
      if (!support.working(status) || !document.hasFocus()) {
        readySince = 0;
        return;
      }
      readySince ||= now;
      if (
        !supportCard &&
        !anyRecording &&
        !linuxAccessIssue &&
        now - readySince >= support.DELAY_MS &&
        support.shouldShow(supportState, now)
      ) {
        supportCard = { shown: supportState.shown, snoozeUntil: supportState.snoozeUntil };
        updateSupport(support.markShown(supportState, now));
      }
    }, 1000);
    return () => clearInterval(timer);
  });

  // Never in the way of a take: hide it and give the show back.
  $effect(() => {
    if (anyRecording && supportCard) {
      updateSupport({ ...supportState, ...supportCard });
      supportCard = null;
    }
  });

  // Whichever button or menu item opened Ko-fi, stop asking.
  $effect(() => {
    let unlisten;
    let gone = false;
    listen("support-opened", () => {
      updateSupport(support.stop(supportState));
      supportCard = null;
    })
      .then((fn) => (gone ? fn() : (unlisten = fn)))
      .catch(() => {});
    return () => {
      gone = true;
      unlisten?.();
    };
  });

  // ---- Transmitter recordings (desktop: transmitters mount as USB drives) ----
  const { drives: recDrives, kind: recKind, job: recJob } = recordings;
  let showRecordings = $state(false);
  const newTakes = $derived(
    $recDrives.reduce((n, tx) => n + tx.takes.filter((take) => recordings.isNew(take, $recKind)).length, 0),
  );
  if (!isMobile) recordings.start().catch(() => {});
  if (!isMobile) startRx().catch(() => {});

  // ---- Receiver over Wi-Fi ---------------------------------------------------
  let showRxWizard = $state(false);
  const wirelessSelected = $derived(selectedDevice && $rxDevices.includes(selectedDevice.id));
  let rxDisconnecting = $state(false);
  async function connectReceiver() {
    const id = await connectRx();
    if (id) {
      showRxWizard = false;
      select(id);
      showToast($t("Receiver connected without a cable"));
    }
  }
  async function disconnectReceiver() {
    if (!selectedDevice) return;
    rxDisconnecting = true;
    try {
      await disconnectRx(selectedDevice.id);
      showToast($t("Disconnected. Your Wi-Fi is back to its usual network."));
    } catch (e) {
      showToast($tError(e), "error");
    } finally {
      rxDisconnecting = false;
    }
  }
  function closeRxWizard() {
    if ($rxWireless.phase === "connecting") cancelRx();
    showRxWizard = false;
  }
  // Say so when transmitters with new recordings get plugged in.
  let seenDrives = new Set();
  $effect(() => {
    const now = new Set($recDrives.map((tx) => tx.mount));
    const added = [...now].some((m) => !seenDrives.has(m));
    seenDrives = now;
    if (added && newTakes > 0 && !showRecordings) {
      showToast(
        newTakes === 1
          ? $t("1 new recording on the transmitters")
          : $t("{n} new recordings on the transmitters", { n: newTakes }),
      );
    }
  });
  // An import finishing while the panel is closed.
  let handledOutcome = null;
  $effect(() => {
    const o = $recJob.outcome;
    if (!o || o === handledOutcome) return;
    handledOutcome = o;
    if (untrack(() => showRecordings) || o.cancelled) return;
    if (o.failed.length > 0) showToast(`${o.failed[0].file}: ${$tError(o.failed[0].error)}`, "error");
    else showToast(o.copied === 1 ? $t("1 file imported") : $t("{n} files imported", { n: o.copied }));
  });

  // Keep <html lang> and the native tray/app menus in the chosen language.
  $effect(() => {
    document.documentElement.lang = $locale;
    invoke("set_locale", { locale: $locale }).catch(() => {});
  });
  const localeCode = $derived(($locale ?? "en").split("-")[0].toUpperCase());
</script>

<div class="app" class:mac={isMac} class:mobile={isMobile}>
  {#if !isMac && !isMobile}
    <div class="rz rz-n" onmousedown={startResize("North")}></div>
    <div class="rz rz-s" onmousedown={startResize("South")}></div>
    <div class="rz rz-e" onmousedown={startResize("East")}></div>
    <div class="rz rz-w" onmousedown={startResize("West")}></div>
    <div class="rz rz-ne" onmousedown={startResize("NorthEast")}></div>
    <div class="rz rz-nw" onmousedown={startResize("NorthWest")}></div>
    <div class="rz rz-se" onmousedown={startResize("SouthEast")}></div>
    <div class="rz rz-sw" onmousedown={startResize("SouthWest")}></div>
  {/if}

  <div class="topbar" class:mac={isMac} data-tauri-drag-region>
    <span class="brand" data-tauri-drag-region>DJI Mic Control</span>
    <div class="drag-fill" data-tauri-drag-region></div>
    <div class="top-actions">
      {#if !isMobile}
        <button
          class="icon-btn rec-btn"
          class:busy={$recJob.running}
          onclick={() => (showRecordings = true)}
          aria-label={$t("Recordings")}
          title={$t("Recordings")}
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="M12 4v10m0 0l-4-4m4 4l4-4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
            <path d="M5 15v2.5A2.5 2.5 0 0 0 7.5 20h9a2.5 2.5 0 0 0 2.5-2.5V15" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
          </svg>
          {#if newTakes > 0}<span class="badge">{newTakes > 99 ? "99+" : newTakes}</span>{/if}
        </button>
      {/if}
      <button class="icon-btn support-btn" onclick={openSupport} aria-label={$t("Buy me a coffee")} title={$t("Buy me a coffee")}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M12 20s-7.5-4.6-7.5-10.1A4.4 4.4 0 0 1 12 7.3a4.4 4.4 0 0 1 7.5 2.6C19.5 15.4 12 20 12 20z"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linejoin="round"
          />
        </svg>
      </button>
      <button class="icon-btn" onclick={() => (showDiagnostics = true)} aria-label={$t("Diagnostic report")} title={$t("Diagnostic report")}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <circle cx="12" cy="12" r="8.5" stroke="currentColor" stroke-width="1.6" />
          <path d="M9.6 9.4a2.5 2.5 0 0 1 4.8.9c0 1.7-2.4 2.1-2.4 3.6" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
          <circle cx="12" cy="16.9" r="1" fill="currentColor" />
        </svg>
      </button>
      <button class="icon-btn" onclick={toggleTheme} aria-label={$t("Toggle theme")} title={$t("Toggle theme")}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <circle cx="12" cy="12" r="8.5" stroke="currentColor" stroke-width="1.6" />
          <path d="M12 3.5a8.5 8.5 0 0 0 0 17z" fill="currentColor" />
        </svg>
      </button>
      <label class="language" title={$t("Language")}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="1.6" />
          <path d="M3 12h18M12 3c2.6 2.6 3.9 5.6 3.9 9s-1.3 6.4-3.9 9c-2.6-2.6-3.9-5.6-3.9-9S9.4 5.6 12 3z" stroke="currentColor" stroke-width="1.6" />
        </svg>
        {localeCode}
        <select aria-label={$t("Language")} value={$locale} onchange={(e) => selectLocale(e.currentTarget.value)}>
          {#each LOCALES as l (l.code)}
            <option value={l.code}>{l.name}</option>
          {/each}
        </select>
      </label>
    </div>

    {#if !isMac && !isMobile}
      <div class="win-controls">
        <button class="win-btn" onclick={minimizeWin} aria-label={$t("Minimize")}>
          <svg width="11" height="11" viewBox="0 0 12 12"><path d="M2 6h8" stroke="currentColor" stroke-width="1.2" /></svg>
        </button>
        <button class="win-btn" onclick={toggleMaxWin} aria-label={maximized ? $t("Restore") : $t("Maximize")}>
          {#if maximized}
            <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.1">
              <rect x="3.2" y="1.6" width="6.2" height="6.2" rx="1" /><rect x="1.6" y="3.6" width="6.2" height="6.8" rx="1" fill="var(--bg)" />
            </svg>
          {:else}
            <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.1"><rect x="2" y="2" width="8" height="8" rx="1" /></svg>
          {/if}
        </button>
        <button class="win-btn close" onclick={closeWin} aria-label={$t("Close")}>
          <svg width="11" height="11" viewBox="0 0 12 12"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" /></svg>
        </button>
      </div>
    {/if}
  </div>

  {#if linuxAccessIssue}
    <button class="banner" onclick={openUdev}>
      {$t("A microphone is connected but not accessible — click to fix USB permissions.")}
    </button>
  {/if}

  {#if wirelessSelected}
    <div class="air-bar">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 9a12 12 0 0 1 16 0M7 12.5a7.5 7.5 0 0 1 10 0M10 16a3 3 0 0 1 4 0" stroke="currentColor" stroke-width="2" stroke-linecap="round" /><circle cx="12" cy="19" r="1.4" fill="currentColor" /></svg>
      <span>{$t("Connected without a cable, over the receiver's Wi-Fi. The transmitters stay disconnected in this mode, and this computer has no Internet over Wi-Fi meanwhile.")}</span>
      <button onclick={disconnectReceiver} disabled={rxDisconnecting}>{$t("Disconnect")}</button>
    </div>
  {/if}

  <main class="main">
    {#if selectedDevice && status}
      <DevicePanel
        device={selectedDevice}
        {devices}
        {status}
        settings={snap.settings}
        {values}
        {pending}
        {pendingTx}
        tab={settingsTab}
        onchange={change}
        onchangeTx={changeTx}
        onselect={select}
        ontab={selectTab}
        ontoast={showToast}
      />
    {:else}
      <div class="placeholder">
        <div class="ph-card">
          {#if linuxAccessIssue}
            <h2>{$t("Permission needed")}</h2>
            <p>{$t("A supported microphone is connected but can't be accessed.")}</p>
            <button class="cta" onclick={openUdev}>{$t("Show setup instructions")}</button>
          {:else}
            <h2>{$t("No microphone connected")}</h2>
            <p>{$t("Connect a supported DJI microphone over USB to get started.")}</p>
            {#if !isMobile}
              <button class="cta-air" onclick={() => (showRxWizard = true)}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M7 7l10 10-5 5V2l5 5L7 17" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
                {$t("Connect a DJI Mic 3 without a cable")}
              </button>
            {/if}
          {/if}
          <button class="ph-diag" onclick={() => (showDiagnostics = true)}>
            {$t("Microphone not detected? Get a diagnostic report")}
          </button>
        </div>
      </div>
    {/if}
  </main>

  {#if supportCard}
    <SupportCard onopen={openSupport} onlater={supportLater} onnever={supportNever} />
  {/if}

  {#if showRecordings}
    <RecordingsPanel onclose={() => (showRecordings = false)} ontoast={showToast} />
  {/if}

  {#if showRxWizard}
    <div class="rx-overlay" onclick={closeRxWizard} role="presentation">
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div class="rx-sheet" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" aria-label={$t("Connect the receiver without a cable")}>
        <button class="rx-x" onclick={closeRxWizard} aria-label={$t("Close")}>
          <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" /></svg>
        </button>
        <WirelessWizard unit="rx" store={rxWireless} onstart={connectReceiver} oncancel={cancelRx} onclose={closeRxWizard} />
      </div>
    </div>
  {/if}

  <Toast {toast} />
</div>

{#if showUdev && udev}
  <UdevModal help={udev} onclose={() => (showUdev = false)} />
{/if}

{#if showDiagnostics}
  <DiagnosticsModal onclose={() => (showDiagnostics = false)} />
{/if}

<style>
  .app {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    /* Frameless windows (Windows/Linux) need their own edge. */
    border: 1px solid var(--line);
  }
  .app.mac,
  .app.mobile {
    border: none;
  }
  .app.mobile .topbar {
    padding: 0 10px 0 14px;
  }
  /* Phones: the brand would collide with the controls on the right. */
  @media (max-width: 400px) {
    .brand {
      position: static;
      transform: none;
    }
  }
  .rz {
    position: fixed;
    z-index: 100;
  }
  .rz-n,
  .rz-s {
    left: 8px;
    right: 8px;
    height: 5px;
    cursor: ns-resize;
  }
  .rz-n {
    top: 0;
  }
  .rz-s {
    bottom: 0;
  }
  .rz-e,
  .rz-w {
    top: 8px;
    bottom: 8px;
    width: 5px;
    cursor: ew-resize;
  }
  .rz-e {
    right: 0;
  }
  .rz-w {
    left: 0;
  }
  .rz-ne,
  .rz-nw,
  .rz-se,
  .rz-sw {
    width: 11px;
    height: 11px;
  }
  .rz-ne {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }
  .rz-nw {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }
  .rz-se {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
  }
  .rz-sw {
    bottom: 0;
    left: 0;
    cursor: nesw-resize;
  }

  .topbar {
    position: relative;
    flex: 0 0 auto;
    height: 40px;
    display: flex;
    align-items: center;
    padding: 0 6px 0 14px;
    background: var(--bg);
    border-bottom: 1px solid var(--line);
    user-select: none;
  }
  .topbar.mac {
    padding-left: 76px;
    padding-right: 14px;
  }
  .brand {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    font-size: 12px;
    font-weight: 500;
    color: var(--dim);
    white-space: nowrap;
  }
  .drag-fill {
    flex: 1 1 auto;
    align-self: stretch;
  }
  .top-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .icon-btn {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--dim);
    transition: background 0.15s, color 0.15s;
  }
  .icon-btn:hover {
    background: var(--raise);
    color: var(--text);
  }
  .support-btn:hover {
    color: var(--rec);
  }
  .rec-btn {
    position: relative;
  }
  .rec-btn.busy {
    color: var(--text);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .badge {
    position: absolute;
    top: 1px;
    right: -2px;
    min-width: 15px;
    height: 15px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--inv);
    color: var(--invInk);
    font-family: var(--mono);
    font-size: 9.5px;
    font-weight: 600;
    line-height: 15px;
    text-align: center;
    pointer-events: none;
  }
  @keyframes pulse {
    50% {
      opacity: 0.45;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .rec-btn.busy {
      animation: none;
    }
  }
  .topbar svg {
    display: block;
  }
  .language {
    position: relative;
    height: 26px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border: 1px solid var(--line2);
    border-radius: 999px;
    color: var(--dim);
    font-family: var(--mono);
    font-size: 11px;
    cursor: pointer;
    transition: color 0.15s;
  }
  .language:hover {
    color: var(--text);
  }
  /* The native select sits invisibly on top: the pill shows the code, the
     dropdown lists full names. */
  .language select {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
  .win-controls {
    display: flex;
    align-self: stretch;
    margin-left: 8px;
  }
  .win-btn {
    width: 40px;
    align-self: stretch;
    display: grid;
    place-items: center;
    border: none;
    background: transparent;
    color: var(--dim);
    transition: background 0.12s, color 0.12s;
  }
  .win-btn:hover {
    background: var(--raise);
    color: var(--text);
  }
  .win-btn.close:hover {
    background: #e5484d;
    color: #fff;
  }
  .banner {
    flex: 0 0 auto;
    text-align: left;
    border: none;
    padding: 10px 16px;
    background: color-mix(in srgb, var(--warn) 16%, var(--bg));
    color: var(--text);
    border-bottom: 1px solid color-mix(in srgb, var(--warn) 40%, transparent);
    font-size: 13px;
  }
  .main {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
  }
  .placeholder {
    flex: 1 1 auto;
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .ph-card {
    text-align: center;
    max-width: 360px;
    color: var(--dim);
  }
  .ph-card h2 {
    margin: 0 0 8px;
    color: var(--text);
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .ph-card p {
    margin: 0 0 16px;
    line-height: 1.5;
  }
  .cta-air {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 16px;
    border: 1px solid var(--line2);
    border-radius: 10px;
    background: transparent;
    color: var(--text);
    font-weight: 500;
    transition: background 0.15s;
  }
  .cta-air:hover {
    background: var(--raise);
  }
  .ph-diag {
    display: block;
    margin: 18px auto 0;
    padding: 4px 6px;
    border: none;
    background: transparent;
    color: var(--faint);
    font-size: 12.5px;
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .ph-diag:hover {
    color: var(--text);
  }
  .air-bar {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 16px;
    background: color-mix(in srgb, var(--good) 12%, var(--bg));
    border-bottom: 1px solid color-mix(in srgb, var(--good) 30%, transparent);
    color: var(--text);
    font-size: 12.5px;
  }
  .air-bar svg {
    color: var(--good);
    flex: 0 0 auto;
  }
  .air-bar span {
    flex: 1 1 auto;
  }
  .air-bar button {
    height: 26px;
    padding: 0 12px;
    border: 1px solid var(--line2);
    border-radius: 8px;
    background: var(--panel);
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
  }
  .rx-overlay {
    position: absolute;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(8, 10, 14, 0.45);
  }
  .rx-sheet {
    position: relative;
    width: min(760px, 100%);
    max-height: 100%;
    overflow-y: auto;
    padding: 44px 20px 14px;
    background: var(--panel);
    border: 1px solid var(--line2);
    border-radius: 16px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35);
  }
  .rx-x {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 1;
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--dim);
  }
  .rx-x:hover {
    background: var(--raise);
    color: var(--text);
  }
  .cta {
    height: 38px;
    padding: 0 16px;
    border: none;
    border-radius: 10px;
    background: var(--inv);
    color: var(--invInk);
    font-weight: 500;
  }
</style>
