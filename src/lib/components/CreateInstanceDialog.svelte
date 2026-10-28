<script lang="ts">
  import {
    api,
    isCommandError,
    type Loader,
    type LoaderRelease,
    type VersionSummary,
  } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";
  import { LOADER_NAMES, LOADERS } from "$lib/loaders";
  import { createInstance } from "$lib/state/instances.svelte";
  import Select from "./Select.svelte";

  let { open = $bindable() }: { open: boolean } = $props();

  let dialog: HTMLDialogElement;
  let versions = $state<VersionSummary[] | null>(null);
  let name = $state("");
  let version = $state("");
  let showSnapshots = $state(false);
  let loader = $state<Loader | null>(null);
  let loaderReleases = $state<LoaderRelease[] | null>(null);
  let loaderVersion = $state("");
  let error = $state<string | null>(null);
  let submitting = $state(false);

  const visibleVersions = $derived(
    (versions ?? []).filter(
      (entry) => entry.kind === "release" || (showSnapshots && entry.kind === "snapshot"),
    ),
  );

  $effect(() => {
    const selected = loader;
    const gameVersion = version;
    loaderReleases = null;
    loaderVersion = "";
    if (!selected || !gameVersion) {
      return;
    }

    let stale = false;
    api
      .loaderVersions(selected, gameVersion)
      .then((releases) => {
        if (stale) {
          return;
        }
        loaderReleases = releases;
        loaderVersion = (releases.find((release) => release.stable) ?? releases[0])?.version ?? "";
      })
      .catch((err) => {
        error = t(`errors.${isCommandError(err) ? err.kind : "core"}`);
      });
    return () => {
      stale = true;
    };
  });

  $effect(() => {
    if (open) {
      dialog.showModal();
      void loadVersions();
    } else {
      dialog.close();
    }
  });

  async function loadVersions() {
    if (versions) {
      return;
    }
    try {
      versions = await api.versions();
      version ||= versions.find((entry) => entry.kind === "release")?.id ?? "";
    } catch (err) {
      error = t(`errors.${isCommandError(err) ? err.kind : "core"}`);
    }
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    submitting = true;
    error = null;
    try {
      const pinned = loader && loaderVersion ? { loader, version: loaderVersion } : null;
      const fallbackName = loader ? `${LOADER_NAMES[loader]} ${version}` : version;
      await createInstance(name.trim() || fallbackName, version, pinned);
      name = "";
      open = false;
    } catch (err) {
      error = t(`errors.${isCommandError(err) ? err.kind : "core"}`);
    } finally {
      submitting = false;
    }
  }
</script>

<dialog bind:this={dialog} onclose={() => (open = false)}>
  <form onsubmit={submit}>
    <h2>{t("create.title")}</h2>

    <label>
      <span>{t("create.name")}</span>
      <input bind:value={name} placeholder={t("create.namePlaceholder")} maxlength="64">
    </label>

    <div class="field">
      <label for="instance-version">{t("create.version")}</label>
      {#if versions}
        <Select
          id="instance-version"
          bind:value={version}
          options={visibleVersions.map((entry) => ({ value: entry.id, label: entry.id }))}
        />
      {:else}
        <div class="placeholder">{t("create.loadingVersions")}</div>
      {/if}
    </div>

    <label class="toggle">
      <input type="checkbox" bind:checked={showSnapshots}>
      <span>{t("create.snapshots")}</span>
    </label>

    <fieldset class="field">
      <legend>{t("create.loader")}</legend>
      <div class="segmented">
        {#each [null, ...LOADERS] as option (option)}
          <label>
            <input type="radio" name="loader" value={option} bind:group={loader}>
            <span>{option ? LOADER_NAMES[option] : t("create.vanilla")}</span>
          </label>
        {/each}
      </div>
    </fieldset>

    {#if loader}
      <div class="field">
        <label for="loader-version">{t("create.loaderVersion")}</label>
        {#if !loaderReleases}
          <div class="placeholder">{t("create.loadingLoader")}</div>
        {:else if loaderReleases.length === 0}
          <p class="hint">{t("create.loaderUnsupported", { loader: LOADER_NAMES[loader] })}</p>
        {:else}
          <Select
            id="loader-version"
            bind:value={loaderVersion}
            options={loaderReleases.map((release) => ({
              value: release.version,
              label: release.stable
                ? release.version
                : `${release.version} (${t("create.unstable")})`,
            }))}
          />
        {/if}
      </div>
    {/if}

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <div class="actions">
      <button type="button" class="secondary" onclick={() => (open = false)}>
        {t("create.cancel")}
      </button>
      <button
        type="submit"
        class="primary"
        disabled={!version || (loader !== null && !loaderVersion) || submitting}
      >
        {t("create.submit")}
      </button>
    </div>
  </form>
</dialog>

<style>
  dialog {
    width: min(420px, calc(100vw - 32px));
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-solid);
    color: var(--text);
    box-shadow: 0 24px 64px rgb(0 0 0 / 0.24);
  }

  dialog[open] {
    animation: pop 220ms var(--ease);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.25);
    backdrop-filter: blur(6px);
  }

  form {
    display: grid;
    gap: 14px;
    padding: 24px;
  }

  h2 {
    margin: 0 0 4px;
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 600;
  }

  label,
  .field {
    display: grid;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
  }

  input:not([type="checkbox"]),
  .placeholder {
    height: 36px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--field);
    color: var(--text);
    font: inherit;
    font-size: 14px;
  }

  .placeholder {
    display: flex;
    align-items: center;
    color: var(--text-muted);
  }

  .segmented {
    display: grid;
    grid-auto-columns: 1fr;
    grid-auto-flow: column;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius-sm);
    background: var(--field);
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: none;
  }

  legend {
    margin-bottom: 6px;
    padding: 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  .segmented label {
    display: grid;
    place-items: center;
    height: 30px;
    border-radius: 6px;
    color: var(--text);
    font-size: 13px;
    transition: background 150ms var(--ease);
  }

  .segmented input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .segmented label:has(input:checked) {
    background: var(--surface-solid);
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.12);
    font-weight: 600;
  }

  .segmented label:has(input:focus-visible) {
    outline: 2px solid var(--accent);
  }

  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text);
    font-size: 13px;
  }

  .toggle input {
    accent-color: var(--accent);
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 13px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }

  .actions button {
    height: 34px;
    padding: 0 16px;
    border: none;
    border-radius: var(--radius-sm);
    font-weight: 600;
  }

  .primary {
    background: var(--accent);
    color: var(--accent-text);
  }

  .primary:disabled {
    opacity: 0.5;
  }

  .secondary {
    background: var(--field);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
  }
</style>
