<script>
  // The real Mic 3 receiver photo with its touch screen re-enacted on top,
  // following the receiver's own flow into app connection mode (as filmed
  // on a Mic 3, firmware 03.01.00.10):
  //   hold  – swipe down → the control menu opens on the receiver settings →
  //           swipe left past device linking and timecode to "App" → tap it →
  //           confirm that current devices will be disconnected → "ready to
  //           connect" (one 10 s loop, a callout per phase)
  //   tap   – the app's connection request with its 4-digit `pin`; accept
  //   power – the screen glows; wifi – waves to the computer; done – a check
  // `option` is the receiver's name for the App option in this language;
  // `labels` holds the callouts { down, swipe, app, confirm, ready, accept }.
  import rxImg from "../assets/devices/mic-3-rx.png";

  let { mode = "power", option = "", pin = "", labels = {} } = $props();
</script>

<div class="stage" class:wifi={mode === "wifi"} aria-hidden="true">
  <div class="device">
    <img src={rxImg} alt="" draggable="false" />

    {#if mode === "power"}
      <span class="screen glow"></span>
    {/if}

    {#if mode === "hold"}
      <span class="screen frame"></span>
      <span class="screen clip">
        <!-- Control menu: icons in a row, the selected one in the middle,
             framed and underlined, with its name above. -->
        <span class="menu">
          <span class="name">{option}</span>
          <span class="row">
            <span class="slot">
              <svg viewBox="0 0 24 24" width="16" height="16"><rect x="3" y="4" width="13" height="11" rx="2" fill="none" stroke="currentColor" stroke-width="2" /><text x="9.5" y="12.5" font-size="6.5" font-weight="700" text-anchor="middle" fill="currentColor">RX</text><circle cx="18" cy="18" r="3.5" fill="none" stroke="currentColor" stroke-width="2" /></svg>
            </span>
            <span class="slot">
              <svg viewBox="0 0 24 24" width="16" height="16"><path d="M9 7l3-3 8 8-3 3M15 17l-3 3-8-8 3-3" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" /><path d="M9 15l6-6" stroke="currentColor" stroke-width="2" /></svg>
            </span>
            <span class="slot"><span class="tc">TC</span></span>
            <span class="slot"><span class="app">App</span></span>
          </span>
          <span class="uline"></span>
        </span>
        <!-- "Connecting to the app disconnects current devices. Connect?" -->
        <span class="dialog">
          <span class="txt"><i></i><i></i><i class="short"></i></span>
          <span class="btns"><b></b><b class="go"></b></span>
        </span>
        <!-- "Ready to connect" -->
        <span class="ready">
          <span class="spin"></span>
          <span class="btns one"><b></b></span>
        </span>
      </span>
      <span class="finger v"></span>
      <span class="finger h"></span>
      <span class="tap t1"></span>
      <span class="tap t2"></span>
      {#each ["down", "swipe", "app", "confirm", "ready"] as k, i}
        {#if labels[k]}<span class="callout p{i + 1}">{labels[k]}</span>{/if}
      {/each}
    {/if}

    {#if mode === "tap"}
      <span class="screen frame on"></span>
      <span class="screen clip">
        <!-- "App connection request", the code, Cancel / Accept -->
        <span class="request">
          <span class="title"></span>
          <span class="pin">{pin || "····"}</span>
          <span class="btns"><b></b><b class="go"></b></span>
        </span>
      </span>
      <span class="tap t3"></span>
      {#if labels.accept}<span class="callout">{labels.accept}</span>{/if}
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
        <path d="M30 14 Q46 30 30 46" />
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
    --loop: 10s;
    --ink: #8fe3f0;
  }
  .device {
    position: relative;
    width: 236px;
    transition: width 0.35s ease;
  }
  .wifi .device {
    width: 150px;
  }
  img {
    display: block;
    width: 100%;
    height: auto;
    user-select: none;
    filter: drop-shadow(0 18px 28px rgba(0, 0, 0, 0.35));
  }

  /* The touch screen on mic-3-rx.png (798 × 339). */
  .screen {
    position: absolute;
    left: 2.6%;
    top: 11.5%;
    width: 69%;
    height: 66%;
    border-radius: 9% / 20%;
    pointer-events: none;
  }
  .screen.glow {
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--good) 55%, transparent), 0 0 26px color-mix(in srgb, var(--good) 45%, transparent);
    animation: glow 1.8s ease-in-out infinite;
  }
  .screen.frame {
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--good) 60%, transparent);
  }
  .screen.frame.on {
    box-shadow: 0 0 0 3px var(--good);
  }
  .screen.clip {
    overflow: hidden;
  }

  /* Layers drawn over the photo's home screen. */
  .menu,
  .dialog,
  .ready,
  .request {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 5px;
    background: #050607;
    color: var(--ink);
  }
  .menu {
    transform: translateY(-105%);
    animation: menu var(--loop) ease-in-out infinite;
  }
  .name {
    height: 11px;
    max-width: 92%;
    font-size: 9px;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0;
    animation: name var(--loop) linear infinite;
  }
  /* Four 34 px slots; the row slides left one slot per swipe. */
  .row {
    display: flex;
    align-items: center;
    animation: row var(--loop) ease-in-out infinite;
  }
  .slot {
    width: 34px;
    display: grid;
    place-items: center;
  }
  .tc {
    padding: 1px 2px;
    border: 1.5px solid currentColor;
    border-radius: 2px;
    font-size: 8px;
    font-weight: 700;
    line-height: 1;
  }
  .app {
    padding: 2px 3px;
    border: 1.5px solid currentColor;
    border-radius: 3px;
    font-size: 12px;
    font-weight: 700;
    line-height: 1;
    color: #d8fbff;
  }
  .uline {
    width: 22px;
    height: 2px;
    border-radius: 2px;
    background: var(--ink);
  }
  .txt {
    display: grid;
    justify-items: center;
    gap: 3px;
    width: 80%;
  }
  .txt i,
  .title {
    display: block;
    width: 100%;
    height: 4px;
    border-radius: 2px;
    background: color-mix(in srgb, var(--ink) 75%, transparent);
  }
  .txt i.short {
    width: 45%;
  }
  .btns {
    display: flex;
    gap: 4px;
    width: 84%;
  }
  .btns b {
    flex: 1;
    height: 11px;
    border-radius: 3px;
    background: #6f7fd8;
  }
  .btns.one b {
    flex: 0 0 70%;
    margin: 0 auto;
  }
  .dialog {
    opacity: 0;
    animation: dialog var(--loop) linear infinite;
  }
  .ready {
    opacity: 0;
    animation: ready var(--loop) linear infinite;
  }
  .spin {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid var(--ink);
    border-right-color: transparent;
    animation: spin 0.9s linear infinite;
  }
  .request .title {
    width: 72%;
  }
  .pin {
    font-family: var(--mono);
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.35em;
    color: #e8fdff;
  }

  /* Fingertips and taps. */
  .finger {
    position: absolute;
    width: 20px;
    height: 20px;
    margin: -10px 0 0 -10px;
    border-radius: 50%;
    background: color-mix(in srgb, #fff 80%, transparent);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--good) 45%, transparent);
    opacity: 0;
  }
  .finger.v {
    left: 37%;
    top: 16%;
    animation: fv var(--loop) ease-in-out infinite;
  }
  .finger.h {
    left: 52%;
    top: 47%;
    animation: fh var(--loop) ease-in-out infinite;
  }
  .tap {
    position: absolute;
    width: 30px;
    height: 30px;
    margin: -15px 0 0 -15px;
    border-radius: 50%;
    border: 3px solid var(--good);
    opacity: 0;
  }
  .tap.t1 {
    left: 37%;
    top: 47%;
    animation: t1 var(--loop) ease-out infinite;
  }
  /* The right-hand button ("Connect" / "Accept"). */
  .tap.t2,
  .tap.t3 {
    left: 52%;
    top: 62%;
  }
  .tap.t2 {
    animation: t2 var(--loop) ease-out infinite;
  }
  .tap.t3 {
    animation: t3 2.4s ease-out infinite;
  }

  /* Callouts under the receiver, one per phase. */
  .callout {
    position: absolute;
    left: 50%;
    top: calc(78% + 14px);
    transform: translateX(-50%);
    width: max-content;
    max-width: 170px;
    padding: 5px 10px;
    border-radius: 999px;
    background: var(--inv);
    color: var(--invInk);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 600;
    line-height: 1.35;
    text-align: center;
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
  .callout.p1,
  .callout.p2,
  .callout.p3,
  .callout.p4,
  .callout.p5 {
    opacity: 0;
  }
  .p1 {
    animation: p1 var(--loop) linear infinite;
  }
  .p2 {
    animation: p2 var(--loop) linear infinite;
  }
  .p3 {
    animation: p3 var(--loop) linear infinite;
  }
  .p4 {
    animation: p4 var(--loop) linear infinite;
  }
  .p5 {
    animation: p5 var(--loop) linear infinite;
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

  /* Timeline: 0–12 % swipe down · 10–16 % menu opens · 18–46 % three swipes
     left to App · 50–58 % tap App · 60–78 % confirm dialog, tap Connect ·
     78–97 % ready to connect. */
  @keyframes fv {
    0% {
      opacity: 0;
      transform: translateY(0);
    }
    2% {
      opacity: 1;
    }
    11% {
      opacity: 1;
      transform: translateY(46px);
    }
    13%,
    100% {
      opacity: 0;
      transform: translateY(46px);
    }
  }
  @keyframes menu {
    0%,
    10% {
      transform: translateY(-105%);
    }
    16%,
    97% {
      transform: translateY(0);
    }
    100% {
      transform: translateY(-105%);
    }
  }
  @keyframes row {
    0%,
    19% {
      transform: translateX(51px);
    }
    25%,
    29% {
      transform: translateX(17px);
    }
    35%,
    39% {
      transform: translateX(-17px);
    }
    45%,
    100% {
      transform: translateX(-51px);
    }
  }
  @keyframes name {
    0%,
    45% {
      opacity: 0;
    }
    48%,
    100% {
      opacity: 1;
    }
  }
  @keyframes fh {
    0%,
    18% {
      opacity: 0;
      transform: translateX(0);
    }
    19% {
      opacity: 1;
      transform: translateX(0);
    }
    25% {
      opacity: 1;
      transform: translateX(-40px);
    }
    26%,
    28% {
      opacity: 0;
      transform: translateX(0);
    }
    29% {
      opacity: 1;
      transform: translateX(0);
    }
    35% {
      opacity: 1;
      transform: translateX(-40px);
    }
    36%,
    38% {
      opacity: 0;
      transform: translateX(0);
    }
    39% {
      opacity: 1;
      transform: translateX(0);
    }
    45% {
      opacity: 1;
      transform: translateX(-40px);
    }
    46%,
    100% {
      opacity: 0;
      transform: translateX(-40px);
    }
  }
  @keyframes t1 {
    0%,
    50% {
      opacity: 0;
      transform: scale(0.6);
    }
    53% {
      opacity: 1;
      transform: scale(0.8);
    }
    58%,
    100% {
      opacity: 0;
      transform: scale(1.6);
    }
  }
  @keyframes dialog {
    0%,
    59% {
      opacity: 0;
    }
    60%,
    77% {
      opacity: 1;
    }
    78%,
    100% {
      opacity: 0;
    }
  }
  @keyframes t2 {
    0%,
    66% {
      opacity: 0;
      transform: scale(0.6);
    }
    69% {
      opacity: 1;
      transform: scale(0.8);
    }
    75%,
    100% {
      opacity: 0;
      transform: scale(1.6);
    }
  }
  @keyframes ready {
    0%,
    77% {
      opacity: 0;
    }
    78%,
    97% {
      opacity: 1;
    }
    100% {
      opacity: 0;
    }
  }
  @keyframes t3 {
    0%,
    40% {
      opacity: 0;
      transform: scale(0.6);
    }
    50% {
      opacity: 1;
      transform: scale(0.8);
    }
    75%,
    100% {
      opacity: 0;
      transform: scale(1.6);
    }
  }
  @keyframes p1 {
    0%,
    15% {
      opacity: 1;
    }
    16%,
    100% {
      opacity: 0;
    }
  }
  @keyframes p2 {
    0%,
    16% {
      opacity: 0;
    }
    17%,
    47% {
      opacity: 1;
    }
    48%,
    100% {
      opacity: 0;
    }
  }
  @keyframes p3 {
    0%,
    48% {
      opacity: 0;
    }
    49%,
    59% {
      opacity: 1;
    }
    60%,
    100% {
      opacity: 0;
    }
  }
  @keyframes p4 {
    0%,
    60% {
      opacity: 0;
    }
    61%,
    77% {
      opacity: 1;
    }
    78%,
    100% {
      opacity: 0;
    }
  }
  @keyframes p5 {
    0%,
    78% {
      opacity: 0;
    }
    79%,
    97% {
      opacity: 1;
    }
    98%,
    100% {
      opacity: 0;
    }
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes glow {
    50% {
      opacity: 0.35;
    }
  }
  @keyframes arc {
    50% {
      opacity: 1;
    }
  }
  /* Without motion: show the menu on App, with its callout. */
  @media (prefers-reduced-motion: reduce) {
    .menu,
    .row,
    .name,
    .dialog,
    .ready,
    .finger,
    .tap,
    .callout,
    .spin,
    .screen.glow,
    .arcs path {
      animation: none;
    }
    .menu {
      transform: none;
    }
    .row {
      transform: translateX(-51px);
    }
    .name,
    .callout.p3 {
      opacity: 1;
    }
  }
</style>
