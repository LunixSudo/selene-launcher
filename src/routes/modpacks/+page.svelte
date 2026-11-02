<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type CatalogHit, type CatalogSort, type Source } from "#lib/api.ts";
  import BackLink from "#lib/components/BackLink.svelte";
  import Segmented from "#lib/components/Segmented.svelte";
  import Select from "#lib/components/Select.svelte";
  import { describeError } from "#lib/errors.ts";
  import { locale, t } from "#lib/i18n/index.svelte.ts";
  import { scrollFade } from "#lib/scroll-fade.ts";
  import { installModpack, library } from "#lib/state/instances.svelte.ts";
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
  }

  let source = $state<Source>("modrinth");
  let sort = $state<CatalogSort>("downloads");
  let query = $state("");
  let hits = $state<CatalogHit[]>([]);
  let page = $state(0);
  let hasMore = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let installing = $state<Record<string, true>>({});
  let installed = $state<Record<string, string>>({});
  let lastRequest: Request | null = null;

  const sortOptions = $derived(
    SORTS.map((value) => ({ value, label: t(`content.sorts.${value}`) })),
  );

  $effect(() => {
    const request = { text: query, from: source, sort };
    const timer = setTimeout(() => void search(request, 0), SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  });

  async function search(request: Request, next: number) {
    lastRequest = request;
    loading = true;
    error = null;
    try {
      const results = await api.searchModpacks(request.from, request.text, request.sort, next);
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

  async function install(hit: CatalogHit) {
    installing[key(hit)] = true;
    error = null;
    const instance = await installModpack(hit.source, hit.id);
    delete installing[key(hit)];
    if (instance) {
      installed[key(hit)] = instance.id;
    } else if (library.importError) {
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
    <div class="sort">
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
        {@const instanceId = installed[key(hit)]}
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
          {#if instanceId}
            <button
              type="button"
              class="install done"
              onclick={() => goto(`/instance/${instanceId}`)}
            >
              {t("instances.open")}
            </button>
          {:else}
            <button
              type="button"
              class="install"
              disabled={installing[key(hit)]}
              onclick={() => install(hit)}
            >
              {installing[key(hit)] ? t("modpacks.installing") : t("modpacks.install")}
            </button>
          {/if}
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
  </div>
</section>

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

  .sort {
    width: 200px;
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
    background: var(--field);
    color: var(--text);
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
