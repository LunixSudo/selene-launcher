<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type CatalogHit, type CatalogSort, type Instance, type Source } from "#lib/api.ts";
  import BackLink from "#lib/components/BackLink.svelte";
  import ModpackDialog from "#lib/components/ModpackDialog.svelte";
  import Segmented from "#lib/components/Segmented.svelte";
  import Select from "#lib/components/Select.svelte";
  import { describeError } from "#lib/errors.ts";
  import { locale, t } from "#lib/i18n/index.svelte.ts";
  import { nearEnd } from "#lib/near-end.ts";
  import { scrollFade } from "#lib/scroll-fade.ts";
  import { library } from "#lib/state/instances.svelte.ts";
  import { goto } from "$app/navigation";

  const SEARCH_DELAY_MS = 300;
  const SOURCES: { value: Source; label: string }[] = [
    { value: "modrinth", label: "Modrinth" },
    { value: "curseforge", label: "CurseForge" },
  ];
  const SORTS: CatalogSort[] = ["relevance", "downloads", "newest", "updated"];

  interface Request {
    text: string;
    from: Source;
    sort: CatalogSort;
    gameVersion: string;
  }

  let source = $state<Source>("modrinth");
  let sort = $state<CatalogSort>("downloads");
  let query = $state("");
  let gameVersion = $state("");
  let releases = $state<string[]>([]);
  let hits = $state<CatalogHit[]>([]);
  let page = $state(0);
  let hasMore = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let installing = $state<Record<string, true>>({});
  let dialogOpen = $state(false);
  let chosen = $state<{ hit: CatalogHit; instances: Instance[] } | null>(null);
  let lastRequest: Request | null = null;

  const sortOptions = $derived(
    SORTS.map((value) => ({ value, label: t(`content.sorts.${value}`) })),
  );
  const versionOptions = $derived([
    { value: "", label: t("modpacks.anyVersion") },
    ...releases.map((value) => ({ value, label: value })),
  ]);

  $effect(() => {
    api
      .versions()
      .then((all) => {
        releases = all.filter((entry) => entry.kind === "release").map((entry) => entry.id);
      })
      .catch(() => {});
  });

  $effect(() => {
    const request = { text: query, from: source, sort, gameVersion };
    const timer = setTimeout(() => void search(request, 0), SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  });

  async function search(request: Request, next: number) {
    lastRequest = request;
    loading = true;
    error = null;
    try {
      const results = await api.searchModpacks(
        request.from,
        request.text,
        request.gameVersion || null,
        request.sort,
        next,
      );
      if (request !== lastRequest) {
        return;
      }
      const shown = new Set(next === 0 ? [] : hits.map(key));
      hits = [...(next === 0 ? [] : hits), ...results.hits.filter((hit) => !shown.has(key(hit)))];
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

  function loadMore() {
    if (!loading && hasMore && lastRequest) {
      void search(lastRequest, page + 1);
    }
  }

  function installedOf(hit: CatalogHit): Instance[] {
    return library.instances.filter(
      (instance) => instance.pack?.source === hit.source && instance.pack.projectId === hit.id,
    );
  }

  /** The newest release of each installed pack, to tell whether an update is there at all. */
  const newest = $state<Record<string, string | null>>({});
  const asked = new Set<string>();

  $effect(() => {
    for (const hit of hits) {
      const id = key(hit);
      if (asked.has(id) || installedOf(hit).length !== 1) {
        continue;
      }
      asked.add(id);
      api
        .modpackVersions(hit.source, hit.id)
        .then((versions) => {
          newest[id] = (versions.find((version) => version.release) ?? versions[0])?.id ?? null;
        })
        .catch(() => {});
    }
  });

  function upToDate(hit: CatalogHit, instances: Instance[]): boolean {
    const latest = newest[key(hit)];
    return instances.length === 1 && latest != null && latest === instances[0].pack?.versionId;
  }

  function choose(hit: CatalogHit, instances: Instance[]) {
    chosen = { hit, instances };
    dialogOpen = true;
  }

  async function trackInstall(hit: CatalogHit, done: Promise<Instance | null>) {
    installing[key(hit)] = true;
    error = null;
    const instance = await done;
    delete installing[key(hit)];
    if (!instance && library.importError) {
      error = t(`errors.${library.importError.kind}`);
    }
  }

  function key(hit: CatalogHit): string {
    return `${hit.source}:${hit.id}`;
  }

  function compact(count: number): string {
    return new Intl.NumberFormat(locale(), { notation: "compact" }).format(count);
  }
</script>

<section class="page">
  <header>
    <BackLink />
    <h1 class="page-title">{t("modpacks.title")}</h1>
  </header>

  <div class="filters">
    <input
      class="search"
      type="search"
      data-search
      bind:value={query}
      placeholder={t("modpacks.search")}
      aria-label={t("modpacks.search")}
    >
    <Segmented
      bind:value={source}
      options={SOURCES}
      name="modpack-source"
      label={t("instance.source")}
    />
    <div class="select">
      <Select bind:value={gameVersion} options={versionOptions} />
    </div>
    <div class="select">
      <Select bind:value={sort} options={sortOptions} />
    </div>
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <div class="scroll" {@attach scrollFade}>
    {#if !loading && hits.length === 0 && !error}
      <p class="muted center">{t("instance.nothingFound")}</p>
    {/if}

    <ul>
      {#each hits as hit (key(hit))}
        {@const instances = installedOf(hit)}
        <li>
          {#if hit.iconUrl}
            <img src={hit.iconUrl} alt="" loading="lazy">
          {:else}
            <span class="icon" aria-hidden="true"></span>
          {/if}
          <div class="info">
            <span class="title">
              {hit.title}
              <span class="muted">· {hit.author}</span>
            </span>
            <span class="description">{hit.description}</span>
            <span class="muted">
              {t("instance.downloads", { count: compact(hit.downloads) })}
              ·
              <button type="button" class="link" onclick={() => openUrl(hit.pageUrl)}>
                {t("instance.openPage")}
              </button>
            </span>
          </div>
          <div class="actions">
            {#if installing[key(hit)]}
              <button type="button" class="install" disabled>{t("modpacks.installing")}</button>
            {:else if instances.length > 0}
              <button
                type="button"
                class="install done"
                title={instances.map((instance) => instance.name).join(", ")}
                onclick={() =>
                  instances.length === 1 ? goto(`/instance/${instances[0].id}`) : goto("/")}
              >
                {instances.length === 1
                  ? t("modpacks.installed")
                  : t("modpacks.installedCount", { count: instances.length })}
              </button>
              {#if !upToDate(hit, instances)}
                <button type="button" class="install ghost" onclick={() => choose(hit, instances)}>
                  {t("modpacks.update")}
                </button>
              {/if}
              <button type="button" class="install ghost" onclick={() => choose(hit, [])}>
                {t("modpacks.add")}
              </button>
            {:else}
              <button type="button" class="install" onclick={() => choose(hit, [])}>
                {t("modpacks.install")}
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>

    {#if hasMore}
      {#key hits.length}
        <div class="more" {@attach nearEnd(loadMore)}>
          {#if loading}
            {t("instance.loadingMore")}
          {/if}
        </div>
      {/key}
    {/if}
  </div>
</section>

{#if chosen}
  {@const hit = chosen.hit}
  <ModpackDialog
    bind:open={dialogOpen}
    source={hit.source}
    projectId={hit.id}
    title={hit.title}
    instances={chosen.instances}
    {gameVersion}
    oninstall={(done) => trackInstall(hit, done)}
  />
{/if}

<style>
  header {
    display: grid;
    gap: 4px;
    padding-top: 8px;
  }

  .filters {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .search {
    flex: 1;
    min-width: 0;
    max-width: 420px;
    height: 36px;
    padding: 0 16px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  .select {
    width: 180px;
  }

  .actions {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: 6px;
  }

  .scroll {
    flex: 1;
    min-height: 0;
    margin: 0 -24px -24px;
    overflow-y: auto;
    padding: 0 24px 24px;
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
    gap: 14px;
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    backdrop-filter: blur(24px);
  }

  img,
  .icon {
    flex: none;
    width: 56px;
    height: 56px;
    border-radius: 14px;
    background: var(--field);
    object-fit: cover;
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

  .install {
    flex: none;
    min-width: 110px;
    height: 32px;
    padding: 0 14px;
    border: none;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }

  .install:disabled {
    opacity: 0.7;
  }

  .install.done {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent);
  }

  .install.ghost {
    background: var(--field);
    color: var(--text);
    font-weight: 500;
  }

  .more {
    min-height: 32px;
    padding-top: 12px;
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }

  .error {
    margin: 0;
    color: var(--danger);
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
</style>
