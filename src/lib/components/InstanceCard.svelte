<script lang="ts">
  import type { Instance } from "#lib/api.ts";
  import { coverOf, iconOf } from "#lib/appearance.ts";
  import MoonPhase from "#lib/components/MoonPhase.svelte";
  import { playTime, relativeTime } from "#lib/format.ts";
  import { t } from "#lib/i18n/index.svelte.ts";
  import { UI_ICONS } from "#lib/icons.ts";
  import { LOADER_NAMES } from "#lib/loaders.ts";
  import { activity, type LaunchState, library, play, stop } from "#lib/state/instances.svelte.ts";

  let {
    instance,
    launch,
    onmenu,
    variant = "card",
  }: {
    instance: Instance;
    launch: LaunchState | undefined;
    onmenu: (x: number, y: number) => void;
    /** A tile in the grid, the wide tile of the last played one, or a line in the list. */
    variant?: "card" | "wide" | "row";
  } = $props();

  const running = $derived(launch?.status === "running");
  const busy = $derived(running ? null : activity(instance.id));
  const href = $derived(`/instance/${instance.id}`);
  const loaderName = $derived(
    instance.loader ? LOADER_NAMES[instance.loader.loader] : t("instances.vanilla"),
  );
  const error = $derived(
    launch?.status === "failed"
      ? t(`errors.${launch.error.kind}`)
      : library.crashes[instance.id]
        ? t("crash.title")
        : null,
  );

  function lastPlayed(): string {
    return instance.lastPlayed
      ? t("instances.lastPlayed", { time: relativeTime(instance.lastPlayed) })
      : t("instances.neverPlayed");
  }

  function summary(): string {
    return instance.playSeconds >= 60
      ? `${lastPlayed()} · ${t("instances.playTime", { time: playTime(instance.playSeconds) })}`
      : lastPlayed();
  }

  function playLabel(): string {
    if (launch?.status === "running") {
      return launch.stopping ? t("instances.stopping") : t("instances.stop");
    }
    return busy?.label ?? t("instances.play");
  }

  function openMenu(event: MouseEvent) {
    event.preventDefault();
    onmenu(event.clientX, event.clientY);
  }

  function openMenuFromButton(event: MouseEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    onmenu(rect.left, rect.bottom + 6);
  }
</script>

{#snippet playButton()}
  <button
    type="button"
    class="play"
    class:running
    style:--progress={busy ? `${busy.percent}%` : "0%"}
    disabled={(busy !== null && !busy.background) ||
      (launch?.status === "running" && launch.stopping)}
    onclick={() => (running ? stop(instance.id) : play(instance.id))}
  >
    {#if busy && !busy.background}
      <MoonPhase percent={busy.percent} />
    {:else}
      <svg class="glyph" viewBox="0 0 24 24" aria-hidden="true">
        <path d={running ? UI_ICONS.stop : UI_ICONS.play} />
      </svg>
    {/if}
    <span>{playLabel()}</span>
  </button>
{/snippet}

{#snippet moreButton()}
  <button
    type="button"
    class="more"
    aria-label={t("instances.actions")}
    onclick={openMenuFromButton}
  >
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.more} /></svg>
  </button>
{/snippet}

{#snippet meta()}
  <span class="chip">{loaderName}</span>
  <span class="version">{instance.version}</span>
{/snippet}

<article class="card {variant}" oncontextmenu={openMenu}>
  {#if variant === "row"}
    <span class="mini" style:background={coverOf(instance.id, instance.cover)}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconOf(instance.icon)} /></svg>
    </span>
    <div class="info">
      <h2><a {href}>{instance.name}</a></h2>
      <p class="meta">
        {@render meta()}
        {#if error}
          <span class="error" role="alert">{error}</span>
        {/if}
      </p>
    </div>
    <p class="summary">{summary()}</p>
    {@render playButton()}
    {@render moreButton()}
  {:else if variant === "wide"}
    <div class="cover" style:background={coverOf(instance.id, instance.cover)}></div>
    {@render moreButton()}
    <div class="body">
      <span class="tile">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconOf(instance.icon)} /></svg>
      </span>
      <div class="titles">
        <p class="summary">{summary()}</p>
        <h2><a {href}>{instance.name}</a></h2>
      </div>
    </div>
    <p class="meta">
      {@render meta()}
      {#if error}
        <span class="error" role="alert">{error}</span>
      {/if}
    </p>
    <div class="actions">
      {@render playButton()}
      <a class="open" {href}>{t("instances.open")}</a>
    </div>
  {:else}
    <div class="cover" style:background={coverOf(instance.id, instance.cover)}>
      <span class="tile">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconOf(instance.icon)} /></svg>
      </span>
    </div>
    {@render moreButton()}
    <div class="body">
      <h2><a {href}>{instance.name}</a></h2>
      <p class="meta">
        {@render meta()}
        <span class="when">
          {instance.lastPlayed ? relativeTime(instance.lastPlayed) : t("instances.fresh")}
        </span>
      </p>
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
    </div>
    {@render playButton()}
  {/if}
</article>

<style>
  svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .tile svg {
    width: 24px;
    height: 24px;
    stroke: #ffffff;
  }

  .mini svg {
    width: 20px;
    height: 20px;
    stroke: #ffffff;
  }

  .more svg {
    stroke-width: 3;
  }

  .play .glyph {
    width: 14px;
    height: 14px;
    fill: currentColor;
  }

  .card {
    position: relative;
    top: 0;
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
    isolation: isolate;
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      0 0 0 1px var(--border);
    backdrop-filter: var(--glass);
    /* Lifted with top, not a transform: some GPUs then blur a square around the whole shadow. */
    transition:
      top 200ms var(--ease),
      box-shadow 200ms var(--ease);
  }

  .card:hover {
    top: -2px;
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      0 0 0 1px var(--border),
      0 12px 24px -12px rgb(0 0 0 / 0.3);
  }

  .cover {
    position: relative;
    flex: none;
    height: 84px;
    image-rendering: pixelated;
  }

  .tile {
    position: absolute;
    bottom: -22px;
    left: 14px;
    z-index: 1;
    display: grid;
    place-items: center;
    width: 46px;
    height: 46px;
    border-radius: 14px;
    background: rgb(24 20 48 / 0.4);
    box-shadow:
      inset 0 0 0 1px rgb(255 255 255 / 0.4),
      0 6px 18px rgb(0 0 0 / 0.22);
    backdrop-filter: blur(12px);
  }

  .more {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 2;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: rgb(0 0 0 / 0.3);
    color: #ffffff;
    opacity: 0;
    transition: opacity 150ms var(--ease);
    backdrop-filter: blur(8px);
  }

  .card:hover .more,
  .more:focus-visible {
    opacity: 1;
  }

  .body {
    flex: 1;
    min-width: 0;
    padding: 32px 16px 0;
  }

  h2 {
    margin: 0;
    overflow: hidden;
    font-size: 17px;
    font-weight: 800;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  h2 a {
    color: inherit;
    text-decoration: none;
  }

  h2 a::after {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    content: "";
  }

  p {
    margin: 0;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    margin-top: 8px;
    font-size: 13px;
  }

  .chip {
    flex: none;
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-fg);
    font-size: 11.5px;
    font-weight: 800;
  }

  .version {
    flex: none;
    font-weight: 750;
  }

  .when {
    margin-left: auto;
    overflow: hidden;
    color: var(--text-muted);
    font-size: 12.5px;
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .error {
    margin-top: 6px;
    overflow: hidden;
    color: var(--danger-text);
    font-size: 12.5px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .play {
    position: relative;
    z-index: 1;
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 40px;
    margin: 14px;
    padding: 0 18px;
    overflow: hidden;
    border: none;
    border-radius: 14px;
    background: var(--accent);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.22),
      0 8px 20px -10px color-mix(in srgb, var(--accent) 75%, transparent);
    color: var(--accent-text);
    font-size: 14.5px;
    font-weight: 800;
    transition:
      filter 150ms var(--ease),
      transform 260ms var(--spring);
  }

  .play::before {
    position: absolute;
    inset: 0;
    width: var(--progress);
    background: rgb(255 255 255 / 0.2);
    transition: width 200ms var(--ease);
    content: "";
  }

  .play > :global(*) {
    position: relative;
  }

  .play span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .play:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .play:active:not(:disabled) {
    transform: scale(0.97);
  }

  .play:disabled {
    cursor: default;
  }

  .play.running {
    background: var(--surface-hover);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text);
  }

  /* The last played instance takes two columns and shows its cover across the whole tile. */
  @container (min-width: 500px) {
    .card.wide {
      grid-column: span 2;
    }
  }

  .wide {
    justify-content: flex-end;
    min-height: 250px;
    color: #ffffff;
  }

  .wide .cover {
    position: absolute;
    inset: 0;
    z-index: -1;
    height: auto;
  }

  .wide .cover::after {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to top,
      rgb(10 8 26 / 0.88),
      rgb(10 8 26 / 0.45) 55%,
      rgb(10 8 26 / 0.05)
    );
    content: "";
  }

  .wide .body {
    display: flex;
    flex: none;
    align-items: center;
    gap: 14px;
    padding: 20px 20px 0;
  }

  .wide .tile {
    position: static;
    flex: none;
    width: 56px;
    height: 56px;
    border-radius: 16px;
  }

  .wide .tile svg {
    width: 28px;
    height: 28px;
  }

  .titles {
    min-width: 0;
  }

  .wide .summary {
    overflow: hidden;
    color: rgb(255 255 255 / 0.78);
    font-size: 13px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .wide h2 {
    font-family: var(--font-display);
    font-size: 26px;
    font-weight: var(--display-weight);
    line-height: 1.2;
  }

  .wide .meta {
    padding: 0 20px;
  }

  .wide .chip {
    background: color-mix(in srgb, var(--accent) 60%, rgb(0 0 0 / 0.3));
    color: #ffffff;
  }

  .wide .error {
    margin: 0;
    color: #ff9a9e;
  }

  .actions {
    position: relative;
    z-index: 1;
    display: flex;
    gap: 10px;
    padding: 16px 20px 20px;
  }

  .wide .play {
    min-width: 0;
    max-width: 240px;
    height: 46px;
    margin: 0;
    padding: 0 28px;
    border-radius: 999px;
    font-size: 15.5px;
    flex: 1 1 auto;
  }

  .wide .play.running {
    background: rgb(255 255 255 / 0.16);
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.25);
    color: #ffffff;
  }

  .open {
    display: grid;
    flex: none;
    place-items: center;
    height: 46px;
    padding: 0 20px;
    border-radius: 999px;
    background: rgb(255 255 255 / 0.14);
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.22);
    color: #ffffff;
    font-weight: 750;
    text-decoration: none;
    backdrop-filter: blur(12px);
    transition:
      background 150ms var(--ease),
      transform 260ms var(--spring);
  }

  .open:hover {
    background: rgb(255 255 255 / 0.22);
  }

  .open:active {
    transform: scale(0.96);
  }

  .row {
    flex-direction: row;
    align-items: center;
    gap: 14px;
    height: 66px;
    padding: 0 10px 0 12px;
    border-radius: var(--radius-md);
  }

  .mini {
    display: grid;
    flex: none;
    place-items: center;
    width: 42px;
    height: 42px;
    border-radius: 12px;
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.2);
  }

  .info {
    flex: 1;
    min-width: 0;
  }

  .row h2 {
    font-size: 15.5px;
  }

  .row .meta {
    margin-top: 4px;
  }

  .row .error {
    margin: 0;
  }

  .row .summary {
    flex: 0 1 260px;
    min-width: 0;
    overflow: hidden;
    color: var(--text-muted);
    font-size: 12.5px;
    font-weight: 650;
    text-align: right;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row .play {
    width: 150px;
    height: 36px;
    margin: 0;
    padding: 0 16px;
    border-radius: 999px;
    font-size: 14px;
  }

  .card.row .more {
    position: relative;
    top: auto;
    right: auto;
    background: transparent;
    color: var(--text-muted);
    opacity: 1;
    backdrop-filter: none;
  }

  .card.row .more:hover {
    background: var(--field);
    color: var(--text);
  }
</style>
