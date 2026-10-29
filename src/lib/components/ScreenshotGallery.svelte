<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { api, type Screenshot } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { relativeTime } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import { UI_ICONS } from "$lib/icons";
  import { library } from "$lib/state/instances.svelte";
  import { tooltip } from "$lib/tooltip";

  let { instanceId }: { instanceId: string } = $props();

  let shots = $state<Screenshot[] | null>(null);
  let error = $state<string | null>(null);
  let copied = $state<string | null>(null);
  const status = $derived(library.launches[instanceId]?.status);

  $effect(() => {
    void instanceId;
    void status;
    void load();
  });

  async function load() {
    try {
      shots = await api.screenshots(instanceId);
    } catch (err) {
      error = describeError(err);
    }
  }

  async function copy(shot: Screenshot) {
    error = null;
    try {
      const image = await fetch(convertFileSrc(shot.path)).then((response) => response.blob());
      await navigator.clipboard.write([new ClipboardItem({ "image/png": image })]);
      copied = shot.name;
      setTimeout(() => (copied = null), 1500);
    } catch (err) {
      error = describeError(err);
    }
  }

  async function remove(shot: Screenshot) {
    await api.deleteScreenshot(instanceId, shot.name);
    shots = shots?.filter((other) => other.name !== shot.name) ?? null;
  }
</script>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

{#if shots && shots.length === 0}
  <div class="empty-state">
    <h3>{t("screenshots.empty")}</h3>
    <p>{t("screenshots.emptyHint")}</p>
  </div>
{:else if shots}
  <ul class="gallery">
    {#each shots as shot (shot.name)}
      <li>
        <button
          type="button"
          class="preview"
          {@attach tooltip(t("screenshots.open"))}
          onclick={() => api.openScreenshot(instanceId, shot.name)}
        >
          <img src={convertFileSrc(shot.path)} alt={shot.name} loading="lazy" decoding="async">
        </button>
        <div class="bar">
          <span>{shot.takenAt ? relativeTime(shot.takenAt) : shot.name}</span>
          <button
            type="button"
            class="icon-button"
            {@attach tooltip(
              copied === shot.name ? t("screenshots.copied") : t("screenshots.copy"),
            )}
            aria-label={t("screenshots.copy")}
            onclick={() => copy(shot)}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.copy} /></svg>
          </button>
          <button
            type="button"
            class="icon-button danger"
            {@attach tooltip(t("screenshots.delete"))}
            aria-label={t("screenshots.delete")}
            onclick={() => remove(shot)}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.delete} /></svg>
          </button>
        </div>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .gallery {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .preview {
    display: block;
    width: 100%;
    padding: 0;
    border: none;
    background: var(--field);
    cursor: zoom-in;
  }

  img {
    display: block;
    width: 100%;
    aspect-ratio: 16 / 9;
    object-fit: cover;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px 6px 4px 12px;
  }

  .bar span {
    flex: 1;
    overflow: hidden;
    color: var(--text-muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .error {
    color: var(--danger);
  }
</style>
