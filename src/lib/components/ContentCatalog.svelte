<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    api,
    type CatalogHit,
    type CatalogItem,
    type CatalogSort,
    type CategoryTag,
    type ContentKind,
    type InstallPreview,
    type ManualDownload,
    type Source,
  } from "$lib/api";
  import { categoryName } from "$lib/categories";
  import { describeError } from "$lib/errors";
  import { locale, t } from "$lib/i18n/index.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import InstallPreviewDialog from "./InstallPreviewDialog.svelte";
  import ProjectDialog from "./ProjectDialog.svelte";
  import Segmented from "./Segmented.svelte";
  import Select from "./Select.svelte";
  import WorldPicker from "./WorldPicker.svelte";

  let {
    instanceId,
    kinds,
    kind: chosenKind = $bindable("mod"),
    world = $bindable(null),
    oninstall,
  }: {
    instanceId: string;
    kinds: ContentKind[];
    kind?: ContentKind;
    world?: string | null;
    oninstall: () => void;
  } = $props();

  const SEARCH_DELAY_MS = 300;
  const SOURCE_NAMES: Record<Source, string> = { modrinth: "Modrinth", curseforge: "CurseForge" };
  const SOURCES = (Object.keys(SOURCE_NAMES) as Source[]).map((value) => ({
    value,
    label: SOURCE_NAMES[value],
  }));
  const SORTS: CatalogSort[] = ["relevance", "downloads", "newest", "updated"];
  const IRIS: CatalogHit = {
    source: "modrinth",
    id: "YL57xq9U",
    title: "Iris Shaders",
    description: "",
    author: "",
    iconUrl: null,
    downloads: 0,
    pageUrl: "",
  };

  let source = $state<Source>(loadSource());
  let chosenCategories = $state<string[]>([]);
  let sort = $state<CatalogSort>("relevance");
  let categories = $state<CategoryTag[]>([]);
  let shownHit = $state<CatalogHit | null>(null);
  let showing = $state(false);
  let hasIris = $state(true);
  let query = $state("");
  let hits = $state<CatalogHit[]>([]);
  let page = $state(0);
  let hasMore = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let manual = $state<ManualDownload[]>([]);
  let installing = $state<Record<string, "busy" | "done">>({});
  let installed = $state(new Map<string, string>());
  let removing = $state<CatalogHit | null>(null);
  let confirmingRemove = $state(false);
  let pending = $state<CatalogHit[]>([]);
  let pendingKind: ContentKind = "mod";
  let preview = $state<InstallPreview | null>(null);
  let confirming = $state(false);

  const kind = $derived(kinds.includes(chosenKind) ? chosenKind : kinds[0]);
  const kindOptions = $derived(
    kinds.map((value) => ({ value, label: t(`content.kinds.${value}`) })),
  );
  const sortOptions = $derived(
    SORTS.map((value) => ({ value, label: t(`content.sorts.${value}`) })),
  );
  const categoryOptions = $derived(
    categories.map((tag) => ({ value: tag.id, label: categoryName(tag.name) })),
  );

  $effect(() => {
    void source;
    void kind;
    chosenCategories = [];
    categories = [];
    api
      .catalogCategories(kind, source)
      .then((tags) => {
        categories = tags;
      })
      .catch(() => {});
  });

  $effect(() => {
    const request = { text: query, from: source, kind, categories: chosenCategories, sort };
    const timer = setTimeout(() => void search(request, 0), SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    void loadInstalled(instanceId, kind, world);
  });

  $effect(() => {
    try {
      localStorage.setItem("catalog-source", source);
    } catch {
      // Not remembering the choice is fine.
    }
  });

  function loadSource(): Source {
    try {
      const saved = localStorage.getItem("catalog-source");
      if (saved === "modrinth" || saved === "curseforge") {
        return saved;
      }
    } catch {
      // Fall back to Modrinth.
    }
    return "modrinth";
  }

  async function loadInstalled(id: string, current: ContentKind, place: string | null) {
    installed = new Map();
    if (current === "dataPack" && !place) {
      return;
    }
    try {
      installed = new Map(
        Object.entries(await api.installedProjects(id, current, worldFor(current, place))),
      );
      if (current === "shaderPack") {
        hasIris = `modrinth:${IRIS.id}` in (await api.installedProjects(id, "mod", null));
      }
    } catch {
      // Without it every entry just offers to install.
    }
  }

  function worldFor(target: ContentKind, place = world): string | null {
    return target === "dataPack" ? place : null;
  }

  function askRemove(hit: CatalogHit) {
    removing = hit;
    confirmingRemove = true;
  }

  async function remove(hit: CatalogHit) {
    const fileName = installed.get(key(hit));
    if (fileName) {
      await api.removeContent(instanceId, kind, worldFor(kind), fileName);
    }
    delete installing[key(hit)];
    oninstall();
    await loadInstalled(instanceId, kind, world);
  }

  function status(hit: CatalogHit): "busy" | "done" | undefined {
    return installing[key(hit)] ?? (installed.has(key(hit)) ? "done" : undefined);
  }

  function key(hit: CatalogHit): string {
    return `${hit.source}:${hit.id}`;
  }

  interface Request {
    text: string;
    from: Source;
    kind: ContentKind;
    categories: string[];
    sort: CatalogSort;
  }

  let lastRequest: Request | null = null;

  async function search(request: Request, next: number) {
    lastRequest = request;
    loading = true;
    error = null;
    try {
      const results = await api.searchCatalog(
        instanceId,
        request.kind,
        request.from,
        request.text,
        $state.snapshot(request.categories),
        request.sort,
        next,
      );
      if (request !== lastRequest) {
        return;
      }
      if (next === 0) {
        hits = results.hits;
      } else {
        // A project published between pages shifts the rest down by one.
        const shown = new Set(hits.map(key));
        hits = [...hits, ...results.hits.filter((hit) => !shown.has(key(hit)))];
      }
      hasMore = results.hasMore;
      page = next;
    } catch (err) {
      if (request === lastRequest) {
        error = describeError(err);
      }
    } finally {
      if (request === lastRequest) {
        loading = false;
      }
    }
  }

  // Shows what will be downloaded when dependencies or manual downloads come along,
  // otherwise installs straight away.
  async function requestInstall(picked: CatalogHit[], target = kind) {
    if (picked.length === 0) {
      return;
    }
    pending = picked;
    pendingKind = target;
    setStatus(picked, "busy");
    error = null;
    try {
      const plan = await api.previewInstall(instanceId, target, worldFor(target), items(picked));
      const onlyPicked = plan.mods.every((planned) => planned.requested);
      if (onlyPicked && plan.manual.length === 0) {
        await install();
      } else {
        preview = plan;
        confirming = true;
      }
    } catch (err) {
      setStatus(picked, undefined);
      error = describeError(err);
    }
  }

  async function install() {
    const picked = pending;
    setStatus(picked, "busy");
    try {
      const result = await api.installContent(
        instanceId,
        pendingKind,
        worldFor(pendingKind),
        items(picked),
      );
      manual = result.manual;
      setStatus(picked, "done");
      oninstall();
      void loadInstalled(instanceId, kind, world);
    } catch (err) {
      setStatus(picked, undefined);
      error = describeError(err);
    }
  }

  function cancelInstall() {
    setStatus(pending, undefined);
    pending = [];
  }

  function items(picked: CatalogHit[]): CatalogItem[] {
    return picked.map((hit) => ({ source: hit.source, id: hit.id }));
  }

  function setStatus(picked: CatalogHit[], value: "busy" | "done" | undefined) {
    for (const hit of picked) {
      if (value) {
        installing[key(hit)] = value;
      } else {
        delete installing[key(hit)];
      }
    }
  }

  function showProject(hit: CatalogHit) {
    shownHit = hit;
    showing = true;
  }

  function compact(count: number): string {
    return new Intl.NumberFormat(locale(), { notation: "compact" }).format(count);
  }
</script>

{#if kinds.length > 1}
  <div class="kinds">
    <Segmented
      bind:value={chosenKind}
      options={kindOptions}
      name="catalog-kind"
      label={t("instance.catalog")}
    />
  </div>
{/if}

{#if kind === "dataPack"}
  <WorldPicker {instanceId} bind:value={world} />
{/if}

<div class="search-bar">
  <input
    class="search"
    type="search"
    data-search
    bind:value={query}
    placeholder={t("catalog.search")}
  >
  <Segmented
    bind:value={source}
    options={SOURCES}
    name="catalog-source"
    label={t("instance.source")}
  />
</div>

<div class="filters">
  <div class="filter">
    <Select
      multiple
      bind:value={chosenCategories}
      options={categoryOptions}
      placeholder={t("content.allCategories")}
    />
  </div>
  <div class="filter">
    <Select bind:value={sort} options={sortOptions} />
  </div>
</div>

{#if kind === "shaderPack" && !hasIris}
  <div class="manual iris" role="alert">
    <p>{t("content.needIris")}</p>
    <button
      type="button"
      class="add"
      disabled={status(IRIS) !== undefined}
      onclick={() =>
        requestInstall([IRIS], "mod").then(() => loadInstalled(instanceId, kind, world))}
    >
      {t("content.addIris")}
    </button>
  </div>
{/if}

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

{#if manual.length > 0}
  <div class="manual" role="alert">
    <p>{t("instance.manualDownload")}</p>
    {#each manual as download (download.url)}
      <button type="button" class="link" onclick={() => openUrl(download.url)}>
        {download.title}
      </button>
    {/each}
  </div>
{/if}

{#if !loading && hits.length === 0 && !error}
  <p class="muted center">{t("instance.nothingFound")}</p>
{/if}

<ul>
  {#each hits as hit (key(hit))}
    <li>
      {#if hit.iconUrl}
        <img src={hit.iconUrl} alt="" loading="lazy">
      {:else}
        <span class="icon-placeholder" aria-hidden="true"></span>
      {/if}
      <div class="info">
        <span class="title">
          <button type="button" class="link name" onclick={() => showProject(hit)}>
            {hit.title}
          </button>
          <span class="muted">· {hit.author}</span>
        </span>
        <span class="description">{hit.description}</span>
        <span class="muted">
          {t("instance.downloads", { count: compact(hit.downloads) })}
          ·
          <button type="button" class="link source" onclick={() => openUrl(hit.pageUrl)}>
            {t("instance.openPage")}
          </button>
        </span>
      </div>
      <button
        type="button"
        class="add"
        class:done={status(hit) === "done"}
        disabled={status(hit) === "busy" || (kind === "dataPack" && !world)}
        onclick={() => (status(hit) === "done" ? askRemove(hit) : requestInstall([hit]))}
      >
        {#if status(hit) === "busy"}
          {t("instance.adding")}
        {:else if status(hit) === "done"}
          <span class="installed-label">{t("instance.added")}</span>
          <span class="remove-label">{t("catalog.remove")}</span>
        {:else}
          {t("instance.add")}
        {/if}
      </button>
    </li>
  {/each}
</ul>

{#if hasMore}
  <button
    type="button"
    class="more"
    disabled={loading}
    onclick={() => lastRequest && search(lastRequest, page + 1)}
  >
    {t("instance.more")}
  </button>
{/if}

{#if removing}
  {@const hit = removing}
  <ConfirmDialog
    bind:open={confirmingRemove}
    title={t("catalog.removeTitle", { name: hit.title })}
    message={t(`catalog.removeMessage.${kind}`)}
    confirmLabel={t("catalog.remove")}
    onconfirm={() => remove(hit)}
  />
{/if}

{#if shownHit}
  <ProjectDialog
    bind:open={showing}
    {instanceId}
    {kind}
    project={{ source: shownHit.source, id: shownHit.id }}
  />
{/if}

{#if preview}
  <InstallPreviewDialog
    {preview}
    bind:open={confirming}
    onconfirm={install}
    oncancel={cancelInstall}
  />
{/if}

<style>
  .search-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }

  .kinds {
    display: flex;
    margin-bottom: 10px;
  }

  .filters {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
  }

  .filter {
    width: 220px;
  }

  .iris {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .iris p {
    flex: 1;
    margin: 0;
  }

  .link.name {
    color: var(--text);
    font-weight: 600;
  }

  .link.name:hover {
    color: var(--accent);
    text-decoration: none;
  }

  .search {
    flex: 1;
    min-width: 0;
    height: 38px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  ul {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  img,
  .icon-placeholder {
    flex: none;
    width: 48px;
    height: 48px;
    border-radius: 12px;
    background: var(--field);
  }

  .info {
    display: grid;
    flex: 1;
    gap: 2px;
    min-width: 0;
  }

  .title {
    font-weight: 600;
  }

  .description {
    display: -webkit-box;
    overflow: hidden;
    font-size: 13px;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }

  .muted {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 400;
  }

  .center {
    padding: 32px 0;
    text-align: center;
  }

  .add {
    flex: none;
    min-width: 96px;
    height: 32px;
    padding: 0 14px;
    border: none;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }

  .add:disabled {
    opacity: 0.7;
  }

  .add.done {
    background: var(--field);
    color: var(--text);
    opacity: 1;
    transition:
      background 150ms var(--ease),
      color 150ms var(--ease);
  }

  .add.done:hover {
    background: color-mix(in srgb, var(--danger) 16%, transparent);
    color: var(--danger);
  }

  .remove-label,
  .add.done:hover .installed-label {
    display: none;
  }

  .add.done:hover .remove-label {
    display: inline;
  }

  .more {
    display: block;
    margin: 12px auto 0;
    height: 32px;
    padding: 0 16px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
  }

  .error {
    color: var(--danger);
  }

  .manual {
    margin-bottom: 12px;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    background: var(--field);
    font-size: 13px;
  }

  .manual p {
    margin: 0 0 4px;
  }

  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
  }

  .link:hover {
    text-decoration: underline;
  }

  .source {
    font-size: 12px;
  }
</style>
