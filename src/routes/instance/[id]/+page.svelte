<script lang="ts">
  import { page } from "$app/state";
  import { api, type ContentKind } from "$lib/api";
  import ContentCatalog from "$lib/components/ContentCatalog.svelte";
  import ContentList from "$lib/components/ContentList.svelte";
  import CrashNotice from "$lib/components/CrashNotice.svelte";
  import GameConsole from "$lib/components/GameConsole.svelte";
  import InstanceSettings from "$lib/components/InstanceSettings.svelte";
  import ScreenshotGallery from "$lib/components/ScreenshotGallery.svelte";
  import Segmented from "$lib/components/Segmented.svelte";
  import ServerList from "$lib/components/ServerList.svelte";
  import ShareDialog from "$lib/components/ShareDialog.svelte";
  import UpdateNotice from "$lib/components/UpdateNotice.svelte";
  import WorldList from "$lib/components/WorldList.svelte";
  import WorldPicker from "$lib/components/WorldPicker.svelte";
  import { playTime } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import { LOADER_NAMES } from "$lib/loaders";
  import { scrollFade } from "$lib/scroll-fade";
  import { activity, library, loadInstances, play } from "$lib/state/instances.svelte";

  type Tab = "content" | "catalog" | "worlds" | "servers" | "screenshots" | "settings" | "logs";

  let chosenTab = $state<Tab | null>(null);
  let refresh = $state(0);
  let chosenKind = $state<ContentKind>("mod");
  let world = $state<string | null>(null);
  let sharing = $state(false);
  let recheck = $state(0);
  let checking = $state(false);

  const instance = $derived(library.instances.find((entry) => entry.id === page.params.id));
  const launch = $derived(instance ? library.launches[instance.id] : undefined);
  const running = $derived(launch?.status === "running");
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
        <span class="chip">
          {instance.loader
            ? `${LOADER_NAMES[instance.loader.loader]} ${instance.loader.version} · ${instance.version}`
            : instance.version}
        </span>
        {#if instance.playSeconds >= 60}
          <span class="played">
            {t("instances.playTime", { time: playTime(instance.playSeconds) })}
          </span>
        {/if}
      </div>
      <div class="actions">
        <button
          type="button"
          class="share refresh"
          class:checking
          disabled={checking}
          title={t("updates.check")}
          aria-label={t("updates.check")}
          onclick={() => recheck++}
        >
          <svg class="spinner" viewBox="0 0 16 16" aria-hidden="true">
            <path d="M13 8a5 5 0 1 1-1.46-3.54" />
            <path d="M13 2.5v3h-3" />
          </svg>
        </button>
        <button type="button" class="share" onclick={() => api.openInstanceFolder(instance.id)}>
          {t("instances.openFolder")}
        </button>
        <button type="button" class="share" onclick={() => (sharing = true)}>
          {t("instance.share")}
        </button>
        <button
          type="button"
          class="play"
          class:running
          disabled={busy !== null}
          style:--progress={busy ? `${busy.percent}%` : "0%"}
          onclick={() => (running ? api.stopInstance(instance.id) : play(instance.id))}
        >
          <span>{running ? t("instances.stop") : (busy?.label ?? t("instances.play"))}</span>
        </button>
      </div>
    </header>

    {#if crash}
      <CrashNotice
        {crash}
        onlogs={() => (chosenTab = "logs")}
        ondismiss={() => delete library.crashes[instance.id]}
      />
    {/if}

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
  header {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: end;
    gap: 4px 16px;
    padding-top: 4px;
  }

  .back-link {
    grid-column: 1 / -1;
  }

  .heading {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  h1 {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip {
    flex: none;
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--field);
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 600;
  }

  .played {
    flex: none;
    color: var(--text-muted);
    font-size: 12px;
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .share {
    height: 38px;
    padding: 0 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-weight: 600;
  }

  .share:hover {
    background: var(--surface-hover);
  }

  .refresh {
    display: grid;
    place-items: center;
    width: 38px;
    padding: 0;
  }

  .refresh svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .refresh.checking svg {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .play {
    position: relative;
    height: 38px;
    padding: 0 28px;
    overflow: hidden;
    border: none;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
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

  .play:disabled {
    opacity: 0.7;
  }

  .play.running {
    background: var(--danger);
    color: #ffffff;
  }

  .tabs {
    display: inline-flex;
    gap: 2px;
    width: fit-content;
    padding: 3px;
    border-radius: 999px;
    background: var(--field);
  }

  .tabs button {
    height: 30px;
    padding: 0 18px;
    border: none;
    border-radius: 999px;
    background: transparent;
    font-size: 13px;
  }

  .tabs button[aria-selected="true"] {
    background: var(--surface-solid);
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.12);
    font-weight: 600;
  }

  .content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: 8px;
  }

  .kinds {
    display: flex;
    margin-bottom: 10px;
  }

  .muted {
    color: var(--text-muted);
  }
</style>
