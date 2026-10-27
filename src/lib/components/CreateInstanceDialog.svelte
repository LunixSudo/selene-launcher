<script lang="ts">
  import { api, isCommandError, type VersionSummary } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";
  import { createInstance } from "$lib/state/instances.svelte";

  let { open = $bindable() }: { open: boolean } = $props();

  let dialog: HTMLDialogElement;
  let versions = $state<VersionSummary[] | null>(null);
  let name = $state("");
  let version = $state("");
  let showSnapshots = $state(false);
  let error = $state<string | null>(null);
  let submitting = $state(false);

  const visibleVersions = $derived(
    (versions ?? []).filter(
      (entry) => entry.kind === "release" || (showSnapshots && entry.kind === "snapshot"),
    ),
  );

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
      await createInstance(name.trim() || version, version);
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
        <select id="instance-version" bind:value={version} required>
          {#each visibleVersions as entry (entry.id)}
            <option value={entry.id}>{entry.id}</option>
          {/each}
        </select>
      {:else}
        <div class="placeholder">{t("create.loadingVersions")}</div>
      {/if}
    </div>

    <label class="toggle">
      <input type="checkbox" bind:checked={showSnapshots}>
      <span>{t("create.snapshots")}</span>
    </label>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <div class="actions">
      <button type="button" class="secondary" onclick={() => (open = false)}>
        {t("create.cancel")}
      </button>
      <button type="submit" class="primary" disabled={!version || submitting}>
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
  select,
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
