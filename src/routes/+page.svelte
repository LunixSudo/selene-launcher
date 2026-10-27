<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { goto } from "$app/navigation";
  import { api, type Instance } from "$lib/api";
  import ContextMenu, { type MenuItem } from "$lib/components/ContextMenu.svelte";
  import CreateInstanceDialog from "$lib/components/CreateInstanceDialog.svelte";
  import DeleteInstanceDialog from "$lib/components/DeleteInstanceDialog.svelte";
  import EditInstanceDialog from "$lib/components/EditInstanceDialog.svelte";
  import InstanceCard from "$lib/components/InstanceCard.svelte";
  import ShareDialog from "$lib/components/ShareDialog.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { UI_ICONS } from "$lib/icons";
  import { importPack, library, loadInstances, play } from "$lib/state/instances.svelte";

  let creating = $state(false);
  let selected = $state<Instance | null>(null);
  let menuAt = $state<{ x: number; y: number } | null>(null);
  let editing = $state(false);
  let sharing = $state(false);
  let deleting = $state(false);

  const menu = $derived.by((): MenuItem[] => {
    const instance = selected;
    if (!instance) {
      return [];
    }
    const status = library.launches[instance.id]?.status;
    const busy = status === "preparing" || status === "running";
    return [
      {
        label: t("instances.play"),
        icon: UI_ICONS.play,
        disabled: busy,
        action: () => play(instance.id),
      },
      {
        label: t("instances.open"),
        icon: UI_ICONS.open,
        action: () => goto(`/instance/${instance.id}`),
      },
      {
        label: t("instances.openFolder"),
        icon: UI_ICONS.folder,
        action: () => api.openInstanceFolder(instance.id),
      },
      { label: t("instances.edit"), icon: UI_ICONS.edit, action: () => (editing = true) },
      { label: t("instance.share"), icon: UI_ICONS.share, action: () => (sharing = true) },
      {
        label: t("instances.delete"),
        icon: UI_ICONS.delete,
        danger: true,
        disabled: busy,
        action: () => (deleting = true),
      },
    ];
  });

  function showMenu(instance: Instance, x: number, y: number) {
    selected = instance;
    menuAt = { x, y };
  }

  $effect(() => {
    void loadInstances();
  });

  async function chooseImport() {
    const path = await open({ filters: [{ name: "Modrinth", extensions: ["mrpack"] }] });
    if (path) {
      await importPack(path);
    }
  }
</script>

<section class="page">
  <header>
    <h1>{t("instances.title")}</h1>
    <div class="actions">
      <button type="button" class="create" onclick={chooseImport}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 3v7M5 7l3 3 3-3M3 13h10" /></svg>
        {t("instances.import")}
      </button>
      <button type="button" class="create" onclick={() => (creating = true)}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 3v10M3 8h10" /></svg>
        {t("instances.create")}
      </button>
    </div>
  </header>

  {#if library.importing > 0}
    <p class="status">{t("instances.importing")}</p>
  {:else if library.importError}
    <p class="status error" role="alert">{t(`errors.${library.importError.kind}`)}</p>
  {/if}

  {#if library.loaded && library.instances.length === 0}
    <div class="empty">
      <h2>{t("instances.empty")}</h2>
      <p>{t("instances.emptyHint")}</p>
    </div>
  {:else}
    <div class="grid">
      {#each library.instances as instance (instance.id)}
        <InstanceCard
          {instance}
          launch={library.launches[instance.id]}
          onmenu={(x, y) => showMenu(instance, x, y)}
        />
      {/each}
    </div>
  {/if}
</section>

<CreateInstanceDialog bind:open={creating} />
<ContextMenu items={menu} bind:position={menuAt} />
{#if selected}
  <EditInstanceDialog instance={selected} bind:open={editing} />
  <ShareDialog instance={selected} bind:open={sharing} />
  <DeleteInstanceDialog instance={selected} bind:open={deleting} />
{/if}

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

  .actions {
    display: flex;
    gap: 8px;
  }

  .status {
    margin: -8px 0 0;
    color: var(--text-muted);
  }

  .status.error {
    color: var(--danger);
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
