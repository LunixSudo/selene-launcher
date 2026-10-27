<script lang="ts">
  import CreateInstanceDialog from "$lib/components/CreateInstanceDialog.svelte";
  import InstanceCard from "$lib/components/InstanceCard.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { library, loadInstances } from "$lib/state/instances.svelte";

  let creating = $state(false);

  $effect(() => {
    void loadInstances();
  });
</script>

<section class="page">
  <header>
    <h1>{t("instances.title")}</h1>
    <button type="button" class="create" onclick={() => (creating = true)}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 3v10M3 8h10" /></svg>
      {t("instances.create")}
    </button>
  </header>

  {#if library.loaded && library.instances.length === 0}
    <div class="empty">
      <h2>{t("instances.empty")}</h2>
      <p>{t("instances.emptyHint")}</p>
    </div>
  {:else}
    <div class="grid">
      {#each library.instances as instance (instance.id)}
        <InstanceCard {instance} launch={library.launches[instance.id]} />
      {/each}
    </div>
  {/if}
</section>

<CreateInstanceDialog bind:open={creating} />

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 20px;
    height: 100%;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 8px;
  }

  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 28px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .create {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    font-weight: 600;
    backdrop-filter: blur(24px);
    transition: background 150ms var(--ease);
  }

  .create:hover {
    background: var(--surface-hover);
  }

  .create svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 16px;
    margin: -6px;
    overflow-y: auto;
    padding: 6px 6px 10px;
  }

  .empty {
    display: grid;
    flex: 1;
    place-content: center;
    gap: 6px;
    border: 1px dashed var(--border);
    border-radius: var(--radius-lg);
    text-align: center;
  }

  .empty h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 600;
  }

  .empty p {
    margin: 0;
    color: var(--text-muted);
  }
</style>
