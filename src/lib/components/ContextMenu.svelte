<script lang="ts" module>
  export interface MenuItem {
    label: string;
    icon: string;
    action: () => void;
    danger?: boolean;
    disabled?: boolean;
  }
</script>

<script lang="ts">
  let {
    items,
    position = $bindable(),
  }: { items: MenuItem[]; position: { x: number; y: number } | null } = $props();

  const MARGIN = 8;

  let menu = $state<HTMLDivElement>();
  let placed = $state({ x: 0, y: 0 });

  $effect(() => {
    if (!position || !menu) {
      return;
    }
    // Keep the menu inside the window when opened near an edge.
    const { width, height } = menu.getBoundingClientRect();
    placed = {
      x: Math.min(position.x, window.innerWidth - width - MARGIN),
      y: Math.min(position.y, window.innerHeight - height - MARGIN),
    };
    menu.querySelector("button")?.focus();

    const close = (event: Event) => {
      if (!(event.target instanceof Node && menu?.contains(event.target))) {
        position = null;
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        position = null;
      }
    };
    window.addEventListener("pointerdown", close, true);
    window.addEventListener("wheel", close, true);
    window.addEventListener("blur", close);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      window.removeEventListener("pointerdown", close, true);
      window.removeEventListener("wheel", close, true);
      window.removeEventListener("blur", close);
      window.removeEventListener("keydown", closeOnEscape);
    };
  });

  function run(item: MenuItem) {
    position = null;
    item.action();
  }

  function moveFocus(event: KeyboardEvent) {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") {
      return;
    }
    event.preventDefault();
    const buttons = [...(menu?.querySelectorAll<HTMLButtonElement>("button:enabled") ?? [])];
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const step = event.key === "ArrowDown" ? 1 : -1;
    buttons[(index + step + buttons.length) % buttons.length]?.focus();
  }
</script>

{#if position}
  <div
    class="menu"
    role="menu"
    tabindex="-1"
    bind:this={menu}
    style:left="{placed.x}px"
    style:top="{placed.y}px"
    onkeydown={moveFocus}
  >
    {#each items as item (item.label)}
      <button
        type="button"
        role="menuitem"
        class:danger={item.danger}
        disabled={item.disabled}
        onclick={() => run(item)}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d={item.icon} /></svg>
        {item.label}
      </button>
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 50;
    display: grid;
    min-width: 200px;
    padding: 5px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-solid);
    box-shadow: 0 16px 40px rgb(0 0 0 / 0.22);
    animation: appear 120ms var(--ease);
  }

  button {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: 13px;
    text-align: left;
  }

  button:hover:enabled,
  button:focus-visible {
    outline: none;
    background: var(--field);
  }

  button:disabled {
    opacity: 0.45;
  }

  .danger {
    color: var(--danger);
  }

  svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  @keyframes appear {
    from {
      opacity: 0;
      transform: scale(0.97);
    }
  }
</style>
