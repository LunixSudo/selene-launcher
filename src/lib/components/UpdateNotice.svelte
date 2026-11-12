<script lang="ts" module>
  // Hidden until the launcher restarts, so the notice does not come back on every visit.
  const dismissed = $state<Record<string, boolean>>({});
</script>

<script lang="ts">
  import { api, type ContentKind, type Instance, type PackVersion } from "#lib/api.ts";
  import { describeError } from "#lib/errors.ts";
  import { t } from "#lib/i18n/index.svelte.ts";
  import { LOADER_NAMES, preferredRelease } from "#lib/loaders.ts";
  import { isPreparing, prepare, replaceInstance } from "#lib/state/instances.svelte.ts";

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
  let packTarget = $state<PackVersion | null>(null);
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
  const pack = $derived(instance.pack);
  const packSource = $derived(pack?.source ?? null);
  const packProject = $derived(pack?.projectId ?? null);
  const packVersion = $derived(pack?.versionId ?? null);
  const chosen = $derived(
    (packTarget !== null && !skipped.pack) ||
      (loaderTarget !== null && !skipped.loader) ||
      pending.some((kind) => !skipped[kind]),
  );
  const found = $derived(packTarget !== null || loaderTarget !== null || pending.length > 0);
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
    packTarget = null;
    counts = {};
    // Moving to another version of the pack can replace mods, so it waits to be asked for.
    skipped = { pack: true };
    error = null;

    const checks: Promise<void>[] = [];
    if (packSource && packProject) {
      const current = packVersion;
      const check = api
        .modpackVersions(packSource, packProject)
        .then((versions) => {
          const newest = versions.find((version) => version.release) ?? versions[0];
          if (!stale && newest && newest.id !== current) {
            packTarget = newest;
          }
        })
        .catch(() => {});
      checks.push(check);
    }
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
      if (packTarget && !skipped.pack) {
        const result = await api.updateModpack(instance.id, packTarget.id);
        replaceInstance(result.instance);
        prepare(instance.id);
      }
      if (instance.loader && loaderTarget && !skipped.loader) {
        // Changing the build stops the game download, which then goes on with the new one.
        const downloading = isPreparing(instance.id);
        replaceInstance(await api.changeLoaderBuild(instance.id, loaderTarget));
        if (downloading) {
          prepare(instance.id);
        }
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
    <span class="mark" aria-hidden="true">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 19V6M6.5 11.5 12 6l5.5 5.5" /></svg>
    </span>
    <div class="text">
      <strong>{found ? t("updates.title") : t("updates.none")}</strong>
      {#if instance.pack && packTarget}
        <label class="toggle">
          <input
            type="checkbox"
            bind:checked={() => !skipped.pack, (checked) => (skipped.pack = !checked)}
          >
          <span>
            {t("updates.pack", { from: instance.pack.versionName, to: packTarget.name })}
          </span>
        </label>
      {/if}
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
        <button
          type="button"
          class="pill-button primary"
          disabled={disabled || working || !chosen}
          onclick={update}
        >
          {working ? t("updates.working") : t("updates.update")}
        </button>
      {/if}
      <button type="button" class="pill-button" onclick={() => (dismissed[instance.id] = true)}>
        {found ? t("updates.later") : t("account.dismiss")}
      </button>
    </div>
  </div>
{/if}

<style>
  .notice {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px 14px 14px;
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px color-mix(in srgb, var(--accent) 40%, transparent);
    backdrop-filter: var(--glass);
  }

  .mark {
    display: grid;
    flex: none;
    align-self: flex-start;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 12px;
    background: var(--accent);
    color: var(--accent-text);
  }

  svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .text {
    display: grid;
    flex: 1;
    gap: 6px;
    min-width: 0;
  }

  strong {
    font-size: 15px;
    font-weight: 800;
  }

  p {
    margin: 0;
    font-size: 13px;
  }

  .error {
    color: var(--danger-text);
  }

  .actions {
    display: flex;
    flex: none;
    gap: 8px;
  }
</style>
