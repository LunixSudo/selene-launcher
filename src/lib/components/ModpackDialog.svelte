<script lang="ts">
  import { api, type Instance, type PackVersion, type Source } from "#lib/api.ts";
  import { describeError } from "#lib/errors.ts";
  import { locale, t } from "#lib/i18n/index.svelte.ts";
  import {
    installModpack,
    library,
    prepare,
    replaceInstance,
  } from "#lib/state/instances.svelte.ts";
  import Dialog from "./Dialog.svelte";
  import Select from "./Select.svelte";

  let {
    open = $bindable(),
    source,
    projectId,
    title,
    instances = [],
    gameVersion = "",
    oninstall,
  }: {
    open: boolean;
    source: Source;
    projectId: string;
    title: string;
    /** Instances of this pack to update; empty to install a new one. */
    instances?: Instance[];
    /** The game version the catalog is filtered by, picked first when the pack has one. */
    gameVersion?: string;
    oninstall?: (done: Promise<Instance | null>) => void;
  } = $props();

  let versions = $state<PackVersion[] | null>(null);
  let versionId = $state("");
  let instanceId = $state("");
  let showBetas = $state(false);
  let working = $state(false);
  let error = $state<string | null>(null);

  const updating = $derived(instances.length > 0);
  const target = $derived(instances.find((instance) => instance.id === instanceId) ?? null);
  const current = $derived(target?.pack?.versionId ?? null);
  const shown = $derived(
    (versions ?? []).filter((version) => showBetas || version.release || version.id === versionId),
  );
  const options = $derived(shown.map((version) => ({ value: version.id, label: label(version) })));
  const instanceOptions = $derived(
    instances.map((instance) => ({
      value: instance.id,
      label: `${instance.name} · ${instance.pack?.versionName ?? ""}`,
    })),
  );

  $effect(() => {
    if (!open) {
      return;
    }
    versions = null;
    error = null;
    instanceId = instances[0]?.id ?? "";
    api
      .modpackVersions(source, projectId)
      .then((found) => {
        versions = found;
        const fitting = gameVersion
          ? found.filter((version) => version.gameVersions.includes(gameVersion))
          : found;
        const pool = fitting.length > 0 ? fitting : found;
        versionId = (pool.find((version) => version.release) ?? pool[0])?.id ?? "";
      })
      .catch((err) => {
        error = describeError(err);
      });
  });

  function label(version: PackVersion): string {
    const parts = [version.name];
    if (version.gameVersions.length > 0) {
      parts.push(version.gameVersions.join(", "));
    }
    if (version.published) {
      parts.push(new Date(version.published).toLocaleDateString(locale()));
    }
    if (version.id === current) {
      parts.push(t("modpacks.current"));
    }
    return parts.join(" · ");
  }

  async function submit() {
    if (!versionId) {
      return;
    }
    error = null;
    if (!updating) {
      open = false;
      oninstall?.(installModpack(source, projectId, versionId, title));
      return;
    }
    if (!target) {
      return;
    }
    working = true;
    try {
      const result = await api.updateModpack(target.id, versionId);
      replaceInstance(result.instance);
      library.importManual = result.manual;
      prepare(target.id);
      open = false;
    } catch (err) {
      error = describeError(err);
    } finally {
      working = false;
    }
  }
</script>

<Dialog
  bind:open
  title={t(updating ? "modpacks.updateTitle" : "modpacks.installTitle", { title })}
  width={480}
  onsubmit={submit}
>
  {#if updating && instances.length > 1}
    <div class="form-field">
      <span>{t("modpacks.instance")}</span>
      <Select bind:value={instanceId} options={instanceOptions} />
    </div>
  {/if}

  <div class="form-field">
    <span>{t("modpacks.version")}</span>
    {#if versions === null && !error}
      <div class="placeholder">{t("modpacks.loadingVersions")}</div>
    {:else if versions && versions.length === 0}
      <p class="muted">{t("modpacks.noVersions")}</p>
    {:else if versions}
      <Select bind:value={versionId} {options} />
      {#if versions.some((version) => !version.release)}
        <label class="toggle">
          <input type="checkbox" bind:checked={showBetas}>
          <span>{t("modpacks.betas")}</span>
        </label>
      {/if}
    {/if}
  </div>

  {#if updating}
    <p class="muted">{t("modpacks.updateHint")}</p>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#snippet actions()}
    <button type="button" onclick={() => (open = false)}>{t("create.cancel")}</button>
    <button
      type="submit"
      class="primary"
      disabled={working || !versionId || (updating && versionId === current)}
    >
      {working ? t("updates.working") : updating ? t("modpacks.update") : t("modpacks.install")}
    </button>
  {/snippet}
</Dialog>

<style>
  .placeholder,
  .muted {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  .error {
    margin: 0;
    color: var(--danger);
  }
</style>
