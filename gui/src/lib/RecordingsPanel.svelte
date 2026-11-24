<script>
  // Transmitter recordings: list the takes on every transmitter drive,
  // import the chosen ones, delete what's been imported, eject the drives.
  // State and commands live in recordings.js.
  import { untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { t, tError, locale } from "./i18n.js";
  import Segmented from "./ui/Segmented.svelte";
  import WirelessWizard from "./WirelessWizard.svelte";
  import {
    wireless,
    drives,
    dest,
    kind,
    job,
    takeId,
    filesFor,
    isNew,
    pickDest,
    importFiles,
    cancelImport,
    clearOutcome,
    deleteFiles,
    openDest,
    eject,
  } from "./recordings.js";

  let { onclose, ontoast } = $props();

  // ---- Formatting -------------------------------------------------------------
  function bytes(n) {
    if (n >= 1e9) return `${(n / 1e9).toFixed(1)} GB`;
    if (n >= 1e6) return `${(n / 1e6).toFixed(1)} MB`;
    return `${Math.max(1, Math.round(n / 1e3))} kB`;
  }
  function duration(secs) {
    // A take under a second still reads as 0:01, not 0:00.
    const s = secs > 0 ? Math.max(1, Math.round(secs)) : 0;
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const ss = String(s % 60).padStart(2, "0");
    return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
  }
  const dateFormat = $derived(new Intl.DateTimeFormat($locale, { dateStyle: "medium", timeStyle: "short" }));
  function when(iso) {
    if (!iso) return $t("Undated");
    const [d, tm] = iso.split("T");
    const [y, mo, da] = d.split("-").map(Number);
    const [h, mi, s] = tm.split(":").map(Number);
    return dateFormat.format(new Date(y, mo - 1, da, h, mi, s));
  }
  function format(info) {
    if (!info) return "—";
    const bits = info.float ? $t("{bits}-bit float", { bits: info.bits_per_sample }) : $t("{bits}-bit", { bits: info.bits_per_sample });
    return `${bits} · ${info.sample_rate / 1000} kHz`;
  }
  const count = (n, one, many) => (n === 1 ? $t(one) : $t(many, { n }));
  const txName = (tx) => (tx.slot ? `TX${tx.slot}` : "TX");

  // ---- Selection ----------------------------------------------------------------
  const allTakes = $derived($drives.flatMap((tx) => tx.takes.map((take) => ({ tx, take, id: takeId(tx, take) }))));
  const hasProcessed = $derived(allTakes.some(({ take }) => take.files.some((f) => f.kind === "processed")));
  const selected = new SvelteSet();
  // Until the user picks by hand, the selection follows "everything new".
  let touched = $state(false);
  $effect(() => {
    if (touched) return;
    const fresh = allTakes.filter(({ take }) => isNew(take, $kind)).map((x) => x.id);
    untrack(() => {
      selected.clear();
      for (const id of fresh) selected.add(id);
    });
  });
  // Forget takes that are gone (deleted, drive ejected).
  $effect(() => {
    const present = new Set(allTakes.map((x) => x.id));
    untrack(() => {
      for (const id of [...selected]) if (!present.has(id)) selected.delete(id);
    });
  });
  function toggle(id) {
    touched = true;
    if (selected.has(id)) selected.delete(id);
    else selected.add(id);
  }
  function selectWhere(pred) {
    touched = true;
    selected.clear();
    for (const x of allTakes) if (pred(x)) selected.add(x.id);
  }
  const chosen = $derived(allTakes.filter((x) => selected.has(x.id)));
  const toCopy = $derived(chosen.flatMap(({ take }) => filesFor(take, $kind)).filter((f) => !f.imported));
  const copyBytes = $derived(toCopy.reduce((n, f) => n + f.size, 0));
  const copyTakes = $derived(chosen.filter(({ take }) => isNew(take, $kind)).length);
  // Files can only be deleted from transmitters mounted as drives.
  const deletable = $derived(chosen.filter(({ tx }) => !tx.wireless).flatMap(({ take }) => take.files).filter((f) => f.imported));

  // ---- Wireless ------------------------------------------------------------------
  let wizardOpen = $state(false);
  const showWizard = $derived(wizardOpen || $wireless.phase === "connecting" || $wireless.phase === "error");
  $effect(() => {
    if ($wireless.phase === "connected") wizardOpen = false;
  });
  const allSelected = $derived(allTakes.length > 0 && chosen.length === allTakes.length);

  // ---- Actions --------------------------------------------------------------------
  const percent = $derived(
    $job.progress && $job.progress.total_bytes > 0 ? Math.floor(($job.progress.done_bytes / $job.progress.total_bytes) * 100) : 0,
  );

  async function changeDest() {
    try {
      await pickDest();
    } catch (e) {
      ontoast?.($tError(e), "error");
    }
  }
  async function open() {
    try {
      await openDest();
    } catch (e) {
      ontoast?.($tError(e), "error");
    }
  }
  async function startImport() {
    confirmDelete = false;
    // Keep these takes selected so they can be deleted once imported.
    touched = true;
    try {
      await importFiles(toCopy.map((f) => f.path));
    } catch (e) {
      ontoast?.($tError(e), "error");
    }
  }

  let confirmDelete = $state(false);
  let confirmTimer;
  async function remove() {
    if (!confirmDelete) {
      confirmDelete = true;
      clearTimeout(confirmTimer);
      confirmTimer = setTimeout(() => (confirmDelete = false), 5000);
      return;
    }
    confirmDelete = false;
    clearTimeout(confirmTimer);
    try {
      const r = await deleteFiles(deletable.map((f) => f.path));
      clearOutcome();
      if (r.failed.length > 0) {
        ontoast?.(`${r.failed[0].file}: ${$tError(r.failed[0].error)}`, "error");
      } else {
        ontoast?.(count(r.deleted, "1 file deleted from the transmitters", "{n} files deleted from the transmitters"));
      }
    } catch (e) {
      ontoast?.($tError(e), "error");
    }
  }

  let ejecting = $state(null);
  async function ejectDrive(tx) {
    ejecting = tx.mount;
    try {
      await eject(tx.mount);
      ontoast?.(tx.wireless ? $t("Disconnected. Your Wi-Fi is back to its usual network.") : $t("{tx} ejected. You can unplug it.", { tx: txName(tx) }));
    } catch (e) {
      ontoast?.($tError(e), "error");
    } finally {
      ejecting = null;
    }
  }

  function onkey(e) {
    if (e.key === "Escape") onclose?.();
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="overlay" onclick={onclose} role="presentation">
  <!-- Escape closes it (window handler); the click only stops the overlay closing it. -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="sheet" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="rec-title">
    <header>
      <div>
        <h2 id="rec-title">{$t("Recordings")}</h2>
        <p class="sub">{$t("Internal recordings on the transmitters, over USB or over the air.")}</p>
      </div>
      {#if !showWizard && $wireless.phase !== "connected"}
        <button class="ghost wl" onclick={() => (wizardOpen = true)}>
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 9a12 12 0 0 1 16 0M7 12.5a7.5 7.5 0 0 1 10 0M10 16a3 3 0 0 1 4 0" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /><circle cx="12" cy="19" r="1.3" fill="currentColor" /></svg>
          {$t("Without the case")}
        </button>
      {/if}
      <button class="x" onclick={onclose} aria-label={$t("Close")}>
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" /></svg>
      </button>
    </header>

    <div class="dest">
      <span class="micro">{$t("Import to")}</span>
      <!-- rtl keeps the end of a long path visible; the bdi keeps its slashes in order. -->
      <code title={$dest}><bdi dir="ltr">{$dest || $t("Choose a folder")}</bdi></code>
      <button class="ghost" onclick={changeDest} disabled={$job.running}>{$t("Change…")}</button>
      <button class="ghost" onclick={open} disabled={!$dest}>{$t("Open folder")}</button>
    </div>

    <div class="body">
      {#if showWizard}
        <WirelessWizard onclose={() => (wizardOpen = false)} />
      {:else if $drives.length === 0}
        <div class="empty">
          <svg width="40" height="40" viewBox="0 0 40 40" fill="none" aria-hidden="true">
            <rect x="6" y="13" width="28" height="18" rx="5" stroke="currentColor" stroke-width="1.5" />
            <rect x="12" y="17" width="6" height="10" rx="2" stroke="currentColor" stroke-width="1.5" />
            <rect x="22" y="17" width="6" height="10" rx="2" stroke="currentColor" stroke-width="1.5" />
            <path d="M20 31v4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          </svg>
          <h3>{$t("No transmitters connected")}</h3>
          <p>{$t("Put the transmitters in the charging case and connect the case to this computer with a USB-C cable. Each transmitter shows up as a drive.")}</p>
          <button class="air-card" onclick={() => (wizardOpen = true)}>
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 9a12 12 0 0 1 16 0M7 12.5a7.5 7.5 0 0 1 10 0M10 16a3 3 0 0 1 4 0" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /><circle cx="12" cy="19" r="1.3" fill="currentColor" /></svg>
            <span>
              <strong>{$t("No case at hand? Download without cables")}</strong>
              <small>{$t("Over Bluetooth and the transmitter's own Wi-Fi. A guided walkthrough shows each step.")}</small>
            </span>
            <svg class="chev" width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M9 6l6 6-6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></svg>
          </button>
        </div>
      {:else}
        {#each $drives as tx (tx.mount)}
          <section>
            <div class="tx-head">
              <span class="tx-name">{txName(tx)}</span>
              {#if tx.wireless}<span class="chip">Wi-Fi</span>{/if}
              <span class="tx-meta">
                {[tx.model, tx.firmware && `${$t("Firmware")} ${tx.firmware}`, count(tx.takes.length, "1 recording", "{n} recordings")]
                  .filter(Boolean)
                  .join(" · ")}
              </span>
              <button class="ghost small" onclick={() => ejectDrive(tx)} disabled={$job.running || ejecting === tx.mount}>
                {#if tx.wireless}
                  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M4 9a12 12 0 0 1 16 0M7 12.5a7.5 7.5 0 0 1 10 0M10 16a3 3 0 0 1 4 0M3 3l18 18" stroke="currentColor" stroke-width="2" stroke-linecap="round" /></svg>
                  {$t("Disconnect")}
                {:else}
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor" aria-hidden="true"><path d="M6 2l4 4.5H2z" /><rect x="2" y="8" width="8" height="1.6" rx=".5" /></svg>
                  {$t("Eject")}
                {/if}
              </button>
            </div>
            <ul>
              {#each tx.takes as take (take.key)}
                {@const id = takeId(tx, take)}
                {@const fresh = isNew(take, $kind)}
                {@const size = filesFor(take, $kind).reduce((n, f) => n + f.size, 0)}
                <li>
                  <label class="row" class:on={selected.has(id)}>
                    <input type="checkbox" checked={selected.has(id)} onchange={() => toggle(id)} disabled={$job.running} />
                    <span class="when">{when(take.recorded_at)}</span>
                    <span class="dur">{duration(take.info?.duration_secs)}</span>
                    <span class="fmt">{format(take.info)}</span>
                    <span class="tags">
                      {#if take.files.some((f) => f.kind === "processed")}<span class="tag">{$t("Processed")}</span>{/if}
                    </span>
                    <span class="size">{bytes(size)}</span>
                    <span class="state" class:done={!fresh}>
                      {#if fresh}{$t("New")}{:else}
                        <svg width="11" height="11" viewBox="0 0 12 12" fill="none" aria-hidden="true"><path d="M2.5 6.2l2.3 2.3 4.7-5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg>
                        {$t("Imported")}
                      {/if}
                    </span>
                  </label>
                </li>
              {/each}
            </ul>
          </section>
        {/each}
      {/if}
    </div>

    {#if $drives.length > 0 && !showWizard}
      <footer>
        {#if $job.running}
          <div class="progress">
            <div class="bar"><div class="fill" style:width="{percent}%"></div></div>
            <div class="progress-text">
              <span>
                {#if $job.progress?.file}
                  {$t("Copying {file} · {i} of {n}", { file: $job.progress.file, i: $job.progress.index + 1, n: $job.progress.count })}
                {:else}
                  {$t("Preparing…")}
                {/if}
              </span>
              <span class="pct">{percent}%</span>
            </div>
          </div>
          <button class="ghost" onclick={cancelImport}>{$t("Cancel")}</button>
        {:else}
          <div class="tools">
            {#if hasProcessed}
              <Segmented
                label={$t("Files to import")}
                width="260px"
                value={$kind}
                options={[
                  { value: "original", label: $t("Original") },
                  { value: "processed", label: $t("Processed") },
                  { value: "both", label: $t("Both") },
                ]}
                onchange={(v) => kind.set(v)}
              />
            {/if}
            <button class="link" onclick={() => (allSelected ? selectWhere(() => false) : selectWhere(() => true))}>
              {allSelected ? $t("Select none") : $t("Select all")}
            </button>
            <button class="link" onclick={() => selectWhere(({ take }) => isNew(take, $kind))}>{$t("Select new")}</button>
          </div>
          <div class="actions">
            {#if $job.outcome}
              <span class="outcome" class:bad={$job.outcome.failed.length > 0}>
                {#if $job.outcome.cancelled}
                  {$t("Import cancelled")}
                {:else if $job.outcome.failed.length > 0}
                  {$t("{n} failed", { n: $job.outcome.failed.length })} · {$tError($job.outcome.failed[0].error)}
                {:else}
                  {count($job.outcome.copied, "1 file imported", "{n} files imported")}
                {/if}
              </span>
            {/if}
            {#if deletable.length > 0}
              <button class="danger" class:armed={confirmDelete} onclick={remove}>
                {confirmDelete
                  ? $t("Click again to delete. This can't be undone.")
                  : count(deletable.length, "Delete 1 imported file from the transmitters", "Delete {n} imported files from the transmitters")}
              </button>
            {/if}
            <button class="primary" onclick={startImport} disabled={toCopy.length === 0 || !$dest}>
              {#if toCopy.length === 0}
                {$t("Nothing new to import")}
              {:else}
                {count(copyTakes, "Import 1 recording", "Import {n} recordings")} · {bytes(copyBytes)}
              {/if}
            </button>
          </div>
        {/if}
      </footer>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: absolute;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(8, 10, 14, 0.45);
    animation: fade 0.15s ease-out;
  }
  .sheet {
    width: min(760px, 100%);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line2);
    border-radius: 16px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35);
    overflow: hidden;
    animation: rise 0.2s ease-out;
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px 12px;
  }
  h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .sub {
    margin: 4px 0 0;
    color: var(--dim);
  }
  header > div {
    flex: 1 1 auto;
  }
  .wl {
    height: 30px;
    padding: 0 12px;
    border: 1px solid var(--line2) !important;
    border-radius: 999px !important;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    font-weight: 500;
    background: transparent;
    color: var(--text);
  }
  .wl:hover {
    background: var(--raise);
  }
  .chip {
    padding: 2px 7px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--good) 18%, transparent);
    color: var(--good);
    font-family: var(--mono);
    font-size: 10px;
    font-weight: 600;
  }
  .air-card {
    display: flex;
    align-items: center;
    gap: 14px;
    width: 100%;
    margin-top: 20px;
    padding: 14px 16px;
    border: 1px solid var(--line2);
    border-radius: 12px;
    background: var(--raise);
    color: var(--text);
    text-align: left;
    transition: border-color 0.15s, transform 0.15s;
  }
  .air-card:hover {
    border-color: var(--inv);
  }
  .air-card span {
    flex: 1 1 auto;
    display: grid;
    gap: 3px;
  }
  .air-card small {
    color: var(--dim);
    font-size: 12px;
    line-height: 1.4;
  }
  .air-card .chev {
    color: var(--dim);
  }
  .x {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--dim);
  }
  .x:hover {
    background: var(--raise);
    color: var(--text);
  }
  .dest {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 20px 4px;
    padding: 6px 6px 6px 12px;
    background: var(--raise);
    border-radius: 10px;
  }
  .dest code {
    flex: 1 1 auto;
    min-width: 0;
    font-family: var(--mono);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .body {
    flex: 1 1 auto;
    min-height: 120px;
    overflow-y: auto;
    padding: 8px 20px 16px;
  }
  section + section {
    margin-top: 10px;
  }
  .tx-head {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 0 8px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }
  .tx-name {
    font-family: var(--mono);
    font-size: 13px;
    font-weight: 600;
  }
  .tx-meta {
    flex: 1 1 auto;
    min-width: 0;
    color: var(--dim);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .row {
    display: grid;
    /* Every row is its own grid, so the columns are fixed to line up. The
       format column fits the longest label ("32 bits flottant · 48 kHz",
       25 monospaced characters); the date gives way first. */
    grid-template-columns: 18px minmax(96px, 1fr) 56px 184px auto 64px 84px;
    align-items: center;
    gap: 12px;
    padding: 8px 8px;
    margin: 0 -8px;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.12s;
  }
  .row:hover {
    background: var(--raise);
  }
  input[type="checkbox"] {
    width: 15px;
    height: 15px;
    margin: 0;
    accent-color: var(--inv);
    cursor: pointer;
  }
  .when {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dur,
  .size,
  .fmt {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--dim);
    white-space: nowrap;
  }
  .dur,
  .size {
    text-align: right;
  }
  .fmt {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tag {
    padding: 2px 6px;
    border: 1px solid var(--line2);
    border-radius: 999px;
    color: var(--dim);
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .state {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
  }
  .state.done {
    color: var(--good);
    font-weight: 400;
  }
  .empty {
    max-width: 380px;
    margin: 28px auto;
    text-align: center;
    color: var(--dim);
  }
  .empty svg {
    color: var(--faint);
  }
  .empty h3 {
    margin: 10px 0 6px;
    color: var(--text);
    font-size: 15px;
    font-weight: 600;
  }
  .empty p {
    margin: 0;
    line-height: 1.55;
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px 16px;
    padding: 12px 20px 16px;
    border-top: 1px solid var(--line);
  }
  .tools,
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .actions {
    margin-left: auto;
  }
  .outcome {
    color: var(--good);
    font-size: 12px;
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .outcome.bad {
    color: var(--rec);
  }
  button.primary,
  button.ghost,
  button.danger {
    height: 32px;
    padding: 0 12px;
    border: none;
    border-radius: 9px;
    font-weight: 500;
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: background 0.15s, color 0.15s, opacity 0.15s;
  }
  button.small {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  .primary {
    background: var(--inv);
    color: var(--invInk);
  }
  .ghost {
    background: transparent;
    color: var(--dim);
  }
  .ghost:hover:not(:disabled) {
    background: color-mix(in srgb, var(--text) 8%, transparent);
    color: var(--text);
  }
  .danger {
    background: transparent;
    color: var(--rec);
  }
  .danger:hover,
  .danger.armed {
    background: color-mix(in srgb, var(--rec) 14%, transparent);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .link {
    border: none;
    background: none;
    padding: 4px 2px;
    color: var(--dim);
    font-size: 12px;
    text-decoration: underline;
    text-decoration-color: var(--line2);
    text-underline-offset: 3px;
  }
  .link:hover {
    color: var(--text);
  }
  .progress {
    flex: 1 1 auto;
    min-width: 0;
  }
  .bar {
    height: 6px;
    background: var(--track);
    border-radius: 999px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--inv);
    border-radius: 999px;
    transition: width 0.15s linear;
  }
  .progress-text {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-top: 6px;
    color: var(--dim);
    font-size: 12px;
  }
  .progress-text span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pct {
    font-family: var(--mono);
  }
  /* Narrow windows: drop the format and tag columns. */
  @media (max-width: 640px) {
    .row {
      grid-template-columns: 18px minmax(0, 1fr) 52px 60px 76px;
    }
    .fmt,
    .tags {
      display: none;
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .overlay,
    .sheet {
      animation: none;
    }
  }
</style>
