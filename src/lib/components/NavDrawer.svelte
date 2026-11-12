<script lang="ts">
  import { coverOf, iconOf } from "#lib/appearance.ts";
  import ReportDialog from "#lib/components/ReportDialog.svelte";
  import { t } from "#lib/i18n/index.svelte.ts";
  import { library } from "#lib/state/instances.svelte.ts";
  import { settings } from "#lib/state/settings.svelte.ts";
  import { ui } from "#lib/state/ui.svelte.ts";
  import { tooltip } from "#lib/tooltip.ts";
  import { page } from "$app/state";

  const SECTIONS = [
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
  ] as const;

  let reporting = $state(false);

  const open = $derived(settings.menuPinned || ui.menuOpen);
  const pinned = $derived(
    settings.pinned
      .map((id) => library.instances.find((instance) => instance.id === id))
      .filter((instance) => instance !== undefined),
  );

  function isCurrent(href: string): boolean {
    return href === "/" ? page.url.pathname === "/" : page.url.pathname.startsWith(href);
  }

  function leave() {
    ui.menuOpen = false;
  }

  function togglePin() {
    settings.menuPinned = !settings.menuPinned;
    ui.menuOpen = false;
  }

  function report() {
    reporting = true;
    leave();
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && ui.menuOpen && !settings.menuPinned) {
      ui.menuOpen = false;
    }
  }
</script>

<svelte:window {onkeydown} />

{#if !settings.menuPinned}
  <div class="scrim" class:shown={ui.menuOpen} aria-hidden="true" onclick={leave}></div>
  <button
    type="button"
    class="handle"
    class:open={ui.menuOpen}
    aria-label={ui.menuOpen ? t("nav.close") : t("nav.open")}
    aria-expanded={ui.menuOpen}
    onclick={() => (ui.menuOpen = !ui.menuOpen)}
  >
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9.5 6l6 6-6 6" /></svg>
  </button>
{/if}

<nav
  class="drawer"
  class:open
  class:pinned={settings.menuPinned}
  aria-label={t("nav.title")}
  inert={!open}
>
  <div class="head">
    <span>{t("nav.title")}</span>
    <button
      type="button"
      class="pin-toggle"
      aria-pressed={settings.menuPinned}
      aria-label={settings.menuPinned ? t("nav.unpinMenu") : t("nav.pinMenu")}
      {@attach tooltip(settings.menuPinned ? t("nav.unpinMenu") : t("nav.pinMenu"))}
      onclick={togglePin}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M9 4h6l-1 5 3.5 3.5h-11L10 9zM12 12.5V20" />
      </svg>
    </button>
  </div>

  {#each SECTIONS as section (section.href)}
    <a
      class="nav"
      class:on={isCurrent(section.href)}
      href={section.href}
      aria-current={isCurrent(section.href) ? "page" : undefined}
      onclick={leave}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d={section.icon} /></svg>
      {t(section.label)}
      {#if section.href === "/" && library.instances.length > 0}
        <span class="hint">{library.instances.length}</span>
      {/if}
    </a>
  {/each}

  {#if pinned.length > 0}
    <div class="group">{t("nav.pinned")}</div>
    {#each pinned as instance (instance.id)}
      <a
        class="pin"
        class:on={page.url.pathname === `/instance/${instance.id}`}
        href={`/instance/${instance.id}`}
        onclick={leave}
      >
        <span class="mini" style:background={coverOf(instance.id, instance.cover)}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconOf(instance.icon)} /></svg>
        </span>
        <span class="name">{instance.name}</span>
      </a>
    {/each}
  {/if}

  <div class="spacer"></div>
  <button type="button" class="nav" onclick={report}>
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path
        d="M5 18.5V6a1.5 1.5 0 0 1 1.5-1.5h11A1.5 1.5 0 0 1 19 6v8a1.5 1.5 0 0 1-1.5 1.5H8.5zM12 8v3.5M12 14h.01"
      />
    </svg>
    {t("nav.report")}
  </button>
</nav>

<ReportDialog bind:open={reporting} />

<style>
  svg {
    flex: none;
    width: 20px;
    height: 20px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .mini svg {
    width: 15px;
    height: 15px;
  }

  .scrim {
    position: fixed;
    inset: var(--titlebar-height) 0 0;
    z-index: 20;
    background: var(--scrim);
    opacity: 0;
    pointer-events: none;
    transition: opacity 300ms var(--ease);
  }

  .scrim.shown {
    opacity: 1;
    pointer-events: auto;
  }

  .handle {
    position: fixed;
    top: calc(50% - 36px + var(--titlebar-height) / 2);
    left: 0;
    z-index: 22;
    display: grid;
    place-items: center;
    width: 22px;
    height: 72px;
    padding: 0;
    border: none;
    border-radius: 0 18px 18px 0;
    background: var(--surface-strong);
    box-shadow:
      inset 0 0 0 1px var(--border),
      6px 0 20px -8px rgb(0 0 0 / 0.4);
    color: var(--text-muted);
    backdrop-filter: var(--glass);
    transition:
      left 500ms var(--spring),
      width 200ms var(--ease),
      color 150ms var(--ease);
  }

  .handle:hover {
    width: 28px;
    color: var(--text);
  }

  .handle.open {
    left: 274px;
  }

  .handle svg {
    width: 16px;
    height: 16px;
    transition: transform 400ms var(--spring);
  }

  .handle.open svg {
    transform: rotate(180deg);
  }

  .drawer {
    position: fixed;
    top: calc(var(--titlebar-height) + 4px);
    bottom: 10px;
    left: 10px;
    z-index: 21;
    display: flex;
    flex-direction: column;
    gap: 3px;
    width: 264px;
    padding: 12px;
    border-radius: var(--radius-lg);
    background: var(--surface-strong);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      0 0 0 1px var(--border),
      24px 0 60px -20px rgb(0 0 0 / 0.55);
    backdrop-filter: var(--glass);
    opacity: 0;
    transform: translateX(-300px);
    transition:
      transform 500ms var(--spring),
      opacity 200ms var(--ease);
  }

  .drawer.open {
    opacity: 1;
    transform: none;
  }

  .drawer.pinned {
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      0 0 0 1px var(--border);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 2px 2px 8px 14px;
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 750;
  }

  .pin-toggle {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-muted);
  }

  .pin-toggle:hover {
    background: var(--field);
    color: var(--text);
  }

  .pin-toggle[aria-pressed="true"] {
    background: var(--accent-soft);
    color: var(--accent-fg);
  }

  .nav,
  .pin {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: var(--text-2);
    font-weight: 700;
    text-align: left;
    text-decoration: none;
    transition:
      background 150ms var(--ease),
      color 150ms var(--ease),
      transform 260ms var(--spring);
  }

  .nav {
    height: 44px;
    padding: 0 14px;
    font-size: 15px;
  }

  .pin {
    height: 40px;
    padding: 0 12px;
    font-size: 14px;
  }

  .nav:hover,
  .pin:hover {
    background: var(--field);
    color: var(--text);
  }

  .nav:active,
  .pin:active {
    transform: scale(0.96);
  }

  .nav.on,
  .pin.on {
    background: var(--accent);
    box-shadow: 0 6px 16px -6px color-mix(in srgb, var(--accent) 65%, transparent);
    color: var(--accent-text);
  }

  .hint {
    margin-left: auto;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 650;
  }

  .nav.on .hint {
    color: rgb(255 255 255 / 0.75);
  }

  .group {
    margin: 18px 14px 6px;
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 750;
  }

  .mini {
    display: grid;
    flex: none;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 9px;
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.18);
    color: #ffffff;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .spacer {
    flex: 1;
  }
</style>
