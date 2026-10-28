<script lang="ts" generics="T extends string">
  import { tick } from "svelte";

  type Props = { options: { value: T; label: string }[]; id?: string; placeholder?: string } & (
    | { multiple?: false; value: T }
    | { multiple: true; value: T[] }
  );

  let { value = $bindable(), options, id, placeholder = "", multiple = false }: Props = $props();

  const GAP = 6;
  const MAX_HEIGHT = 280;

  let button: HTMLButtonElement;
  let list = $state<HTMLDivElement>();
  let open = $state(false);
  let active = $state(0);
  let placement = $state({ left: 0, top: 0, width: 0, maxHeight: MAX_HEIGHT, above: false });

  const chosen = $derived(options.filter((option) => isChosen(option.value)));
  const label = $derived(
    chosen.length === 0
      ? placeholder
      : chosen.length === 1
        ? chosen[0].label
        : `${chosen[0].label} +${chosen.length - 1}`,
  );

  function isChosen(option: T): boolean {
    return Array.isArray(value) ? value.includes(option) : value === option;
  }

  $effect(() => {
    if (!open) {
      return;
    }
    const close = (event: Event) => {
      const target = event.target as Node;
      if (!button.contains(target) && !list?.contains(target)) {
        open = false;
      }
    };
    window.addEventListener("pointerdown", close, true);
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("pointerdown", close, true);
      window.removeEventListener("resize", close);
    };
  });

  async function show() {
    const rect = button.getBoundingClientRect();
    const below = window.innerHeight - rect.bottom - GAP * 2;
    const above = rect.top - GAP * 2;
    const flip = below < 160 && above > below;
    placement = {
      left: rect.left,
      width: rect.width,
      top: flip ? rect.top - GAP : rect.bottom + GAP,
      maxHeight: Math.min(MAX_HEIGHT, flip ? above : below),
      above: flip,
    };
    active = Math.max(
      0,
      options.findIndex((option) => isChosen(option.value)),
    );
    open = true;
    await tick();
    scrollToActive();
  }

  function toggle() {
    if (open) {
      open = false;
    } else {
      void show();
    }
  }

  function choose(index: number) {
    const option = options[index].value;
    if (Array.isArray(value)) {
      value = isChosen(option) ? value.filter((item) => item !== option) : [...value, option];
      return;
    }
    value = option;
    open = false;
    button.focus();
  }

  // A fixed list is placed inside any ancestor with a backdrop filter and clipped by it,
  // so it moves up to the page, or to the dialog the select sits in.
  function lift(node: HTMLElement) {
    (button.closest("dialog") ?? document.body).append(node);
  }

  function scrollToActive() {
    list?.children[active]?.scrollIntoView({ block: "nearest" });
  }

  function onkeydown(event: KeyboardEvent) {
    if (!open) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(event.key)) {
        event.preventDefault();
        void show();
      }
      return;
    }
    const moves: Record<string, number> = {
      ArrowDown: 1,
      ArrowUp: -1,
      PageDown: 8,
      PageUp: -8,
    };
    if (event.key in moves) {
      event.preventDefault();
      active = Math.min(options.length - 1, Math.max(0, active + moves[event.key]));
      scrollToActive();
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      choose(active);
    } else if (event.key === "Escape" || event.key === "Tab") {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
      }
      open = false;
    }
  }
</script>

<button
  bind:this={button}
  {id}
  type="button"
  class="field"
  class:open
  aria-haspopup="listbox"
  aria-expanded={open}
  onclick={toggle}
  {onkeydown}
>
  <span class:placeholder={chosen.length === 0}>{label}</span>
  <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 6l4 4 4-4" /></svg>
</button>

{#if open}
  <div
    class="list"
    bind:this={list}
    {@attach lift}
    role="listbox"
    aria-multiselectable={multiple}
    class:above={placement.above}
    style:left="{placement.left}px"
    style:top="{placement.top}px"
    style:width="{placement.width}px"
    style:max-height="{placement.maxHeight}px"
  >
    {#each options as option, index (option.value)}
      <div
        role="option"
        tabindex="-1"
        aria-selected={isChosen(option.value)}
        class:active={index === active}
        onpointerenter={() => (active = index)}
        onpointerdown={(event) => event.preventDefault()}
        onclick={() => choose(index)}
        onkeydown={() => {}}
      >
        {option.label}
        {#if isChosen(option.value)}
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.5l3 3 6-7" /></svg>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    height: 36px;
    padding: 0 10px 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--field);
    color: var(--text);
    font-size: 14px;
    text-align: left;
    cursor: pointer;
  }

  .field span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .field .placeholder {
    color: var(--text-muted);
  }

  .field.open {
    border-color: var(--accent);
  }

  .field svg {
    transition: transform 150ms var(--ease);
  }

  .field.open svg {
    transform: rotate(180deg);
  }

  .list {
    position: fixed;
    z-index: 100;
    margin: 0;
    padding: 5px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-solid);
    box-shadow: 0 16px 40px rgb(0 0 0 / 0.25);
    animation: drop 140ms var(--ease);
  }

  .list.above {
    transform: translateY(-100%);
    animation: none;
  }

  [role="option"] {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 32px;
    padding: 0 10px;
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: 14px;
    cursor: pointer;
  }

  [role="option"].active {
    background: var(--field);
  }

  [role="option"][aria-selected="true"] {
    color: var(--accent);
    font-weight: 600;
  }

  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
</style>
