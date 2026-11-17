<script>
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { t, locale, selectLocale } from "./lib/i18n.js";
  import { LOCALES } from "./lib/translations.js";
  import DevicePanel from "./lib/DevicePanel.svelte";
  import UdevModal from "./lib/UdevModal.svelte";
  import Toast from "./lib/ui/Toast.svelte";
  import { snapshot, setSetting, setTxSetting, udevHelp } from "./lib/api.js";

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
      error = String(e);
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
      error = String(e);
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
      error = String(e);
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
          {/if}
        </div>
      </div>
    {/if}
  </main>

  <Toast {toast} />
</div>

{#if showUdev && udev}
  <UdevModal help={udev} onclose={() => (showUdev = false)} />
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
