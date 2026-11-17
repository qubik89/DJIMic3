<script>
  import { t } from "./i18n.js";
  import DevicePicto from "./DevicePicto.svelte";
  import BatteryIcon from "./BatteryIcon.svelte";

  // Device name + chevron in the header; opens a popover listing every
  // connected receiver. Replaces the old sidebar.
  let { device, devices = [], connected = false, serial = "", onselect } = $props();
  let open = $state(false);
  let root = $state(null);

  function pick(id) {
    open = false;
    if (id !== device.id) onselect?.(id);
  }
  function onwindowclick(e) {
    if (open && root && !root.contains(e.target)) open = false;
  }
  function onwindowkey(e) {
    if (open && e.key === "Escape") open = false;
  }
</script>

<svelte:window onclick={onwindowclick} onkeydown={onwindowkey} />

<div class="switcher" bind:this={root}>
  <button class="name-btn" aria-haspopup="listbox" aria-expanded={open} onclick={() => (open = !open)}>
    <span class="name">{device.model_name}</span>
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path d="M6 9l6 6 6-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </button>
  <div class="status">
    <span class="state" class:on={connected}>
      <span class="dot"></span>{connected ? $t("Connected") : $t("No signal")}
    </span>
    <span class="divider"></span>
    <span class="serial">{serial}</span>
  </div>

  {#if open}
    <div class="pop" role="listbox">
      {#each devices as d (d.id)}
        <button class="item" class:selected={d.id === device.id} role="option" aria-selected={d.id === device.id} onclick={() => pick(d.id)}>
          <span class="img"><DevicePicto pictogram={`${d.pictogram_key}-rx`} width={48} height={24} /></span>
          <span class="text">
            <span class="iname">{d.model_name}</span>
            <span class="iserial">{d.rx_serial ?? d.id}</span>
            {#if d.tx?.some((x) => x)}
              <span class="batts">
                {#each d.tx as tx, i}
                  {#if tx}
                    <span class="batt">TX{i + 1}<BatteryIcon value={tx.battery} charging={!!tx.charging} size={14} percent={false} tip={false} /></span>
                  {/if}
                {/each}
              </span>
            {/if}
          </span>
          <span class="idot" class:on={d.connected}></span>
        </button>
      {/each}
      <div class="hint">{$t("Connect another receiver over USB to add it here.")}</div>
    </div>
  {/if}
</div>

<style>
  .switcher {
    position: relative;
    min-width: 0;
  }
  .name-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0;
    border: none;
    background: none;
    color: var(--text);
  }
  .name {
    font-size: 24px;
    font-weight: 600;
    letter-spacing: -0.02em;
    line-height: 1.1;
    white-space: nowrap;
  }
  .name-btn svg {
    color: var(--dim);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 5px;
    font-size: 12px;
    color: var(--dim);
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 500;
  }
  .state .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--faint);
  }
  .state.on {
    color: var(--good);
  }
  .state.on .dot {
    background: var(--good);
  }
  .divider {
    width: 1px;
    height: 10px;
    background: var(--line2);
  }
  .serial {
    font-family: var(--mono);
    font-size: 11.5px;
  }
  .pop {
    position: absolute;
    top: 62px;
    left: 0;
    z-index: 20;
    width: 300px;
    padding: 6px;
    background: var(--panel);
    border: 1px solid var(--line2);
    border-radius: 12px;
    box-shadow: 0 18px 40px rgba(0, 0, 0, 0.35);
  }
  .item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px;
    border: none;
    border-radius: 8px;
    background: transparent;
    text-align: left;
    color: var(--text);
  }
  .item:hover,
  .item.selected {
    background: var(--raise);
  }
  .img {
    width: 48px;
    height: 24px;
    display: grid;
    place-items: center;
    flex: 0 0 auto;
  }
  .text {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .iname {
    font-size: 13px;
    font-weight: 500;
  }
  .iserial {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--dim);
  }
  .batts {
    display: flex;
    gap: 10px;
    margin-top: 3px;
    font-family: var(--mono);
    font-size: 10px;
    color: var(--dim);
  }
  .batt {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .idot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--faint);
    flex: 0 0 auto;
  }
  .idot.on {
    background: var(--good);
  }
  .hint {
    padding: 10px 10px 6px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--dim);
  }
</style>
