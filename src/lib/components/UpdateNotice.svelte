<script lang="ts" module>
  // Hidden until the launcher restarts, so the notice does not come back on every visit.
  const dismissed = $state<Record<string, boolean>>({});
</script>

<script lang="ts">
  import { api, type ContentKind, type Instance } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { t } from "$lib/i18n/index.svelte";
  import { LOADER_NAMES, preferredRelease } from "$lib/loaders";
  import { replaceInstance } from "$lib/state/instances.svelte";

  let {
    instance,
    disabled,
    refresh,
    onupdated,
  }: { instance: Instance; disabled: boolean; refresh: number; onupdated: () => void } = $props();

  let loaderTarget = $state<string | null>(null);
  let counts = $state<Partial<Record<ContentKind, number>>>({});
  let working = $state(false);
  let error = $state<string | null>(null);

  const kinds = $derived<ContentKind[]>(
    instance.loader ? ["mod", "resourcePack", "shaderPack"] : ["resourcePack"],
  );
  const parts = $derived(
    kinds
      .filter((kind) => (counts[kind] ?? 0) > 0)
      .map((kind) => `${t(`content.kinds.${kind}`).toLocaleLowerCase()}: ${counts[kind]}`),
  );
  const visible = $derived(
    !dismissed[instance.id] && (loaderTarget !== null || parts.length > 0 || error !== null),
  );

  $effect(() => {
    const { id, version, loader } = instance;
    const checked = kinds;
    void refresh;
    let stale = false;
    loaderTarget = null;
    counts = {};
    error = null;

    if (loader) {
      api
        .loaderVersions(loader.loader, version)
        .then((releases) => {
          const preferred = preferredRelease(releases);
          const current = releases.findIndex((release) => release.version === loader.version);
          // Newest first, so an earlier index is a newer build.
          if (!stale && preferred && current > releases.indexOf(preferred)) {
            loaderTarget = preferred.version;
          }
        })
        .catch(() => {});
    }
    for (const kind of checked) {
      api
        .checkUpdates(id, kind, null)
        .then((updates) => {
          if (!stale) {
            counts[kind] = updates.length;
          }
        })
        .catch(() => {});
    }
    return () => {
      stale = true;
    };
  });

  async function update() {
    working = true;
    error = null;
    try {
      if (instance.loader && loaderTarget) {
        const loader = { loader: instance.loader.loader, version: loaderTarget };
        replaceInstance(await api.changeVersion(instance.id, instance.version, loader));
      }
      for (const kind of kinds) {
        await api.updateContent(instance.id, kind, null);
      }
      onupdated();
    } catch (err) {
      error = describeError(err);
    } finally {
      working = false;
    }
  }
</script>

{#if visible}
  <div class="notice" role="status">
    <div class="text">
      <strong>{t("updates.title")}</strong>
      {#if instance.loader && loaderTarget}
        <p>
          {t("updates.loader", {
            loader: LOADER_NAMES[instance.loader.loader],
            from: instance.loader.version,
            to: loaderTarget,
          })}
        </p>
      {/if}
      {#if parts.length > 0}
        <p>{t("updates.content", { list: parts.join(", ") })}</p>
      {/if}
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
    </div>
    <div class="actions">
      <button type="button" disabled={disabled || working} onclick={update}>
        {working ? t("updates.working") : t("updates.updateAll")}
      </button>
      <button type="button" class="ghost" onclick={() => (dismissed[instance.id] = true)}>
        {t("updates.later")}
      </button>
    </div>
  </div>
{/if}

<style>
  .notice {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 16px;
    border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--accent) 14%, var(--surface));
  }

  .text {
    display: grid;
    flex: 1;
    gap: 4px;
    min-width: 0;
  }

  p {
    margin: 0;
    font-size: 13px;
  }

  .error {
    color: var(--danger);
  }

  .actions {
    display: flex;
    flex: none;
    gap: 6px;
  }

  button {
    height: 30px;
    padding: 0 12px;
    border: none;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-text);
    font-size: 12px;
    font-weight: 600;
  }

  button:disabled {
    opacity: 0.6;
  }

  .ghost {
    background: var(--field);
    color: var(--text);
  }
</style>
