<script>
  import { t } from "./i18n.js";

  // Live input meter for one transmitter. `level` is 0–1 of the meter scale
  // (−48…0 dB), or null when unknown. The bar eases toward it every frame,
  // holds the peak for 800 ms then lets it fall, and latches CLIP for
  // 1.8 s after the level reaches the top of the scale.
  let { level = null, muted = false } = $props();

  const SEGMENTS = 36;
  const DB_MIN = -48;
  const CLIP_AT = 0.97;
  const TICKS = [
    ["−48", 0],
    ["−24", 0.5],
    ["−12", 0.75],
    ["−6", 0.875],
    ["0", 1],
  ];

  let shown = $state(0); // eased fraction drawn
  let peak = $state(0); // held peak fraction
  let clipping = $state(false);
  let readout = $state("—");

  const lit = $derived(muted ? 0 : Math.round(shown * SEGMENTS));
  const peakSeg = $derived(muted ? -1 : Math.min(SEGMENTS - 1, Math.round(peak * SEGMENTS) - 1));
  const colorAt = (p) => (p >= 0.875 ? "var(--rec)" : p >= 0.75 ? "var(--warn)" : "var(--good)");

  $effect(() => {
    let raf;
    let peakUntil = 0;
    let clipUntil = 0;
    let lastText = 0;
    const loop = (now) => {
      const target = muted || level == null ? 0 : Math.max(0, Math.min(1, level));
      // Fast attack, slower release.
      shown += (target - shown) * (target > shown ? 0.45 : 0.12);
      if (shown > peak) {
        peak = shown;
        peakUntil = now + 800;
      } else if (now > peakUntil) {
        peak = Math.max(0, peak - 0.5 / -DB_MIN); // 0.5 dB per frame
      }
      if (!muted && target >= CLIP_AT) clipUntil = now + 1800;
      clipping = !muted && now < clipUntil;
      if (now - lastText > 120) {
        lastText = now;
        const db = DB_MIN + peak * -DB_MIN;
        readout = muted || peak <= 0.02 ? "—" : db > -0.5 ? "0" : "−" + Math.abs(db).toFixed(0);
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  });
</script>

<div class="meter">
  <div class="row">
    <div class="bar">
      <div class="segs">
        {#each { length: SEGMENTS } as _, i}
          {@const on = i < lit || (i === peakSeg && peakSeg > 0)}
          <div class="seg" style:background={on ? colorAt(i / SEGMENTS) : "var(--track)"} style:opacity={on ? 1 : 0.6}></div>
        {/each}
      </div>
      {#if muted}
        <div class="muted">{$t("MUTED")}</div>
      {/if}
    </div>
    <span class="read">{readout}</span>
    <span class="clip" class:on={clipping}>CLIP</span>
  </div>
  <div class="row under">
    <div class="scale">
      {#if clipping}
        <div class="warn">{$t("CLIPPING — LOWER INPUT GAIN")}</div>
      {:else}
        {#each TICKS as [label, f] (label)}
          <span style:left="{f * 100}%" style:transform={f === 0 ? "none" : f === 1 ? "translateX(-100%)" : "translateX(-50%)"}>{label}</span>
        {/each}
      {/if}
    </div>
    <div class="spacer"></div>
  </div>
</div>

<style>
  .meter {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .bar {
    position: relative;
    flex: 1 1 auto;
    height: 22px;
  }
  .segs {
    display: flex;
    gap: 2px;
    height: 100%;
  }
  .seg {
    flex: 1 1 0;
    border-radius: 2px;
  }
  .muted {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--mono);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.14em;
    color: var(--warn);
  }
  .read {
    width: 28px;
    text-align: right;
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--dim);
    font-variant-numeric: tabular-nums;
  }
  .clip {
    width: 38px;
    text-align: center;
    padding: 3px 0;
    border-radius: 5px;
    border: 1px solid var(--line2);
    font-family: var(--mono);
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--faint);
    transition: background 0.12s, color 0.12s;
  }
  .clip.on {
    background: var(--rec);
    border-color: var(--rec);
    color: #fff;
  }
  .under {
    height: 14px;
  }
  .scale {
    position: relative;
    flex: 1 1 auto;
    height: 100%;
  }
  .scale span {
    position: absolute;
    font-family: var(--mono);
    font-size: 9.5px;
    color: var(--faint);
  }
  .warn {
    font-family: var(--mono);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    color: var(--rec);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .spacer {
    width: 76px;
    flex: 0 0 auto;
  }
</style>
