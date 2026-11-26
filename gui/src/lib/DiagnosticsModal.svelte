<script>
  import { onMount } from "svelte";
  import { t, tError } from "./i18n.js";
  import { diagnostics, openIssues } from "./api.js";
  let { onclose } = $props();

  let report = $state("");
  let loading = $state(true);
  let failure = $state(null);
  let copied = $state(false);
  let copyFailed = $state(false);
  let pre = $state(null);

  async function load() {
    loading = true;
    failure = null;
    try {
      report = await diagnostics();
    } catch (e) {
      failure = String(e);
    } finally {
      loading = false;
    }
  }
  onMount(load);

  // The webview's clipboard API, then the old selection-based copy.
  async function writeClipboard(text) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      const area = document.createElement("textarea");
      area.value = text;
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.appendChild(area);
      area.select();
      const ok = document.execCommand("copy");
      area.remove();
      return ok;
    }
  }

  async function copy() {
    copyFailed = false;
    if (await writeClipboard(report)) {
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } else {
      copyFailed = true;
      // Leave the text selected so it can be copied by hand.
      const range = document.createRange();
      range.selectNodeContents(pre);
      const sel = window.getSelection();
      sel.removeAllRanges();
      sel.addRange(range);
    }
  }

  function onkeydown(e) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window {onkeydown} />

<div class="overlay" onclick={onclose} role="presentation">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" aria-label={$t("Diagnostic report")}>
    <header>
      <h2>{$t("Diagnostic report")}</h2>
      <button class="x" onclick={onclose} aria-label={$t("Close")}>×</button>
    </header>

    <p class="lead">
      {$t("If your microphone isn't detected or something doesn't work, send this report along with a short description of the problem. It lists the DJI devices connected over USB and what they answered. Serial numbers are cut to their last four characters.")}
    </p>

    <div class="report">
      <div class="report-head">
        <button class="ghost" onclick={load} disabled={loading}>{$t("Refresh")}</button>
        <div class="actions">
          <button class="ghost" onclick={() => openIssues().catch(() => {})}>{$t("Report a problem on GitHub")}</button>
          <button class="copy" onclick={copy} disabled={loading || !report}>{copied ? $t("Copied") : $t("Copy report")}</button>
        </div>
      </div>
      {#if loading}
        <p class="state">{$t("Collecting…")}</p>
      {:else if failure}
        <p class="state error">{$t("Couldn't create the report: {detail}", { detail: $tError(failure) })}</p>
      {:else}
        <pre bind:this={pre}>{report}</pre>
      {/if}
    </div>

    {#if copyFailed}
      <p class="foot">{$t("Couldn't copy automatically. The text is selected: copy it by hand.")}</p>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(8, 10, 14, 0.5);
    display: grid;
    place-items: center;
    padding: 24px;
    z-index: 50;
  }
  .modal {
    width: min(680px, 100%);
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    padding: 20px 22px 22px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: 17px;
  }
  .x {
    border: none;
    background: transparent;
    color: var(--text-dim);
    font-size: 22px;
    line-height: 1;
  }
  .lead {
    color: var(--text-dim);
    line-height: 1.5;
  }
  .report {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg);
  }
  .report-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }
  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .copy,
  .ghost {
    flex: 0 0 auto;
    border-radius: 6px;
    padding: 4px 10px;
    font-size: 12px;
  }
  .copy {
    border: 1px solid var(--border-strong);
    background: var(--bg-panel);
    color: var(--text);
  }
  .ghost {
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-dim);
  }
  .ghost:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--border);
  }
  button:disabled {
    opacity: 0.5;
  }
  pre {
    margin: 0;
    padding: 12px;
    overflow: auto;
    min-height: 120px;
    max-height: 48vh;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11.5px;
    line-height: 1.5;
    white-space: pre;
    user-select: text;
    -webkit-user-select: text;
  }
  .state {
    margin: 0;
    padding: 18px 12px;
    color: var(--text-dim);
    font-size: 13px;
  }
  .state.error {
    color: var(--danger);
  }
  .foot {
    margin: 10px 0 0;
    color: var(--text-faint);
    font-size: 13px;
  }
</style>
