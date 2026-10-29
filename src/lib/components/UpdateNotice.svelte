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
    recheck,
    checking = $bindable(false),
    onupdated,
  }: {
    instance: Instance;
    disabled: boolean;
    /** Bumped by the refresh button: checks again and shows the notice even if dismissed. */
    recheck: number;
    checking?: boolean;
    onupdated: () => void;
  } = $props();

  let loaderTarget = $state<string | null>(null);
  let counts = $state<Partial<Record<ContentKind, number>>>({});
  let skipped = $state<Record<string, boolean>>({});
  let working = $state(false);
  let error = $state<string | null>(null);
  // Installing from the catalog adds the newest builds, so only an update here calls for a
  // new check.
  let updated = $state(0);

  // Saving any setting replaces the instance object; only these fields call for a new check.
  const id = $derived(instance.id);
  const version = $derived(instance.version);
  const loaderId = $derived(instance.loader?.loader ?? null);
  const loaderVersion = $derived(instance.loader?.version ?? null);
  const kinds = $derived<ContentKind[]>(
    loaderId ? ["mod", "resourcePack", "shaderPack"] : ["resourcePack"],
  );
  const pending = $derived(kinds.filter((kind) => (counts[kind] ?? 0) > 0));
  const chosen = $derived(
    (loaderTarget !== null && !skipped.loader) || pending.some((kind) => !skipped[kind]),
  );
  const found = $derived(loaderTarget !== null || pending.length > 0);
  const visible = $derived(
    !dismissed[instance.id] && (found || error !== null || (recheck > 0 && !checking)),
  );

  $effect(() => {
    const loader = loaderId && loaderVersion ? { loader: loaderId, version: loaderVersion } : null;
    const checked = kinds;
    void updated;
    if (recheck > 0) {
      delete dismissed[id];
    }
    let stale = false;
    checking = true;
    loaderTarget = null;
    counts = {};
    skipped = {};
    error = null;

    const checks: Promise<void>[] = [];
    if (loader) {
      const check = api
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
      checks.push(check);
    }
    for (const kind of checked) {
      const check = api
        .checkUpdates(id, kind, null)
        .then((updates) => {
          if (!stale) {
            counts[kind] = updates.length;
          }
        })
        .catch(() => {});
      checks.push(check);
    }
    void Promise.all(checks).then(() => {
      if (!stale) {
        checking = false;
      }
    });
    return () => {
      stale = true;
    };
  });

  async function update() {
    working = true;
    error = null;
    try {
      if (instance.loader && loaderTarget && !skipped.loader) {
        replaceInstance(await api.changeLoaderBuild(instance.id, loaderTarget));
      }
      for (const kind of pending.filter((kind) => !skipped[kind])) {
        await api.updateContent(instance.id, kind, null);
      }
      updated++;
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
      <strong>{found ? t("updates.title") : t("updates.none")}</strong>
      {#if instance.loader && loaderTarget}
        <label class="toggle">
          <input
            type="checkbox"
            bind:checked={() => !skipped.loader, (checked) => (skipped.loader = !checked)}
          >
          <span>
            {t("updates.loader", {
              loader: LOADER_NAMES[instance.loader.loader],
              from: instance.loader.version,
              to: loaderTarget,
            })}
          </span>
        </label>
      {/if}
      {#each pending as kind (kind)}
        <label class="toggle">
          <input
            type="checkbox"
            bind:checked={() => !skipped[kind], (checked) => (skipped[kind] = !checked)}
          >
          <span>{t(`updates.kinds.${kind}`, { count: counts[kind] ?? 0 })}</span>
        </label>
      {/each}
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
    </div>
    <div class="actions">
      {#if found}
        <button type="button" disabled={disabled || working || !chosen} onclick={update}>
          {working ? t("updates.working") : t("updates.update")}
        </button>
      {/if}
      <button type="button" class="ghost" onclick={() => (dismissed[instance.id] = true)}>
        {found ? t("updates.later") : t("account.dismiss")}
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
    gap: 6px;
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
