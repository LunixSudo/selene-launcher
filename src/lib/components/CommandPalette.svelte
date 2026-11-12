<script lang="ts">
  import { coverOf, iconOf } from "#lib/appearance.ts";
  import { t } from "#lib/i18n/index.svelte.ts";
  import { library } from "#lib/state/instances.svelte.ts";
  import { ui } from "#lib/state/ui.svelte.ts";
  import { goto } from "$app/navigation";

  interface Entry {
    id: string;
    label: string;
    hint: string;
    href: string;
    icon: string;
    cover?: string;
  }

  const PAGES = [
    {
      href: "/",
      label: "nav.instances",
      icon: "M4.5 4.5h6v6h-6zM13.5 4.5h6v6h-6zM4.5 13.5h6v6h-6zM13.5 13.5h6v6h-6z",
    },
    {
      href: "/modpacks",
      label: "nav.modpacks",
      icon: "M12 3.5 19.5 7.5v9L12 20.5 4.5 16.5v-9zM4.5 7.5 12 11.5l7.5-4M12 11.5v9",
    },
    {
      href: "/skins",
      label: "nav.skins",
      icon: "M9 4 4.5 6.5 6 10.5l2-.6V20h8V9.9l2 .6 1.5-4L15 4c-.4 1.5-1.6 2.5-3 2.5S9.4 5.5 9 4z",
    },
    {
      href: "/settings",
      label: "settings.title",
      icon: "M4 7h9M17 7h3M4 17h3M11 17h9M15 5a2 2 0 1 1 0 4 2 2 0 0 1 0-4zM9 15a2 2 0 1 1 0 4 2 2 0 0 1 0-4z",
    },
  ] as const;

  let query = $state("");
  let selected = $state(0);
  let input = $state<HTMLInputElement>();

  const entries = $derived.by((): Entry[] => {
    const wanted = query.trim().toLocaleLowerCase();
    const matches = (text: string) => text.toLocaleLowerCase().includes(wanted);
    const pages = PAGES.map((entry) => ({
      id: entry.href,
      label: t(entry.label),
      hint: t("palette.page"),
      href: entry.href,
      icon: entry.icon,
    }));
    const instances = library.instances.map((instance) => ({
      id: instance.id,
      label: instance.name,
      hint: [instance.loader?.loader, instance.version].filter(Boolean).join(" "),
      href: `/instance/${instance.id}`,
      icon: iconOf(instance.icon),
      cover: coverOf(instance.id, instance.cover),
    }));
    return [...pages, ...instances].filter((entry) => matches(entry.label)).slice(0, 8);
  });

  $effect(() => {
    if (ui.paletteOpen) {
      query = "";
      selected = 0;
      queueMicrotask(() => input?.focus());
    }
  });

  $effect(() => {
    if (selected >= entries.length) {
      selected = Math.max(0, entries.length - 1);
    }
  });

  function choose(entry: Entry | undefined) {
    if (entry) {
      ui.paletteOpen = false;
      void goto(entry.href);
    }
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      ui.paletteOpen = false;
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      selected = (selected + 1) % Math.max(1, entries.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      selected = (selected - 1 + entries.length) % Math.max(1, entries.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      choose(entries[selected]);
    }
  }
</script>

{#if ui.paletteOpen}
  <div class="scrim" aria-hidden="true" onclick={() => (ui.paletteOpen = false)}></div>
  <div class="palette" role="dialog" aria-modal="true" aria-label={t("palette.title")}>
    <label class="field">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M10.5 4a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM15.5 15.5 20 20" />
      </svg>
      <input
        bind:this={input}
        bind:value={query}
        {onkeydown}
        type="search"
        placeholder={t("palette.placeholder")}
        aria-label={t("palette.placeholder")}
      >
      <kbd>Esc</kbd>
    </label>
    <ul aria-label={t("palette.title")}>
      {#each entries as entry, index (entry.id)}
        <li>
          <button
            type="button"
            tabindex="-1"
            class:selected={index === selected}
            onpointermove={() => (selected = index)}
            onclick={() => choose(entry)}
          >
            <span class="icon" class:cover={entry.cover} style:background={entry.cover}>
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d={entry.icon} /></svg>
            </span>
            <span class="label">{entry.label}</span>
            <span class="hint">{entry.hint}</span>
          </button>
        </li>
      {:else}
        <li class="empty">{t("palette.empty")}</li>
      {/each}
    </ul>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: var(--scrim);
  }

  .palette {
    position: fixed;
    top: calc(var(--titlebar-height) + 40px);
    left: 50%;
    z-index: 41;
    width: min(560px, calc(100vw - 32px));
    padding: 8px;
    border-radius: var(--radius-lg);
    background: var(--surface-strong);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      0 0 0 1px var(--border),
      0 30px 60px -20px rgb(0 0 0 / 0.6);
    backdrop-filter: var(--glass);
    transform: translateX(-50%);
    animation: drop 320ms var(--spring);
  }

  @keyframes drop {
    from {
      opacity: 0;
      transform: translate(-50%, -8px) scale(0.98);
    }
  }

  .field {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 48px;
    padding: 0 10px 0 16px;
    border-radius: 999px;
    background: var(--field);
    color: var(--text-muted);
  }

  input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 16px;
    outline: none;
  }

  input::-webkit-search-cancel-button {
    display: none;
  }

  kbd {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--field);
    box-shadow: 0 0 0 1px var(--border);
    color: var(--text-2);
    font-family: inherit;
    font-size: 11.5px;
    font-weight: 750;
  }

  ul {
    margin: 6px 0 0;
    padding: 0;
    list-style: none;
  }

  li button {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    height: 48px;
    padding: 0 12px;
    border: none;
    border-radius: 16px;
    background: transparent;
    text-align: left;
  }

  li button.selected {
    background: var(--field);
  }

  .icon {
    display: grid;
    flex: none;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 10px;
    background: var(--field);
    color: var(--text-2);
  }

  .icon.cover {
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.18);
    color: #ffffff;
  }

  svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 15px;
    font-weight: 750;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hint {
    color: var(--text-muted);
    font-size: 12.5px;
    font-weight: 700;
    text-transform: capitalize;
  }

  .empty {
    padding: 16px;
    color: var(--text-muted);
    text-align: center;
  }
</style>
