<script lang="ts">
  import type { Instance } from "$lib/api";
  import { coverOf, iconOf } from "$lib/appearance";
  import { playTime } from "$lib/format";
  import { locale, t } from "$lib/i18n/index.svelte";
  import { UI_ICONS } from "$lib/icons";
  import { LOADER_NAMES } from "$lib/loaders";
  import { type LaunchState, library, play } from "$lib/state/instances.svelte";

  let {
    instance,
    launch,
    onmenu,
  }: {
    instance: Instance;
    launch: LaunchState | undefined;
    onmenu: (x: number, y: number) => void;
  } = $props();

  const busy = $derived(launch?.status === "preparing" || launch?.status === "running");

  function lastPlayed(): string {
    if (!instance.lastPlayed) {
      return t("instances.neverPlayed");
    }
    const minutes = (new Date(instance.lastPlayed).getTime() - Date.now()) / 60_000;
    const format = new Intl.RelativeTimeFormat(locale(), { numeric: "auto" });
    const time =
      Math.abs(minutes) < 60
        ? format.format(Math.round(minutes), "minute")
        : Math.abs(minutes) < 60 * 24
          ? format.format(Math.round(minutes / 60), "hour")
          : format.format(Math.round(minutes / 60 / 24), "day");
    return t("instances.lastPlayed", { time });
  }

  function playLabel(): string {
    if (launch?.status === "running") {
      return t("instances.running");
    }
    if (launch?.status === "preparing") {
      return launch.stage
        ? t(`launch.${launch.stage}`, { percent: launch.percent })
        : t("launch.starting");
    }
    return t("instances.play");
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

<article class="card" oncontextmenu={openMenu}>
  <div class="cover" style:background={coverOf(instance.id, instance.cover)}>
    <span class="tile">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconOf(instance.icon)} /></svg>
    </span>
    <button
      type="button"
      class="more"
      aria-label={t("instances.actions")}
      onclick={openMenuFromButton}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.more} /></svg>
    </button>
  </div>

  <div class="body">
    <h2><a href="/instance/{instance.id}">{instance.name}</a></h2>
    <p class="meta">
      <span class="chip">
        {instance.loader ? LOADER_NAMES[instance.loader.loader] : t("create.vanilla")}
      </span>
      {instance.version}
    </p>
    <p>
      {lastPlayed()}
      {#if instance.playSeconds >= 60}
        · {t("instances.playTime", { time: playTime(instance.playSeconds) })}
      {/if}
    </p>
    {#if launch?.status === "failed"}
      <p class="error" role="alert">{t(`errors.${launch.error.kind}`)}</p>
    {:else if library.crashes[instance.id]}
      <p class="error" role="alert">{t("crash.title")}</p>
    {/if}
  </div>

  <button
    type="button"
    class="play"
    class:running={launch?.status === "running"}
    disabled={busy}
    style:--progress={launch?.status === "preparing" ? `${launch.percent}%` : "0%"}
    onclick={() => play(instance.id)}
  >
    {#if !busy}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.play} /></svg>
    {/if}
    <span>{playLabel()}</span>
  </button>
</article>

<style>
  svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    backdrop-filter: blur(24px) saturate(1.4);
    transition:
      transform 200ms var(--ease),
      box-shadow 200ms var(--ease);
  }

  .card:hover {
    transform: translateY(-2px);
    box-shadow: 0 14px 36px rgb(0 0 0 / 0.16);
  }

  .cover {
    position: relative;
    display: flex;
    align-items: flex-end;
    height: 92px;
    padding: 12px;
  }

  .cover::after {
    position: absolute;
    inset: 0;
    background: radial-gradient(circle at 85% 0%, rgb(255 255 255 / 0.35), transparent 55%);
    content: "";
    pointer-events: none;
  }

  .tile {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    margin-bottom: -30px;
    border: 1px solid rgb(255 255 255 / 0.4);
    border-radius: 14px;
    background: rgb(255 255 255 / 0.22);
    box-shadow: 0 6px 18px rgb(0 0 0 / 0.18);
    backdrop-filter: blur(12px);
  }

  .tile svg {
    width: 24px;
    height: 24px;
    stroke: #ffffff;
  }

  .more {
    position: absolute;
    z-index: 1;
    top: 8px;
    right: 8px;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.25);
    color: #ffffff;
    opacity: 0;
    transition: opacity 150ms var(--ease);
    backdrop-filter: blur(8px);
  }

  .card:hover .more,
  .more:focus-visible {
    opacity: 1;
  }

  .more svg {
    stroke-width: 3;
  }

  .body {
    flex: 1;
    padding: 24px 14px 0;
  }

  h2 {
    margin: 0;
    overflow: hidden;
    font-size: 16px;
    font-weight: 650;
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
    margin: 4px 0 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
    font-weight: 500;
  }

  .chip {
    padding: 1px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent);
    font-size: 11px;
    font-weight: 600;
  }

  .error {
    color: var(--danger);
  }

  .play {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin: 12px 14px 14px;
    height: 38px;
    overflow: hidden;
    border: none;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
    box-shadow: 0 4px 14px color-mix(in srgb, var(--accent) 35%, transparent);
    transition:
      filter 150ms var(--ease),
      transform 100ms var(--ease);
  }

  .play::before {
    position: absolute;
    inset: 0;
    width: var(--progress);
    background: rgb(255 255 255 / 0.25);
    transition: width 200ms var(--ease);
    content: "";
  }

  .play span,
  .play svg {
    position: relative;
  }

  .play svg {
    width: 14px;
    height: 14px;
    fill: currentColor;
  }

  .play:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .play:active:not(:disabled) {
    transform: scale(0.98);
  }

  .play:disabled {
    cursor: default;
  }

  .play.running {
    background: var(--surface-hover);
    color: var(--text);
    box-shadow: none;
  }
</style>
