<script lang="ts">
  import { page } from "$app/state";
  import ModCatalog from "$lib/components/ModCatalog.svelte";
  import ModList from "$lib/components/ModList.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { LOADER_NAMES } from "$lib/loaders";
  import { library, loadInstances, play } from "$lib/state/instances.svelte";

  type Tab = "mods" | "catalog";

  let tab = $state<Tab>("mods");
  let refresh = $state(0);

  const instance = $derived(library.instances.find((entry) => entry.id === page.params.id));
  const launch = $derived(instance ? library.launches[instance.id] : undefined);
  const busy = $derived(launch?.status === "preparing" || launch?.status === "running");

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
      </div>
      <button type="button" class="play" disabled={busy} onclick={() => play(instance.id)}>
        {launch?.status === "running" ? t("instances.running") : t("instances.play")}
      </button>
    </header>

    {#if instance.loader}
      <div class="tabs" role="tablist">
        {#each ["mods", "catalog"] as const as name (name)}
          <button
            type="button"
            role="tab"
            aria-selected={tab === name}
            onclick={() => (tab = name)}
          >
            {t(`instance.${name}`)}
          </button>
        {/each}
      </div>

      <div class="content" role="tabpanel">
        {#if tab === "mods"}
          <ModList instanceId={instance.id} {refresh} />
        {:else}
          <ModCatalog instanceId={instance.id} oninstall={() => refresh++} />
        {/if}
      </div>
    {:else}
      <p class="muted">{t("instance.noLoader")}</p>
    {/if}
  </section>
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
    font-weight: 600;
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
