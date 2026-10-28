<script lang="ts">
  import {
    api,
    type Instance,
    type LoaderRelease,
    type LoaderVersion,
    type VersionChange,
  } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { t } from "$lib/i18n/index.svelte";
  import { LOADER_NAMES, preferredRelease, releaseOptions } from "$lib/loaders";
  import { replaceInstance } from "$lib/state/instances.svelte";
  import Dialog from "./Dialog.svelte";
  import Select from "./Select.svelte";

  let { instance, open = $bindable() }: { instance: Instance; open: boolean } = $props();

  let releases = $state<string[]>([]);
  let version = $state("");
  let change = $state<VersionChange | null>(null);
  let loaderBuilds = $state<LoaderRelease[] | null>(null);
  let loaderVersion = $state("");
  let working = $state(false);
  let error = $state<string | null>(null);

  const options = $derived(releases.map((value) => ({ value, label: value })));
  const loader = $derived<LoaderVersion | null>(
    instance.loader && loaderVersion
      ? { loader: instance.loader.loader, version: loaderVersion }
      : null,
  );
  const unchanged = $derived(
    version === instance.version && (!instance.loader || loaderVersion === instance.loader.version),
  );

  $effect(() => {
    if (!open) {
      return;
    }
    version = instance.version;
    change = null;
    error = null;
    api
      .versions()
      .then((all) => {
        releases = all.filter((entry) => entry.kind === "release").map((entry) => entry.id);
      })
      .catch((err) => {
        error = describeError(err);
      });
  });

  // The same loader stays, on its current build for the same game version, otherwise on the
  // preferred one for the new version.
  $effect(() => {
    const gameVersion = version;
    const current = instance.loader;
    change = null;
    loaderBuilds = null;
    loaderVersion = "";
    if (!open || !current || !gameVersion) {
      return;
    }
    let stale = false;
    api
      .loaderVersions(current.loader, gameVersion)
      .then((builds) => {
        if (stale) {
          return;
        }
        loaderBuilds = builds;
        const keep = gameVersion === instance.version ? current.version : null;
        loaderVersion =
          builds.find((build) => build.version === keep)?.version ??
          preferredRelease(builds)?.version ??
          "";
      })
      .catch((err) => {
        error = describeError(err);
      });
    return () => {
      stale = true;
    };
  });

  $effect(() => {
    void loaderVersion;
    change = null;
  });

  async function check() {
    if (instance.loader && !loader) {
      error = describeError({ kind: "noCompatibleVersion", message: "" });
      return;
    }
    working = true;
    error = null;
    try {
      change = await api.previewVersionChange(instance.id, version, loader);
    } catch (err) {
      error = describeError(err);
    } finally {
      working = false;
    }
  }

  async function apply() {
    if (!change) {
      await check();
      return;
    }
    working = true;
    error = null;
    try {
      replaceInstance(await api.changeVersion(instance.id, version, loader));
      open = false;
    } catch (err) {
      error = describeError(err);
    } finally {
      working = false;
    }
  }
</script>

<Dialog bind:open title={t("changeVersion.title")} width={480} onsubmit={apply}>
  <div class="form-field">
    <span>{t("create.version")}</span>
    <Select bind:value={version} {options} />
  </div>

  {#if instance.loader}
    <div class="form-field">
      <span
        >{t("changeVersion.loaderVersion", { loader: LOADER_NAMES[instance.loader.loader] })}</span
      >
      {#if !loaderBuilds}
        <div class="placeholder">{t("create.loadingLoader")}</div>
      {:else if loaderBuilds.length === 0}
        <p class="muted">
          {t("create.loaderUnsupported", { loader: LOADER_NAMES[instance.loader.loader] })}
        </p>
      {:else}
        <Select bind:value={loaderVersion} options={releaseOptions(loaderBuilds)} />
      {/if}
    </div>
  {/if}

  {#if change}
    {#if change.updated.length > 0}
      <section>
        <h3>{t("changeVersion.updated", { count: change.updated.length })}</h3>
        <ul>
          {#each change.updated as update (update.fileName)}
            <li>{update.title} <span class="muted">→ {update.to}</span></li>
          {/each}
        </ul>
      </section>
    {/if}
    {#if change.incompatible.length > 0}
      <section>
        <h3 class="warn">
          {t("changeVersion.incompatible", { count: change.incompatible.length })}
        </h3>
        <p class="muted">{t("changeVersion.incompatibleHint")}</p>
        <ul>
          {#each change.incompatible as title (title)}
            <li>{title}</li>
          {/each}
        </ul>
      </section>
    {/if}
    {#if change.unknown.length > 0}
      <section>
        <h3>{t("changeVersion.unknown", { count: change.unknown.length })}</h3>
        <ul>
          {#each change.unknown as title (title)}
            <li>{title}</li>
          {/each}
        </ul>
      </section>
    {/if}
    {#if change.updated.length + change.incompatible.length + change.unknown.length === 0}
      <p class="muted">{t("changeVersion.nothing")}</p>
    {/if}
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#snippet actions()}
    <button type="button" onclick={() => (open = false)}>{t("create.cancel")}</button>
    <button type="submit" class="primary" disabled={working || unchanged}>
      {working
        ? t("changeVersion.working")
        : change
          ? t("changeVersion.apply")
          : t("changeVersion.check")}
    </button>
  {/snippet}
</Dialog>

<style>
  .placeholder {
    display: flex;
    align-items: center;
    height: 36px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--field);
    color: var(--text-muted);
    font-size: 14px;
  }

  h3 {
    margin: 0 0 6px;
    font-size: 13px;
  }

  .warn {
    color: var(--danger);
  }

  ul {
    display: grid;
    gap: 2px;
    max-height: 160px;
    margin: 0;
    padding-left: 18px;
    overflow-y: auto;
    font-size: 13px;
  }

  p {
    margin: 0 0 6px;
    font-size: 12px;
  }

  .muted {
    color: var(--text-muted);
  }

  .error {
    color: var(--danger);
  }
</style>
