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
  import InstallPreviewDialog from "./InstallPreviewDialog.svelte";
  import ProjectDialog from "./ProjectDialog.svelte";
  import Segmented from "./Segmented.svelte";
  import Select from "./Select.svelte";

  let {
    instanceId,
    kinds,
    oninstall,
  }: { instanceId: string; kinds: ContentKind[]; oninstall: () => void } = $props();

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
  let chosenKind = $state<ContentKind>("mod");
  let category = $state("");
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
  let installed = $state(new Set<string>());
  let selecting = $state(false);
  let selected = $state<Record<string, CatalogHit>>({});
  let pending = $state<CatalogHit[]>([]);
  let pendingKind: ContentKind = "mod";
  let preview = $state<InstallPreview | null>(null);
  let confirming = $state(false);

  const kind = $derived(kinds.includes(chosenKind) ? chosenKind : kinds[0]);
  const selectedCount = $derived(Object.keys(selected).length);
  const kindOptions = $derived(
    kinds.map((value) => ({ value, label: t(`content.kinds.${value}`) })),
  );
  const sortOptions = $derived(
    SORTS.map((value) => ({ value, label: t(`content.sorts.${value}`) })),
  );
  const categoryOptions = $derived([
    { value: "", label: t("content.allCategories") },
    ...categories.map((tag) => ({ value: tag.id, label: categoryName(tag.name) })),
  ]);

  $effect(() => {
    void source;
    void kind;
    selected = {};
    category = "";
    categories = [];
    api
      .catalogCategories(kind, source)
      .then((tags) => {
        categories = tags;
      })
      .catch(() => {});
  });

  $effect(() => {
    const request = { text: query, from: source, kind, category, sort };
    const timer = setTimeout(() => void search(request, 0), SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    void loadInstalled(instanceId, kind);
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

  async function loadInstalled(id: string, current: ContentKind) {
    try {
      installed = new Set(await api.installedProjects(id, current));
      if (current === "shaderPack") {
        hasIris = (await api.installedProjects(id, "mod")).includes(`modrinth:${IRIS.id}`);
      }
    } catch {
      // Without it every entry just offers to install.
    }
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
    category: string;
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
        request.category || null,
        request.sort,
        next,
      );
      if (request !== lastRequest) {
        return;
      }
      hits = next === 0 ? results.hits : [...hits, ...results.hits];
      hasMore = results.hasMore;
      page = next;
    } catch (err) {
      error = describeError(err);
    } finally {
      loading = false;
    }
  }

  function toggleSelected(hit: CatalogHit) {
    if (key(hit) in selected) {
      delete selected[key(hit)];
    } else {
      selected[key(hit)] = hit;
    }
  }

  function toggleSelecting() {
    if (selecting) {
      stopSelecting();
    } else {
      selecting = true;
    }
  }

  function stopSelecting() {
    selecting = false;
    selected = {};
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
      const plan = await api.previewInstall(instanceId, target, items(picked));
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
      const result = await api.installContent(instanceId, pendingKind, items(picked));
      manual = result.manual;
      setStatus(picked, "done");
      stopSelecting();
      oninstall();
      void loadInstalled(instanceId, kind);
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
  <button
    type="button"
    class="select-mode"
    class:on={selecting}
    aria-pressed={selecting}
    onclick={toggleSelecting}
  >
    {selecting ? t("catalog.done") : t("catalog.select")}
  </button>
</div>

<div class="filters">
  <div class="filter">
    <Select bind:value={category} options={categoryOptions} />
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
      onclick={() => requestInstall([IRIS], "mod").then(() => loadInstalled(instanceId, kind))}
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
    <li class:picked={key(hit) in selected}>
      {#if selecting}
        <label class="pick">
          <input
            type="checkbox"
            checked={key(hit) in selected}
            disabled={status(hit) !== undefined}
            onchange={() => toggleSelected(hit)}
            aria-label={hit.title}
          >
          <span aria-hidden="true"></span>
        </label>
      {/if}
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
        disabled={status(hit) !== undefined}
        onclick={() => requestInstall([hit])}
      >
        {#if status(hit) === "busy"}
          {t("instance.adding")}
        {:else if status(hit) === "done"}
          {t("instance.added")}
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

{#if selecting && selectedCount > 0}
  <div class="selection-bar">
    <strong>{t("catalog.selected", { count: selectedCount })}</strong>
    <button type="button" class="clear" onclick={() => (selected = {})}>
      {t("catalog.clear")}
    </button>
    <button type="button" class="add" onclick={() => requestInstall(Object.values(selected))}>
      {t("catalog.addSelected", { count: selectedCount })}
    </button>
  </div>
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
  .select-mode {
    flex: none;
    height: 38px;
    padding: 0 16px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    font-size: 13px;
    font-weight: 600;
  }

  .select-mode.on {
    border-color: var(--accent);
    color: var(--accent);
  }

  .pick {
    display: grid;
    flex: none;
    place-items: center;
    cursor: pointer;
  }

  .pick input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .pick span {
    width: 22px;
    height: 22px;
    border: 2px solid var(--border);
    border-radius: 50%;
    transition:
      background 150ms var(--ease),
      border-color 150ms var(--ease);
  }

  .pick input:checked + span {
    border-color: var(--accent);
    background:
      var(--accent)
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'%3E%3Cpath d='M4 8.5l3 3 5-6' fill='none' stroke='white' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E")
      center / 14px no-repeat;
  }

  .pick input:disabled + span {
    opacity: 0.35;
  }

  .pick input:focus-visible + span {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  li.picked {
    border-color: var(--accent);
  }

  .selection-bar {
    position: sticky;
    bottom: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
    padding: 10px 10px 10px 18px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface-solid);
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.25);
    animation: rise 180ms var(--ease);
  }

  .selection-bar strong {
    flex: 1;
    font-weight: 600;
  }

  .clear {
    height: 32px;
    padding: 0 14px;
    border: none;
    border-radius: 999px;
    background: var(--field);
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }

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
