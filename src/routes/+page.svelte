<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type Instance } from "#lib/api.ts";
  import ConfirmDialog from "#lib/components/ConfirmDialog.svelte";
  import ContextMenu, { type MenuItem } from "#lib/components/ContextMenu.svelte";
  import CreateInstanceDialog from "#lib/components/CreateInstanceDialog.svelte";
  import EditInstanceDialog from "#lib/components/EditInstanceDialog.svelte";
  import InstanceCard from "#lib/components/InstanceCard.svelte";
  import Select from "#lib/components/Select.svelte";
  import ShareDialog from "#lib/components/ShareDialog.svelte";
  import { playTime } from "#lib/format.ts";
  import { t } from "#lib/i18n/index.svelte.ts";
  import { UI_ICONS } from "#lib/icons.ts";
  import { scrollFade } from "#lib/scroll-fade.ts";
  import { settle } from "#lib/settle.ts";
  import { shortcutIcon } from "#lib/shortcut-icon.ts";
  import {
    deleteInstance,
    importPack,
    library,
    loadInstances,
    play,
    stop,
  } from "#lib/state/instances.svelte.ts";
  import {
    type InstanceView,
    SORT_ORDERS,
    type SortOrder,
    settings,
    togglePinned,
  } from "#lib/state/settings.svelte.ts";
  import { tooltip } from "#lib/tooltip.ts";
  import { goto } from "$app/navigation";

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
            action: () => stop(instance.id),
          }
        : {
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
      {
        label: settings.pinned.includes(instance.id) ? t("instances.unpin") : t("instances.pin"),
        icon: UI_ICONS.pin,
        action: () => togglePinned(instance.id),
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
        action: async () => api.createShortcut(instance.id, await shortcutIcon(instance)),
      },
      {
        label: t("instances.delete"),
        icon: UI_ICONS.delete,
        danger: true,
        disabled: status === "running",
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

  // The last played instance gets a wide tile while the list runs from the last played.
  const featured = $derived(
    settings.instanceView === "grid" &&
      settings.sort === "lastPlayed" &&
      !query.trim() &&
      visible[0]?.lastPlayed
      ? visible[0].id
      : null,
  );

  const VIEWS: { value: InstanceView; icon: string }[] = [
    { value: "grid", icon: "M4.5 4.5h6v6h-6zM13.5 4.5h6v6h-6zM4.5 13.5h6v6h-6zM13.5 13.5h6v6h-6z" },
    { value: "list", icon: "M4.5 6.5h15M4.5 12h15M4.5 17.5h15" },
  ];

  const played = $derived.by(() => {
    let total = 0;
    let most: Instance | null = null;
    for (const instance of library.instances) {
      total += instance.playSeconds;
      if (!most || instance.playSeconds > most.playSeconds) {
        most = instance;
      }
    }
    return most && total >= 60 ? { time: playTime(total), name: most.name } : null;
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
    <div class="heading">
      <h1 class="page-title">{t("instances.title")}</h1>
      {#if played}
        <p class="played">{t("instances.totalPlayTime", played)}</p>
      {/if}
    </div>
    <div class="actions">
      <button type="button" class="pill-button" onclick={chooseImport}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 4.5v10M8 10.5l4 4 4-4M5 19.5h14" />
        </svg>
        {t("instances.import")}
      </button>
      <button type="button" class="pill-button primary" onclick={() => (creating = true)}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
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
      <p>{t("instance.manualPackDownload")}</p>
      {#each library.importManual as download (download.url)}
        <button type="button" class="link" onclick={() => openUrl(download.url)}>
          {download.title}
        </button>
      {/each}
    </div>
  {/if}

  {#if library.loadError}
    <p class="status" role="alert">{t(`errors.${library.loadError.kind}`)}</p>
  {:else if library.loaded && library.instances.length === 0}
    <div class="empty">
      <h2>{t("instances.empty")}</h2>
      <p>{t("instances.emptyHint")}</p>
      <div class="empty-actions">
        <a class="pill-button" href="/modpacks">{t("modpacks.title")}</a>
        <button type="button" class="pill-button primary" onclick={() => (creating = true)}>
          {t("instances.create")}
        </button>
      </div>
    </div>
  {:else}
    <div class="filters">
      <label class="search">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M10.5 4a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM15.5 15.5 20 20" />
        </svg>
        <input
          type="search"
          data-search
          bind:value={query}
          placeholder={t("instances.search")}
          aria-label={t("instances.search")}
        >
        <kbd>Ctrl F</kbd>
      </label>
      <div class="sort">
        <Select bind:value={settings.sort} options={sortOptions} />
      </div>
      <fieldset class="views">
        <legend class="sr-only">{t("instances.view")}</legend>
        {#each VIEWS as view (view.value)}
          <button
            type="button"
            aria-pressed={settings.instanceView === view.value}
            aria-label={t(`instances.views.${view.value}`)}
            {@attach tooltip(t(`instances.views.${view.value}`))}
            onclick={() => (settings.instanceView = view.value)}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d={view.icon} /></svg>
          </button>
        {/each}
      </fieldset>
    </div>

    <div class="scroll" {@attach scrollFade}>
      {#if library.loaded && visible.length === 0}
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
          <div class="grid" class:list={settings.instanceView === "list"} {@attach settle}>
            {#each instances as instance (instance.id)}
              <InstanceCard
                {instance}
                launch={library.launches[instance.id]}
                onmenu={(x, y) => showMenu(instance, x, y)}
                variant={settings.instanceView === "list"
                  ? "row"
                  : instance.id === featured
                    ? "wide"
                    : "card"}
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
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding-top: 8px;
  }

  .actions {
    display: flex;
    flex: none;
    gap: 10px;
  }

  .actions .pill-button {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 42px;
    padding: 0 20px;
    font-size: 14.5px;
  }

  .actions svg,
  .search svg,
  .views svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .heading {
    min-width: 0;
  }

  .played {
    margin: 6px 0 0;
    overflow: hidden;
    color: var(--text-2);
    font-size: 14px;
    font-weight: 650;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .filters {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .search {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 10px;
    max-width: 390px;
    height: 42px;
    padding: 0 8px 0 16px;
    border-radius: 999px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
    color: var(--text-muted);
  }

  .search input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-weight: 650;
    outline: none;
  }

  .search input::-webkit-search-cancel-button {
    display: none;
  }

  .search:focus-within {
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 2px var(--accent);
  }

  kbd {
    flex: none;
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--field);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-2);
    font-family: inherit;
    font-size: 11.5px;
    font-weight: 750;
  }

  .sort {
    width: 220px;
  }

  .sort :global(.field) {
    height: 42px;
    padding: 0 14px 0 18px;
    border: none;
    border-radius: 999px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
    font-weight: 700;
  }

  .sort :global(.field.open) {
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 2px var(--accent);
  }

  .views {
    display: flex;
    gap: 2px;
    margin: 0 0 0 auto;
    padding: 4px;
    border: none;
    border-radius: 999px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
  }

  .views button {
    display: grid;
    place-items: center;
    width: 40px;
    height: 34px;
    padding: 0;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: var(--text-muted);
    transition:
      background 150ms var(--ease),
      color 150ms var(--ease);
  }

  .views button:hover {
    color: var(--text);
  }

  .views button[aria-pressed="true"] {
    background: var(--accent);
    color: var(--accent-text);
  }

  /* Reaches into the page padding so the lifted cards' shadows are not cut off at its edge. */
  .scroll {
    flex: 1;
    min-height: 0;
    margin: -14px -24px -24px;
    overflow-y: auto;
    padding: 14px 24px 24px;
  }

  .group {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 10px 0 12px;
    padding: 0;
    border: none;
    background: none;
    color: var(--text-2);
    font-size: 14px;
    font-weight: 750;
  }

  .group:first-child {
    margin-top: 0;
  }

  .group span {
    padding: 1px 9px;
    border-radius: 999px;
    background: var(--field);
    font-size: 12px;
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
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 16px;
    container-type: inline-size;
  }

  .grid.list {
    grid-template-columns: 1fr;
    gap: 8px;
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

  .empty-actions {
    display: flex;
    justify-content: center;
    gap: 10px;
    margin-top: 14px;
  }

  a.pill-button {
    display: grid;
    place-items: center;
    color: inherit;
    text-decoration: none;
  }
</style>
