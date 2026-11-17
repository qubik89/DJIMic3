<script>
  // 40×24 switch: track --track/--inv, knob --knobOff/--knobOn.
  let { checked = false, disabled = false, label = "", onchange } = $props();
</script>

<button
  class="switch"
  class:on={checked}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => !disabled && onchange?.(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .switch {
    position: relative;
    flex: 0 0 auto;
    width: 40px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 999px;
    background: var(--track);
    transition: background 0.18s;
  }
  .switch.on {
    background: var(--inv);
  }
  .switch:disabled {
    cursor: default;
  }
  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--knobOff);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
    transition: left 0.18s;
  }
  .on .knob {
    left: 19px;
    background: var(--knobOn);
  }
</style>
