<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type CatalogHit, type ManualDownload } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { locale, t } from "$lib/i18n/index.svelte";

  let { instanceId, oninstall }: { instanceId: string; oninstall: () => void } = $props();

  const SEARCH_DELAY_MS = 300;
  const SOURCE_NAMES = { modrinth: "Modrinth", curseforge: "CurseForge" };

  let query = $state("");
  let hits = $state<CatalogHit[]>([]);
  let page = $state(0);
  let hasMore = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let manual = $state<ManualDownload[]>([]);
  let installing = $state<Record<string, "busy" | "done">>({});

  $effect(() => {
    const text = query;
    const timer = setTimeout(() => void search(text, 0), SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  });

  function key(hit: CatalogHit): string {
    return `${hit.source}:${hit.id}`;
  }

  async function search(text: string, next: number) {
    loading = true;
    error = null;
    try {
      const results = await api.searchMods(instanceId, text, next);
      hits = next === 0 ? results.hits : [...hits, ...results.hits];
      hasMore = results.hasMore;
      page = next;
    } catch (err) {
      error = describeError(err);
    } finally {
      loading = false;
    }
  }

  async function install(hit: CatalogHit) {
    installing[key(hit)] = "busy";
    error = null;
    try {
      const result = await api.installMod(instanceId, hit.source, hit.id);
      manual = result.manual;
      installing[key(hit)] = "done";
      oninstall();
    } catch (err) {
      delete installing[key(hit)];
      error = describeError(err);
    }
  }

  function compact(count: number): string {
    return new Intl.NumberFormat(locale(), { notation: "compact" }).format(count);
  }
</script>

<input class="search" type="search" bind:value={query} placeholder={t("instance.search")}>

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
        <span class="title">{hit.title} <span class="muted">· {hit.author}</span></span>
        <span class="description">{hit.description}</span>
        <span class="muted">
          {t("instance.downloads", { count: compact(hit.downloads) })}
          ·
          <button type="button" class="link source" onclick={() => openUrl(hit.pageUrl)}>
            {SOURCE_NAMES[hit.source]}
          </button>
        </span>
      </div>
      <button
        type="button"
        class="add"
        class:done={installing[key(hit)] === "done"}
        disabled={installing[key(hit)] !== undefined}
        onclick={() => install(hit)}
      >
        {#if installing[key(hit)] === "busy"}
          {t("instance.adding")}
        {:else if installing[key(hit)] === "done"}
          {t("instance.added")}
        {:else}
          {t("instance.add")}
        {/if}
      </button>
    </li>
  {/each}
</ul>

{#if hasMore}
  <button type="button" class="more" disabled={loading} onclick={() => search(query, page + 1)}>
    {t("instance.more")}
  </button>
{/if}

<style>
  .search {
    width: 100%;
    height: 38px;
    margin-bottom: 12px;
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
