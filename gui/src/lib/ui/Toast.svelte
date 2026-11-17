<script>
  // Bottom-centered pill. Set `toast` to { msg, kind, key }; a new key
  // restarts the 2.6 s timer. kind "error" uses the record red.
  let { toast = null } = $props();
  let shown = $state(false);
  let current = $state(null);

  $effect(() => {
    if (!toast) return;
    current = toast;
    shown = true;
    const timer = setTimeout(() => (shown = false), 2600);
    return () => clearTimeout(timer);
  });
</script>

<div class="toast" class:shown class:error={current?.kind === "error"} role="status" aria-live="polite">
  {current?.msg ?? ""}
</div>

<style>
  .toast {
    position: absolute;
    left: 50%;
    bottom: 24px;
    z-index: 30;
    transform: translate(-50%, 8px);
    opacity: 0;
    pointer-events: none;
    padding: 10px 16px;
    border-radius: 999px;
    background: var(--inv);
    color: var(--invInk);
    font-size: 13px;
    font-weight: 500;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.3);
    white-space: nowrap;
    transition: opacity 0.2s, transform 0.2s;
  }
  .toast.shown {
    opacity: 1;
    transform: translate(-50%, 0);
  }
  .toast.error {
    background: var(--rec);
    color: #fff;
  }
</style>
