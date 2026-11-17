<script>
  // −/+ stepper over a setting's ordered options (e.g. a gain in 1 dB steps).
  // `options` = [{ value, label }]; labels like "-3 dB" are reformatted with a
  // real minus sign.
  let { options = [], value = null, disabled = false, label = "", minWidth = 58, onchange } = $props();

  const index = $derived(options.findIndex((o) => o.value === value));
  const shown = $derived(index >= 0 ? options[index].label.replace(/^-/, "−") : "—");

  function step(d) {
    if (disabled || index < 0) return;
    const next = options[index + d];
    if (next) onchange?.(next.value);
  }
  function onkeydown(e) {
    if (e.key === "ArrowLeft" || e.key === "ArrowDown") (e.preventDefault(), step(-1));
    if (e.key === "ArrowRight" || e.key === "ArrowUp") (e.preventDefault(), step(1));
  }
</script>

<div class="stepper" class:disabled role="spinbutton" tabindex="0" aria-label={label} aria-valuetext={shown} {onkeydown}>
  <button aria-label="−" disabled={disabled || index <= 0} onclick={() => step(-1)}>−</button>
  <span class="val" style:min-width="{minWidth}px">{shown}</span>
  <button aria-label="+" disabled={disabled || index < 0 || index >= options.length - 1} onclick={() => step(1)}>+</button>
</div>

<style>
  .stepper {
    display: flex;
    align-items: center;
    height: 32px;
    padding: 0 3px;
    background: var(--raise);
    border-radius: 10px;
    outline: none;
  }
  .stepper:focus-visible {
    box-shadow: 0 0 0 2px var(--line2);
  }
  button {
    width: 28px;
    height: 26px;
    border: none;
    border-radius: 7px;
    background: transparent;
    font-size: 16px;
    line-height: 1;
    transition: background 0.15s;
  }
  button:hover:not(:disabled) {
    background: var(--track);
  }
  button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .val {
    text-align: center;
    font-family: var(--mono);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
