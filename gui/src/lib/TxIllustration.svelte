<script>
  // The real Mic 3 transmitter photo with animated hints on top, for the
  // wireless download guide. `mode`:
  //   power – status LED glowing (take it out of the case, switch it on)
  //   hold  – a 2-second ring filling around the link button
  //   tap   – the transmitter buzzes twice, then a short press on the link button
  //   wifi  – Wi-Fi waves from the transmitter to the computer
  //   done  – a check mark
  // `label` is the callout next to the button (hold/tap). The receiver has
  // its own picture (RxIllustration).
  import tx from "../assets/devices/mic-3-tx.png";

  let { mode = "power", label = "" } = $props();
</script>

<div class="stage" class:wifi={mode === "wifi"} aria-hidden="true">
  <div class="device" class:buzz={mode === "tap"}>
    <img src={tx} alt="" draggable="false" />

    {#if mode === "power"}
      <span class="led"></span>
    {/if}

    {#if mode === "hold" || mode === "tap"}
      <span class="button-spot">
        {#if mode === "hold"}
          <svg class="ring" viewBox="0 0 44 44">
            <circle class="track" cx="22" cy="22" r="19" />
            <circle class="fill" cx="22" cy="22" r="19" pathLength="100" />
          </svg>
        {:else}
          <span class="ripple"></span>
        {/if}
        <span class="dot"></span>
      </span>
      {#if label}<span class="callout">{label}</span>{/if}
    {/if}

    {#if mode === "tap"}
      <span class="waves left"><i></i><i></i></span>
      <span class="waves right"><i></i><i></i></span>
    {/if}

    {#if mode === "done"}
      <span class="check">
        <svg viewBox="0 0 24 24" width="22" height="22"><path d="M5 12.5l4.2 4.2L19 7" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" /></svg>
      </span>
    {/if}
  </div>

  {#if mode === "wifi"}
    <div class="air">
      <svg viewBox="0 0 120 60" class="arcs">
        <path d="M18 30 a12 12 0 0 1 0 0" />
        <path class="a1" d="M30 14 Q46 30 30 46" />
        <path class="a2" d="M48 6 Q72 30 48 54" />
        <path class="a3" d="M66 0 Q98 30 66 60" />
      </svg>
    </div>
    <div class="laptop">
      <svg viewBox="0 0 96 64" width="104" height="70">
        <rect x="14" y="4" width="68" height="44" rx="5" fill="var(--raise)" stroke="currentColor" stroke-width="2" />
        <rect x="20" y="10" width="56" height="32" rx="2" fill="var(--panel)" />
        <path d="M44 22l4 6 4-6M48 17v11" stroke="var(--good)" stroke-width="2.4" fill="none" stroke-linecap="round" stroke-linejoin="round" />
        <path d="M4 52h88l-6 8H10z" fill="var(--raise)" stroke="currentColor" stroke-width="2" stroke-linejoin="round" />
      </svg>
    </div>
  {/if}
</div>

<style>
  .stage {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 230px;
    color: var(--dim);
  }
  .device {
    position: relative;
    width: 190px;
    transition: width 0.35s ease;
  }
  .wifi .device {
    width: 120px;
  }


























  img {
    display: block;
    width: 100%;
    height: auto;
    user-select: none;
    filter: drop-shadow(0 18px 28px rgba(0, 0, 0, 0.35));
  }

  /* Positions measured on mic-3-tx.png (599 × 614). */
  .led {
    position: absolute;
    left: 92.8%;
    top: 16%;
    width: 22px;
    height: 22px;
    margin: -11px 0 0 -11px;
    border-radius: 50%;
    background: radial-gradient(circle, rgba(80, 255, 140, 0.95) 0 22%, rgba(80, 255, 140, 0.35) 45%, transparent 70%);
    animation: glow 1.6s ease-in-out infinite;
  }
  .button-spot {
    position: absolute;
    left: 96%;
    top: 68.4%;
    width: 44px;
    height: 44px;
    margin: -22px 0 0 -22px;
  }
  /* Translucent, so the button's own chain icon stays visible. */
  .dot {
    position: absolute;
    inset: 12px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--good) 22%, transparent);
  }
  .ring {
    position: absolute;
    inset: 0;
    transform: rotate(-90deg);
  }
  .ring circle {
    fill: none;
    stroke-width: 4;
  }
  .ring .track {
    stroke: color-mix(in srgb, var(--inv) 22%, transparent);
  }
  .ring .fill {
    stroke: var(--good);
    stroke-linecap: round;
    stroke-dasharray: 100;
    stroke-dashoffset: 100;
    animation: hold 3s linear infinite;
  }
  .ripple {
    position: absolute;
    inset: 8px;
    border-radius: 50%;
    border: 3px solid var(--good);
    opacity: 0;
    animation: tap 2.6s ease-out infinite;
  }
  /* Under the button, so it never runs into the text next to the picture. */
  .callout {
    position: absolute;
    left: 96%;
    top: calc(68.4% + 30px);
    transform: translateX(-50%);
    padding: 5px 10px;
    border-radius: 999px;
    background: var(--inv);
    color: var(--invInk);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
  }
  .callout::before {
    content: "";
    position: absolute;
    left: 50%;
    bottom: 100%;
    width: 2px;
    height: 8px;
    background: var(--inv);
  }

  /* Two buzzes, then the press. */
  .buzz {
    animation: buzz 2.6s linear infinite;
  }
  .waves {
    position: absolute;
    top: 30%;
    display: flex;
    gap: 4px;
    opacity: 0;
    animation: waves 2.6s linear infinite;
  }
  .waves.left {
    right: 100%;
    margin-right: 6px;
    flex-direction: row-reverse;
  }
  .waves.right {
    left: 100%;
    margin-left: 12px;
  }
  .waves i {
    display: block;
    width: 8px;
    height: 34px;
    border: 2.5px solid var(--warn);
    border-left: none;
    border-radius: 0 40px 40px 0;
  }
  .waves.left i {
    border: 2.5px solid var(--warn);
    border-right: none;
    border-radius: 40px 0 0 40px;
  }
  .waves i + i {
    height: 48px;
    margin-top: -7px;
  }

  .check {
    position: absolute;
    right: -6px;
    top: -6px;
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--good);
    color: #fff;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.3);
    animation: pop 0.35s ease-out;
  }

  .air {
    width: 120px;
  }
  .arcs path {
    fill: none;
    stroke: var(--good);
    stroke-width: 3.5;
    stroke-linecap: round;
    opacity: 0.15;
    animation: arc 1.5s ease-in-out infinite;
  }
  .arcs .a2 {
    animation-delay: 0.2s;
  }
  .arcs .a3 {
    animation-delay: 0.4s;
  }
  .laptop {
    color: var(--dim);
  }












  @keyframes glow {
    50% {
      transform: scale(1.5);
      opacity: 0.55;
    }
  }
  @keyframes hold {
    0% {
      stroke-dashoffset: 100;
    }
    66% {
      stroke-dashoffset: 0;
    }
    100% {
      stroke-dashoffset: 0;
    }
  }
  @keyframes buzz {
    0%,
    4%,
    8%,
    12%,
    16% {
      transform: translateX(0) rotate(0);
    }
    2%,
    10% {
      transform: translateX(-4px) rotate(-1.5deg);
    }
    6%,
    14% {
      transform: translateX(4px) rotate(1.5deg);
    }
    20%,
    24%,
    28%,
    32%,
    36% {
      transform: translateX(0) rotate(0);
    }
    22%,
    30% {
      transform: translateX(-4px) rotate(-1.5deg);
    }
    26%,
    34% {
      transform: translateX(4px) rotate(1.5deg);
    }
    100% {
      transform: none;
    }
  }
  @keyframes waves {
    0%,
    17%,
    20%,
    37% {
      opacity: 1;
    }
    18%,
    19%,
    38%,
    100% {
      opacity: 0;
    }
  }
  @keyframes tap {
    0%,
    50% {
      opacity: 0;
      transform: scale(0.6);
    }
    55% {
      opacity: 1;
      transform: scale(0.8);
    }
    80% {
      opacity: 0;
      transform: scale(1.6);
    }
    100% {
      opacity: 0;
    }
  }
  @keyframes arc {
    50% {
      opacity: 1;
    }
  }
  @keyframes pop {
    from {
      transform: scale(0.4);
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .led,
    .ring .fill,
    .ripple,
    .buzz,
    .waves,
    .arcs path {
      animation: none;
    }
    .ring .fill {
      stroke-dashoffset: 0;
    }
    .ripple,
    .waves,
    .arcs path {
      opacity: 1;
    }
  }
</style>
