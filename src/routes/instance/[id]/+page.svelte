<script lang="ts">
  import { api, type ContentKind } from "#lib/api.ts";
  import ContentCatalog from "#lib/components/ContentCatalog.svelte";
  import ContentList from "#lib/components/ContentList.svelte";
  import CrashNotice from "#lib/components/CrashNotice.svelte";
  import GameConsole from "#lib/components/GameConsole.svelte";
  import InstanceSettings from "#lib/components/InstanceSettings.svelte";
  import MoonPhase from "#lib/components/MoonPhase.svelte";
  import ReportDialog from "#lib/components/ReportDialog.svelte";
  import ScreenshotGallery from "#lib/components/ScreenshotGallery.svelte";
  import Segmented from "#lib/components/Segmented.svelte";
  import ServerList from "#lib/components/ServerList.svelte";
  import ShareDialog from "#lib/components/ShareDialog.svelte";
  import UpdateNotice from "#lib/components/UpdateNotice.svelte";
  import WorldList from "#lib/components/WorldList.svelte";
  import WorldPicker from "#lib/components/WorldPicker.svelte";
  import { playTime } from "#lib/format.ts";
  import { t } from "#lib/i18n/index.svelte.ts";
  import { UI_ICONS } from "#lib/icons.ts";
  import { LOADER_NAMES } from "#lib/loaders.ts";
  import { scrollFade } from "#lib/scroll-fade.ts";
  import { activity, library, loadInstances, play, stop } from "#lib/state/instances.svelte.ts";
  import { tooltip } from "#lib/tooltip.ts";
  import { page } from "$app/state";

  type Tab = "content" | "catalog" | "worlds" | "servers" | "screenshots" | "settings" | "logs";

  let chosenTab = $state<Tab | null>(null);
  let refresh = $state(0);
  let chosenKind = $state<ContentKind>("mod");
  let world = $state<string | null>(null);
  let sharing = $state(false);
  let reporting = $state(false);
  let recheck = $state(0);
  let checking = $state(false);

  const instance = $derived(library.instances.find((entry) => entry.id === page.params.id));
  const launch = $derived(instance ? library.launches[instance.id] : undefined);
  const running = $derived(launch?.status === "running");
  const stopping = $derived(launch?.status === "running" && launch.stopping === true);
  const busy = $derived(instance && !running ? activity(instance.id) : null);
  const COMMON_TABS: Tab[] = ["worlds", "servers", "screenshots", "settings", "logs"];

  const tabs: Tab[] = ["content", "catalog", ...COMMON_TABS];
  const kinds = $derived<ContentKind[]>(
    instance?.loader
      ? ["mod", "resourcePack", "shaderPack", "dataPack"]
      : ["resourcePack", "dataPack"],
  );
  const kind = $derived(kinds.includes(chosenKind) ? chosenKind : kinds[0]);
  const kindOptions = $derived(
    kinds.map((value) => ({ value, label: t(`content.kinds.${value}`) })),
  );
  const crash = $derived(instance ? library.crashes[instance.id] : undefined);
  const tab = $derived(chosenTab ?? tabs[0]);

  $effect(() => {
    if (!library.loaded) {
      void loadInstances();
    }
  });
</script>

{#if instance}
  <section class="page">
    <header>
      <a class="back-link" href="/">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
        {t("instance.back")}
      </a>
      <div class="heading">
        <h1 class="page-title">{instance.name}</h1>
        <div class="facts">
          <span class="fact">
            <span class="chip">
              {instance.loader ? LOADER_NAMES[instance.loader.loader] : t("instances.vanilla")}
            </span>
            {instance.loader
              ? `${instance.loader.version} · ${instance.version}`
              : instance.version}
          </span>
          {#if instance.playSeconds >= 60}
            <span class="fact">
              {t("instances.playTime", { time: playTime(instance.playSeconds) })}
            </span>
          {/if}
        </div>
      </div>
      <div class="actions">
        <button
          type="button"
          class="round"
          class:checking
          disabled={checking}
          aria-label={t("updates.check")}
          {@attach tooltip(t("updates.check"))}
          onclick={() => recheck++}
        >
          <svg class="spinner" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M19 12a7 7 0 1 1-2.05-4.95M19 4.5V8h-3.5" />
          </svg>
        </button>
        <button
          type="button"
          class="round"
          aria-label={t("instances.openFolder")}
          {@attach tooltip(t("instances.openFolder"))}
          onclick={() => api.openInstanceFolder(instance.id)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.folder} /></svg>
        </button>
        <button
          type="button"
          class="round"
          aria-label={t("instance.share")}
          {@attach tooltip(t("instance.share"))}
          onclick={() => (sharing = true)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.share} /></svg>
        </button>
        <button
          type="button"
          class="play"
          class:running
          disabled={(busy !== null && !busy.background) || stopping}
          style:--progress={busy ? `${busy.percent}%` : "0%"}
          onclick={() => (running ? stop(instance.id) : play(instance.id))}
        >
          {#if busy && !busy.background}
            <MoonPhase percent={busy.percent} />
          {:else}
            <svg class="glyph" viewBox="0 0 24 24" aria-hidden="true">
              <path d={running ? UI_ICONS.stop : UI_ICONS.play} />
            </svg>
          {/if}
          <span>
            {#if running}
              {stopping ? t("instances.stopping") : t("instances.stop")}
            {:else}
              {busy?.label ?? t("instances.play")}
            {/if}
          </span>
        </button>
      </div>
    </header>

    {#if crash}
      <CrashNotice
        {crash}
        onlogs={() => (chosenTab = "logs")}
        onreport={() => (reporting = true)}
        ondismiss={() => delete library.crashes[instance.id]}
      />
    {/if}

    <ReportDialog bind:open={reporting} instanceId={instance.id} />

    <UpdateNotice
      {instance}
      {recheck}
      bind:checking
      disabled={running || library.tasks[instance.id] !== undefined}
      onupdated={() => refresh++}
    />

    <div class="tabs" role="tablist">
      {#each tabs as name (name)}
        <button
          type="button"
          role="tab"
          aria-selected={tab === name}
          onclick={() => (chosenTab = name)}
        >
          {t(`instance.${name}`)}
        </button>
      {/each}
    </div>

    {#key tab}
      <div class="content" role="tabpanel" {@attach scrollFade}>
        {#if tab === "content"}
          <div class="kit">
            <div class="kinds">
              <Segmented
                bind:value={() => kind, (value) => (chosenKind = value)}
                options={kindOptions}
                name="content-kind"
                label={t("instance.content")}
              />
            </div>
            {#if kind === "dataPack"}
              <WorldPicker instanceId={instance.id} bind:value={world} />
              {#if world}
                <ContentList instanceId={instance.id} {kind} {world} {refresh} />
              {/if}
            {:else}
              <ContentList instanceId={instance.id} {kind} {refresh} />
            {/if}
            <button
              type="button"
              class="pill-button primary add"
              onclick={() => (chosenTab = "catalog")}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
              {t("instance.add")}
            </button>
          </div>
        {:else if tab === "catalog"}
          <ContentCatalog
            instanceId={instance.id}
            {kinds}
            bind:kind={chosenKind}
            bind:world
            oninstall={() => refresh++}
          />
        {:else if tab === "worlds"}
          <WorldList instanceId={instance.id} />
        {:else if tab === "servers"}
          <ServerList instanceId={instance.id} />
        {:else if tab === "screenshots"}
          <ScreenshotGallery instanceId={instance.id} />
        {:else if tab === "logs"}
          <GameConsole instanceId={instance.id} />
        {:else}
          {#if !instance.loader}
            <p class="muted">{t("instance.noLoader")}</p>
          {/if}
          <InstanceSettings {instance} />
        {/if}
      </div>
    {/key}
  </section>

  <ShareDialog {instance} bind:open={sharing} />
{/if}

<style>
  /* In a narrow window the buttons move under the name instead of squeezing it. */
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 6px 20px;
    padding-top: 4px;
  }

  .back-link {
    flex-basis: 100%;
  }

  .heading {
    flex: 1 1 380px;
    min-width: 0;
  }

  h1 {
    overflow: hidden;
    font-size: 40px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 14px;
  }

  .fact {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 13px;
    border-radius: 999px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
    font-size: 13.5px;
    font-weight: 750;
    white-space: nowrap;
    backdrop-filter: var(--glass);
  }

  .fact:has(.chip) {
    padding-left: 4px;
  }

  .chip {
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-fg);
    font-size: 11.5px;
    font-weight: 800;
  }

  .actions {
    display: flex;
    flex: none;
    align-items: center;
    gap: 10px;
    margin-left: auto;
    white-space: nowrap;
  }

  .actions svg {
    width: 19px;
    height: 19px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .round {
    display: grid;
    flex: none;
    place-items: center;
    width: 46px;
    height: 46px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
    color: var(--text-2);
    backdrop-filter: var(--glass);
    transition:
      background 150ms var(--ease),
      color 150ms var(--ease),
      transform 260ms var(--spring);
  }

  .round:hover:enabled {
    background: var(--surface-hover);
    color: var(--text);
  }

  .round:active:enabled {
    transform: scale(0.94);
  }

  .round.checking svg {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .play {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 9px;
    min-width: 172px;
    height: 46px;
    padding: 0 28px;
    overflow: hidden;
    border: none;
    border-radius: 999px;
    background: var(--accent);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.22),
      0 10px 24px -10px color-mix(in srgb, var(--accent) 80%, transparent);
    color: var(--accent-text);
    font-size: 15.5px;
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

  .play .glyph {
    width: 15px;
    height: 15px;
    fill: currentColor;
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
    background: var(--danger);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.2),
      0 10px 24px -12px var(--danger);
    color: #ffffff;
  }

  .tabs {
    display: flex;
    flex: none;
    gap: 2px;
    width: fit-content;
    max-width: 100%;
    padding: 5px;
    overflow-x: auto;
    border-radius: 999px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
    scrollbar-width: none;
    backdrop-filter: var(--glass);
  }

  .tabs button {
    flex: none;
    height: 38px;
    padding: 0 17px;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: var(--text-2);
    font-size: 14.5px;
    font-weight: 750;
    transition:
      background 150ms var(--ease),
      color 150ms var(--ease);
  }

  .tabs button:hover {
    color: var(--text);
  }

  .tabs button[aria-selected="true"] {
    background: var(--accent);
    box-shadow: 0 6px 16px -8px color-mix(in srgb, var(--accent) 80%, transparent);
    color: var(--accent-text);
  }

  .content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: 8px;
  }

  /* The kinds, the list's own buttons and Add share one line above the list. */
  .kit {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    column-gap: 10px;
  }

  .kit > :global(*) {
    flex: 1 1 100%;
    order: 3;
  }

  .kit > .kinds {
    display: flex;
    flex: none;
    order: 0;
    margin-bottom: 12px;
  }

  .kit > :global(.toolbar) {
    flex: 1 1 0;
    order: 1;
  }

  .kit > .add {
    display: flex;
    flex: none;
    align-items: center;
    gap: 6px;
    order: 2;
    height: 38px;
    margin-bottom: 12px;
  }

  .kit > .add svg {
    width: 17px;
    height: 17px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .muted {
    color: var(--text-muted);
  }
</style>
