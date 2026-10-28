<script lang="ts">
  import { api, type Instance, type LoaderVersion, type VersionChange } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { t } from "$lib/i18n/index.svelte";
  import { preferredRelease } from "$lib/loaders";
  import { replaceInstance } from "$lib/state/instances.svelte";
  import Dialog from "./Dialog.svelte";
  import Select from "./Select.svelte";

  let { instance, open = $bindable() }: { instance: Instance; open: boolean } = $props();

  let releases = $state<string[]>([]);
  let version = $state("");
  let change = $state<VersionChange | null>(null);
  let loader = $state<LoaderVersion | null>(null);
  let working = $state(false);
  let error = $state<string | null>(null);

  const options = $derived(releases.map((value) => ({ value, label: value })));

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

  $effect(() => {
    void version;
    change = null;
  });

  async function check() {
    working = true;
    error = null;
    try {
      loader = await loaderFor(version);
      change = await api.previewVersionChange(instance.id, version, loader);
    } catch (err) {
      error = describeError(err);
    } finally {
      working = false;
    }
  }

  // Keeps the same loader, on its preferred build for the new game version.
  async function loaderFor(gameVersion: string): Promise<LoaderVersion | null> {
    if (!instance.loader) {
      return null;
    }
    const builds = await api.loaderVersions(instance.loader.loader, gameVersion);
    const build = preferredRelease(builds);
    if (!build) {
      throw { kind: "noCompatibleVersion", message: "" };
    }
    return { loader: instance.loader.loader, version: build.version };
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
    <button type="submit" class="primary" disabled={working || version === instance.version}>
      {working
        ? t("changeVersion.working")
        : change
          ? t("changeVersion.apply")
          : t("changeVersion.check")}
    </button>
  {/snippet}
</Dialog>

<style>
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
