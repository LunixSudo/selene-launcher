<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { goto } from "$app/navigation";
  import { api, type Instance } from "$lib/api";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import ContextMenu, { type MenuItem } from "$lib/components/ContextMenu.svelte";
  import CreateInstanceDialog from "$lib/components/CreateInstanceDialog.svelte";
  import EditInstanceDialog from "$lib/components/EditInstanceDialog.svelte";
  import InstanceCard from "$lib/components/InstanceCard.svelte";
  import Select from "$lib/components/Select.svelte";
  import ShareDialog from "$lib/components/ShareDialog.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { UI_ICONS } from "$lib/icons";
  import {
    deleteInstance,
    importPack,
    library,
    loadInstances,
    play,
  } from "$lib/state/instances.svelte";
  import { SORT_ORDERS, type SortOrder, settings } from "$lib/state/settings.svelte";

  let creating = $state(false);
  let selected = $state<Instance | null>(null);
  let menuAt = $state<{ x: number; y: number } | null>(null);
  let editing = $state(false);
  let sharing = $state(false);
  let deleting = $state(false);
  let query = $state("");

  const menu = $derived.by((): MenuItem[] => {
    const instance = selected;
    if (!instance) {
      return [];
    }
    const status = library.launches[instance.id]?.status;
    const busy = status === "preparing" || status === "running";
    return [
      status === "running"
        ? {
            label: t("instances.stop"),
            icon: UI_ICONS.stop,
            action: () => api.stopInstance(instance.id),
          }
        : {
            label: t("instances.play"),
            icon: UI_ICONS.play,
            disabled: busy,
            action: () => play(instance.id),
          },
      {
        label: t("instances.continue"),
        icon: UI_ICONS.open,
        disabled: busy || !instance.lastPlayed,
        action: () => continueLastWorld(instance.id),
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
      {
        label: t("instances.duplicate"),
        icon: UI_ICONS.copy,
        disabled: busy,
        action: () => duplicate(instance),
      },
      { label: t("instance.share"), icon: UI_ICONS.share, action: () => (sharing = true) },
      {
        label: t("instances.shortcut"),
        icon: UI_ICONS.shortcut,
        action: () => api.createShortcut(instance.id),
      },
      {
        label: t("instances.delete"),
        icon: UI_ICONS.delete,
        danger: true,
        disabled: busy,
        action: () => (deleting = true),
      },
    ];
  });

  const sortOptions = $derived(
    SORT_ORDERS.map((value) => ({ value, label: t(`instances.sorts.${value}`) })),
  );

  const visible = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    const matches = library.instances.filter(
      (instance) =>
        !needle ||
        instance.name.toLowerCase().includes(needle) ||
        instance.version.includes(needle) ||
        (instance.loader?.loader.includes(needle) ?? false) ||
        (instance.group?.toLowerCase().includes(needle) ?? false),
    );
    return matches.sort(compare(settings.sort));
  });

  const groups = $derived.by(() => {
    const byName = new Map<string | null, Instance[]>();
    for (const instance of visible) {
      byName.set(instance.group, [...(byName.get(instance.group) ?? []), instance]);
    }
    return [...byName.entries()].sort(([a], [b]) =>
      a === null ? -1 : b === null ? 1 : a.localeCompare(b),
    );
  });

  function compare(order: SortOrder): (a: Instance, b: Instance) => number {
    const time = (date: string | null) => (date ? new Date(date).getTime() : 0);
    switch (order) {
      case "name":
        return (a, b) => a.name.localeCompare(b.name);
      case "playTime":
        return (a, b) => b.playSeconds - a.playSeconds;
      case "created":
        return (a, b) => time(b.createdAt) - time(a.createdAt);
      default:
        return (a, b) =>
          time(b.lastPlayed) - time(a.lastPlayed) || time(b.createdAt) - time(a.createdAt);
    }
  }

  function toggleGroup(name: string) {
    settings.collapsedGroups = settings.collapsedGroups.includes(name)
      ? settings.collapsedGroups.filter((group) => group !== name)
      : [...settings.collapsedGroups, name];
  }

  async function continueLastWorld(id: string) {
    const { worlds, quickPlay } = await api.worlds(id);
    const last = worlds[0];
    await play(id, quickPlay && last ? { kind: "world", value: last.folder } : null);
  }

  async function duplicate(instance: Instance) {
    const copy = await api.duplicateInstance(
      instance.id,
      t("instances.copyName", { name: instance.name }),
    );
    library.instances = [...library.instances, copy];
  }

  function showMenu(instance: Instance, x: number, y: number) {
    selected = instance;
    menuAt = { x, y };
  }

  $effect(() => {
    void loadInstances();
  });

  async function chooseImport() {
    const path = await open({
      filters: [{ name: t("instances.packFiles"), extensions: ["mrpack", "zip"] }],
    });
    if (path) {
      await importPack(path);
    }
  }
</script>

<section class="page">
  <header>
    <h1 class="page-title">{t("instances.title")}</h1>
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

  {#if library.importManual.length > 0}
    <div class="notice" role="alert">
      <p>{t("instance.manualDownload")}</p>
      {#each library.importManual as download (download.url)}
        <button type="button" class="link" onclick={() => openUrl(download.url)}>
          {download.title}
        </button>
      {/each}
    </div>
  {/if}

  {#if library.loaded && library.instances.length === 0}
    <div class="empty">
      <h2>{t("instances.empty")}</h2>
      <p>{t("instances.emptyHint")}</p>
    </div>
  {:else}
    <div class="filters">
      <input
        type="search"
        class="search"
        data-search
        bind:value={query}
        placeholder={t("instances.search")}
        aria-label={t("instances.search")}
      >
      <div class="sort">
        <Select bind:value={settings.sort} options={sortOptions} />
      </div>
    </div>

    <div class="scroll">
      {#if visible.length === 0}
        <p class="status">{t("instances.nothingFound")}</p>
      {/if}
      {#each groups as [name, instances] (name)}
        {@const collapsed = name !== null && settings.collapsedGroups.includes(name)}
        {#if name !== null || groups.length > 1}
          <button
            type="button"
            class="group"
            aria-expanded={!collapsed}
            onclick={() => name !== null && toggleGroup(name)}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 6l4 4 4-4" /></svg>
            {name ?? t("instances.ungrouped")}
            <span>{instances.length}</span>
          </button>
        {/if}
        {#if !collapsed}
          <div class="grid">
            {#each instances as instance (instance.id)}
              <InstanceCard
                {instance}
                launch={library.launches[instance.id]}
                onmenu={(x, y) => showMenu(instance, x, y)}
              />
            {/each}
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</section>

<CreateInstanceDialog bind:open={creating} />
<ContextMenu items={menu} bind:position={menuAt} />
{#if selected}
  <EditInstanceDialog instance={selected} bind:open={editing} />
  <ShareDialog instance={selected} bind:open={sharing} />
  {@const instance = selected}
  <ConfirmDialog
    bind:open={deleting}
    title={t("delete.title", { name: instance.name })}
    message={t("delete.message")}
    confirmLabel={t("delete.confirm")}
    onconfirm={() => deleteInstance(instance.id)}
  />
{/if}

<style>
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 8px;
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

  .notice {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-top: -8px;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
  }

  .notice p {
    flex-basis: 100%;
    margin: 0;
  }

  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    cursor: pointer;
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

  .filters {
    display: flex;
    gap: 8px;
    margin-top: -6px;
  }

  .search {
    flex: 1;
    max-width: 360px;
    height: 36px;
    padding: 0 16px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  .sort {
    width: 210px;
  }

  .scroll {
    flex: 1;
    min-height: 0;
    margin: -6px;
    overflow-y: auto;
    padding: 6px 6px 10px;
  }

  .group {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 10px 0 10px;
    padding: 0;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 600;
  }

  .group:first-child {
    margin-top: 0;
  }

  .group span {
    padding: 0 7px;
    border-radius: 999px;
    background: var(--field);
    font-size: 11px;
  }

  .group svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 150ms var(--ease);
  }

  .group[aria-expanded="false"] svg {
    transform: rotate(-90deg);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 16px;
  }

  .grid + .group {
    margin-top: 22px;
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
    font-weight: var(--display-weight);
  }

  .empty p {
    margin: 0;
    color: var(--text-muted);
  }
</style>
