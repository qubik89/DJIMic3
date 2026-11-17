<script>
  // Non-modal card in the bottom-right corner asking for a coffee on Ko-fi.
  // When it appears is decided in App.svelte (see support.js).
  import { t } from "./i18n.js";

  let { onopen, onlater, onnever } = $props();
</script>

<aside class="support" aria-label={$t("Buy me a coffee")}>
  <div class="body">
    <svg class="cup" width="20" height="20" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path d="M4 9h12v5a5 5 0 0 1-5 5H9a5 5 0 0 1-5-5V9z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
      <path d="M16 10.5h1.5a2.5 2.5 0 0 1 0 5H16" stroke="currentColor" stroke-width="1.6" />
      <path d="M8 3.5c-.6.8-.6 1.7 0 2.5M12 3.5c-.6.8-.6 1.7 0 2.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    </svg>
    <p>{$t("This app is free and will stay free. If it saves you a take, you can buy me a coffee.")}</p>
  </div>
  <div class="actions">
    <button class="primary" onclick={onopen}>{$t("Buy me a coffee")}</button>
    <button class="ghost" onclick={onlater}>{$t("Later")}</button>
    <button class="ghost" onclick={onnever}>{$t("Don't show again")}</button>
  </div>
</aside>

<style>
  .support {
    position: absolute;
    right: 20px;
    bottom: 20px;
    /* Below the toast, which is short-lived. */
    z-index: 25;
    width: 340px;
    max-width: calc(100% - 40px);
    padding: 16px;
    background: var(--panel);
    border: 1px solid var(--line2);
    border-radius: 14px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.28);
    animation: rise 0.25s ease-out;
  }
  .body {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .cup {
    flex: 0 0 auto;
    margin-top: 1px;
    color: var(--dim);
  }
  p {
    margin: 0;
    line-height: 1.5;
    color: var(--text);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    margin-top: 14px;
    padding-left: 32px;
  }
  .actions button {
    height: 32px;
    padding: 0 12px;
    border: none;
    border-radius: 9px;
    font-weight: 500;
    white-space: nowrap;
  }
  .primary {
    margin-right: 4px;
    background: var(--inv);
    color: var(--invInk);
  }
  .ghost {
    background: transparent;
    color: var(--dim);
    transition: background 0.15s, color 0.15s;
  }
  .ghost:hover {
    background: var(--raise);
    color: var(--text);
  }
  /* Phones: full width along the bottom. */
  @media (max-width: 560px) {
    .support {
      left: 12px;
      right: 12px;
      bottom: calc(12px + env(safe-area-inset-bottom, 0px));
      width: auto;
      max-width: none;
    }
    .actions {
      padding-left: 0;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .support {
      animation: none;
    }
  }
</style>
