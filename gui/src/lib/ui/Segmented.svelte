<script>
  // Equal-width segmented picker. `options` = [{ value, label }].
  let { options = [], value = null, disabled = false, mono = false, label = "", width = null, onchange } = $props();
</script>

<div class="seg" class:mono role="group" aria-label={label} style:width>
  {#each options as o (o.value)}
    <button
      class:active={o.value === value}
      {disabled}
      onclick={() => !disabled && o.value !== value && onchange?.(o.value)}
    >
      {o.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    display: flex;
    height: 32px;
    padding: 3px;
    gap: 2px;
    background: var(--raise);
    border-radius: 10px;
  }
  button {
    flex: 1 1 0;
    min-width: 0;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--dim);
    font-size: 12.5px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background 0.15s, color 0.15s;
  }
  .mono button {
    font-family: var(--mono);
    font-size: 12px;
  }
  button.active {
    background: var(--inv);
    color: var(--invInk);
  }
  button:disabled {
    cursor: default;
  }
</style>
