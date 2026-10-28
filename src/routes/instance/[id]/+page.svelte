<script lang="ts">
  import { page } from "$app/state";
  import { api } from "$lib/api";
  import CrashNotice from "$lib/components/CrashNotice.svelte";
  import GameConsole from "$lib/components/GameConsole.svelte";
  import InstanceSettings from "$lib/components/InstanceSettings.svelte";
  import ModCatalog from "$lib/components/ModCatalog.svelte";
  import ModList from "$lib/components/ModList.svelte";
  import ScreenshotGallery from "$lib/components/ScreenshotGallery.svelte";
  import ServerList from "$lib/components/ServerList.svelte";
  import ShareDialog from "$lib/components/ShareDialog.svelte";
  import WorldList from "$lib/components/WorldList.svelte";
  import { playTime } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import { LOADER_NAMES } from "$lib/loaders";
  import { library, loadInstances, play } from "$lib/state/instances.svelte";

  type Tab = "mods" | "catalog" | "worlds" | "servers" | "screenshots" | "settings" | "logs";

  let chosenTab = $state<Tab | null>(null);
  let refresh = $state(0);
  let sharing = $state(false);

  const instance = $derived(library.instances.find((entry) => entry.id === page.params.id));
  const launch = $derived(instance ? library.launches[instance.id] : undefined);
  const running = $derived(launch?.status === "running");
  const COMMON_TABS: Tab[] = ["worlds", "servers", "screenshots", "settings", "logs"];

  const tabs = $derived<Tab[]>(
    instance?.loader ? ["mods", "catalog", ...COMMON_TABS] : COMMON_TABS,
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
      <a class="back" href="/">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
        {t("instance.back")}
      </a>
      <div class="heading">
        <h1>{instance.name}</h1>
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
          disabled={launch?.status === "preparing"}
          onclick={() => (running ? api.stopInstance(instance.id) : play(instance.id))}
        >
          {running ? t("instances.stop") : t("instances.play")}
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

    <div class="content" role="tabpanel">
      {#if tab === "mods"}
        <ModList instanceId={instance.id} {refresh} />
      {:else if tab === "catalog"}
        <ModCatalog instanceId={instance.id} oninstall={() => refresh++} />
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
  </section>

  <ShareDialog {instance} bind:open={sharing} />
{/if}

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
    height: 100%;
  }

  header {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: end;
    gap: 4px 16px;
    padding-top: 4px;
  }

  .back {
    display: inline-flex;
    grid-column: 1 / -1;
    align-items: center;
    gap: 4px;
    width: fit-content;
    color: var(--accent);
    font-size: 13px;
    text-decoration: none;
  }

  .back svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .heading {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  h1 {
    margin: 0;
    overflow: hidden;
    font-family: var(--font-display);
    font-size: 28px;
    font-weight: var(--display-weight);
    letter-spacing: -0.02em;
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

  .play {
    height: 38px;
    padding: 0 28px;
    border: none;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
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

  .muted {
    color: var(--text-muted);
  }
</style>
