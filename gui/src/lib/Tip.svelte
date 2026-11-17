<script>
  // Hover/focus/click tooltip. The app's webview doesn't render native
  // `title` tooltips, so this draws its own bubble, positioned with
  // `position: fixed` so card overflow can't clip it.
  let { text = "", kind = "info", children } = $props();

  let anchor = $state(null);
  let open = $state(false);
  let pos = $state({ left: 0, top: 0, below: false });

  const WIDTH = 280;
  const MARGIN = 10;

  function show() {
    if (!anchor || !text) return;
    const r = anchor.getBoundingClientRect();
    const left = Math.min(
      Math.max(MARGIN, r.left + r.width / 2 - WIDTH / 2),
      window.innerWidth - WIDTH - MARGIN,
    );
    // Prefer above; flip below when there's no room.
    const below = r.top < 120;
    pos = { left, top: below ? r.bottom + 8 : r.top - 8, below };
    open = true;
  }
  function hide() {
    open = false;
  }
  function toggle(e) {
    e.stopPropagation();
    open ? hide() : show();
  }
</script>

<svelte:window onscroll={hide} onresize={hide} />

<span
  class="tip {kind}"
  bind:this={anchor}
  role="button"
  tabindex="0"
  aria-label={text}
  onmouseenter={show}
  onmouseleave={hide}
  onfocus={show}
  onblur={hide}
  onclick={toggle}
  onkeydown={(e) => (e.key === "Enter" || e.key === " ") && toggle(e)}
>
  {@render children?.()}
</span>

{#if open}
  <div
    class="bubble {kind}"
    class:below={pos.below}
    style="left:{pos.left}px; top:{pos.top}px; width:{WIDTH}px"
    role="tooltip"
  >
    {text}
  </div>
{/if}

<style>
  .tip {
    display: inline-flex;
    align-items: center;
    cursor: help;
    outline: none;
    line-height: 1;
  }
  .tip:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: 4px;
  }
  .bubble {
    position: fixed;
    z-index: 1000;
    transform: translateY(-100%);
    padding: 9px 11px;
    border-radius: 8px;
    background: var(--bg-elev, #2a2d33);
    color: var(--text, #f2f3f5);
    border: 1px solid var(--border-strong, rgba(255, 255, 255, 0.14));
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
    font-size: 12.5px;
    font-weight: 400;
    line-height: 1.45;
    white-space: normal;
    text-transform: none;
    letter-spacing: normal;
    pointer-events: none;
  }
  .bubble.below {
    transform: none;
  }
  .bubble.warn {
    border-color: var(--warn);
  }
</style>
