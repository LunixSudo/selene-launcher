<script lang="ts">
  import type { Instance } from "$lib/api";
  import { locale, t } from "$lib/i18n/index.svelte";
  import { LOADER_NAMES } from "$lib/loaders";
  import { deleteInstance, type LaunchState, play } from "$lib/state/instances.svelte";

  let { instance, launch }: { instance: Instance; launch: LaunchState | undefined } = $props();

  let confirmingDelete = $state(false);

  const busy = $derived(launch?.status === "preparing" || launch?.status === "running");
  const hue = $derived(hash(instance.id) % 360);

  function hash(text: string): number {
    let value = 0;
    for (const char of text) {
      value = (value * 31 + char.charCodeAt(0)) >>> 0;
    }
    return value;
  }

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

  async function remove() {
    if (!confirmingDelete) {
      confirmingDelete = true;
      return;
    }
    await deleteInstance(instance.id);
  }
</script>

<article class="card" style:--hue={hue}>
  <div class="cover">
    <span class="version">
      {instance.loader
        ? `${LOADER_NAMES[instance.loader.loader]} ${instance.version}`
        : instance.version}
    </span>
    <button
      type="button"
      class="delete"
      class:confirming={confirmingDelete}
      disabled={busy}
      aria-label={t("instances.delete")}
      onclick={remove}
      onblur={() => (confirmingDelete = false)}
    >
      {#if confirmingDelete}
        {t("instances.confirmDelete")}
      {:else}
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5" />
        </svg>
      {/if}
    </button>
  </div>

  <div class="body">
    <h2>{instance.name}</h2>
    <p>{lastPlayed()}</p>
    {#if launch?.status === "failed"}
      <p class="error" role="alert">{t(`errors.${launch.error.kind}`)}</p>
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
    <span>{playLabel()}</span>
  </button>
</article>

<style>
  .card {
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
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.12);
  }

  .cover {
    position: relative;
    height: 96px;
    background:
      radial-gradient(circle at 20% 20%, hsl(var(--hue) 90% 72% / 0.9), transparent 60%),
      linear-gradient(135deg, hsl(var(--hue) 70% 55%), hsl(calc(var(--hue) + 50) 70% 45%));
  }

  .version {
    position: absolute;
    bottom: 10px;
    left: 12px;
    padding: 2px 8px;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.28);
    color: #ffffff;
    font-size: 12px;
    font-weight: 600;
    backdrop-filter: blur(8px);
  }

  .delete {
    position: absolute;
    top: 8px;
    right: 8px;
    display: grid;
    place-items: center;
    min-width: 28px;
    height: 28px;
    padding: 0 6px;
    border: none;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.28);
    color: #ffffff;
    font-size: 12px;
    opacity: 0;
    transition:
      opacity 150ms var(--ease),
      background 150ms var(--ease);
    backdrop-filter: blur(8px);
  }

  .card:hover .delete,
  .delete:focus-visible,
  .delete.confirming {
    opacity: 1;
  }

  .delete.confirming {
    padding: 0 10px;
    background: var(--danger);
  }

  .delete:disabled {
    display: none;
  }

  .delete svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .body {
    flex: 1;
    padding: 12px 14px 0;
  }

  h2 {
    margin: 0;
    overflow: hidden;
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  p {
    margin: 4px 0 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  .error {
    color: var(--danger);
  }

  .play {
    position: relative;
    margin: 12px 14px 14px;
    height: 36px;
    overflow: hidden;
    border: none;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
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

  .play span {
    position: relative;
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
  }
</style>
