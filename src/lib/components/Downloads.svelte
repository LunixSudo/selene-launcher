<script lang="ts">
  import { coverOf } from "#lib/appearance.ts";
  import MoonPhase from "#lib/components/MoonPhase.svelte";
  import { t } from "#lib/i18n/index.svelte.ts";
  import { activity, cancelStart, library } from "#lib/state/instances.svelte.ts";
  import { tooltip } from "#lib/tooltip.ts";

  let open = $state(false);

  const items = $derived(
    library.instances.flatMap((instance) => {
      const busy = activity(instance.id);
      return busy ? [{ instance, busy }] : [];
    }),
  );
  const percent = $derived(
    items.length > 0
      ? Math.round(items.reduce((sum, { busy }) => sum + busy.percent, 0) / items.length)
      : 0,
  );

  /** A start can be called off while it downloads; add-on tasks finish on their own. */
  function cancellable(id: string): boolean {
    return library.launches[id]?.status === "preparing" && !library.tasks[id];
  }

  function onkeydown(event: KeyboardEvent) {
    if (open && event.key === "Escape") {
      open = false;
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="downloads">
  <button
    type="button"
    class="toggle"
    class:active={items.length > 0}
    aria-expanded={open}
    aria-label={t("downloads.title")}
    {@attach tooltip(t("downloads.title"))}
    onclick={() => (open = !open)}
  >
    {#if items.length > 0}
      <MoonPhase {percent} />
    {:else}
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 4.5v10M8 10.5l4 4 4-4M5 19.5h14" />
      </svg>
    {/if}
  </button>

  {#if open}
    <div class="outside" aria-hidden="true" onclick={() => (open = false)}></div>
    <div class="pop" role="dialog" aria-label={t("downloads.title")}>
      <h2>{t("downloads.title")}</h2>
      {#each items as { instance, busy } (instance.id)}
        <div class="item">
          <span class="thumb" style:background={coverOf(instance.id, instance.cover)}></span>
          <div class="info">
            <a href="/instance/{instance.id}" onclick={() => (open = false)}>{instance.name}</a>
            <span class="label">
              {busy.background ? t("downloads.ahead", { percent: busy.percent }) : busy.label}
            </span>
            <span class="bar"><span style:width="{busy.percent}%"></span></span>
          </div>
          {#if cancellable(instance.id)}
            <button
              type="button"
              class="icon-button"
              aria-label={t("downloads.cancel")}
              {@attach tooltip(t("downloads.cancel"))}
              onclick={() => cancelStart(instance.id)}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7l10 10M17 7 7 17" /></svg>
            </button>
          {/if}
        </div>
      {:else}
        <p class="empty">{t("downloads.empty")}</p>
      {/each}
      <p class="note">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 3.5l7 3v5c0 4.5-3 7.8-7 9-4-1.2-7-4.5-7-9v-5zM8.8 12l2.2 2.2 4.2-4.4" />
        </svg>
        {t("downloads.note")}
      </p>
    </div>
  {/if}
</div>

<style>
  .downloads {
    position: relative;
    margin-right: 4px;
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

  .toggle {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-2);
    transition:
      background 150ms var(--ease),
      color 150ms var(--ease);
  }

  .toggle:hover,
  .toggle[aria-expanded="true"] {
    background: var(--surface-hover);
    color: var(--text);
  }

  .toggle.active {
    color: var(--gold);
    filter: drop-shadow(0 0 6px color-mix(in srgb, var(--gold) 55%, transparent));
  }

  .outside {
    position: fixed;
    inset: 0;
    z-index: 30;
  }

  .pop {
    position: absolute;
    top: calc(100% + 8px);
    right: -60px;
    z-index: 31;
    display: grid;
    gap: 4px;
    width: 380px;
    padding: 16px 14px 14px;
    border-radius: var(--radius-lg);
    background: var(--surface-strong);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      0 0 0 1px var(--border),
      0 30px 60px -20px rgb(0 0 0 / 0.6);
    backdrop-filter: var(--glass);
    animation: drop 320ms var(--spring);
  }

  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.98);
    }
  }

  h2 {
    margin: 0 0 6px 4px;
    font-size: 16px;
    font-weight: 800;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 4px;
  }

  .thumb {
    flex: none;
    width: 42px;
    height: 42px;
    border-radius: 12px;
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.18);
  }

  .info {
    display: grid;
    flex: 1;
    gap: 3px;
    min-width: 0;
  }

  .info a {
    overflow: hidden;
    color: var(--text);
    font-size: 14.5px;
    font-weight: 800;
    text-decoration: none;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .label {
    color: var(--text-muted);
    font-size: 12.5px;
    font-weight: 650;
  }

  .bar {
    height: 4px;
    margin-top: 3px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--field);
  }

  .bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transition: width 300ms var(--ease);
  }

  .icon-button svg {
    width: 16px;
    height: 16px;
    stroke-width: 2;
  }

  .empty {
    margin: 4px 4px 8px;
    color: var(--text-muted);
    font-size: 13.5px;
    font-weight: 650;
  }

  .note {
    display: flex;
    gap: 10px;
    margin: 6px 0 0;
    padding: 12px 4px 0;
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 12.5px;
    font-weight: 600;
    line-height: 1.45;
  }

  .note svg {
    flex: none;
    width: 17px;
    height: 17px;
    color: var(--ok);
  }
</style>
