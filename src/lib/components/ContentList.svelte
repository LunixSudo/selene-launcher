<script lang="ts">
  import {
    api,
    type ContentKind,
    type ContentUpdate,
    type InstalledFile,
    type ProjectRef,
  } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { t } from "$lib/i18n/index.svelte";
  import { UI_ICONS } from "$lib/icons";
  import { notify } from "$lib/notify";
  import { library } from "$lib/state/instances.svelte";
  import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";
  import ProjectDialog from "./ProjectDialog.svelte";

  let { instanceId, kind, refresh }: { instanceId: string; kind: ContentKind; refresh: number } =
    $props();

  let files = $state<InstalledFile[] | null>(null);
  let updates = $state<ContentUpdate[] | null>(null);
  let checking = $state(false);
  let updating = $state(false);
  let error = $state<string | null>(null);
  let menuFile = $state<InstalledFile | null>(null);
  let menuAt = $state<{ x: number; y: number } | null>(null);
  let shown = $state<{ file: InstalledFile; project: ProjectRef } | null>(null);
  let showing = $state(false);
  let startOn = $state<"about" | "versions">("about");

  $effect(() => {
    void refresh;
    void kind;
    updates = null;
    void load();
  });

  async function load() {
    try {
      files = await api.installedContent(instanceId, kind);
    } catch (err) {
      error = describeError(err);
    }
  }

  async function toggle(file: InstalledFile) {
    await api.setContentEnabled(instanceId, kind, file.fileName, !file.enabled);
    await load();
  }

  async function remove(file: InstalledFile) {
    await api.removeContent(instanceId, kind, file.fileName);
    updates = null;
    await load();
  }

  async function checkUpdates() {
    checking = true;
    error = null;
    try {
      updates = await api.checkUpdates(instanceId, kind);
    } catch (err) {
      error = describeError(err);
    } finally {
      checking = false;
    }
  }

  async function updateAll() {
    updating = true;
    error = null;
    try {
      const count = await api.updateContent(instanceId, kind);
      updates = [];
      if (count > 0) {
        const name = library.instances.find((instance) => instance.id === instanceId)?.name ?? "";
        void notify(t("notify.modsUpdated"), t("notify.modsUpdatedBody", { name, count }));
      }
      await load();
    } catch (err) {
      error = describeError(err);
    } finally {
      updating = false;
    }
  }

  function show(file: InstalledFile, view: "about" | "versions") {
    if (file.project) {
      shown = { file, project: file.project };
      startOn = view;
      showing = true;
    }
  }

  function openMenu(file: InstalledFile, event: MouseEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    menuFile = file;
    menuAt = { x: rect.left, y: rect.bottom + 4 };
  }

  const menu = $derived.by((): MenuItem[] => {
    const file = menuFile;
    if (!file) {
      return [];
    }
    return [
      {
        label: t("content.details"),
        icon: UI_ICONS.open,
        disabled: !file.project,
        action: () => show(file, "about"),
      },
      {
        label: t("content.changeVersion"),
        icon: UI_ICONS.backup,
        disabled: !file.project,
        action: () => show(file, "versions"),
      },
      {
        label: t("instance.remove"),
        icon: UI_ICONS.delete,
        danger: true,
        action: () => remove(file),
      },
    ];
  });
</script>

<div class="toolbar">
  {#if updates && updates.length > 0}
    <button type="button" class="pill-button primary" disabled={updating} onclick={updateAll}>
      {updating ? t("instance.updating") : t("instance.updateAll", { count: updates.length })}
    </button>
  {:else if updates}
    <span class="muted">{t("instance.upToDate")}</span>
  {:else}
    <button
      type="button"
      class="pill-button"
      disabled={checking || !files?.length}
      onclick={checkUpdates}
    >
      {checking ? t("instance.checking") : t("instance.checkUpdates")}
    </button>
  {/if}
</div>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

{#if files && files.length === 0}
  <div class="empty-state">
    <h3>{t(`content.empty.${kind}`)}</h3>
    <p>{t("content.emptyHint")}</p>
  </div>
{:else if files}
  <ul class="rows">
    {#each files as file (file.fileName)}
      {@const update = updates?.find((entry) => entry.fileName === file.fileName)}
      <li class="row-item" class:disabled={!file.enabled}>
        {#if file.iconUrl}
          <img class="row-icon" src={file.iconUrl} alt="" loading="lazy">
        {:else}
          <span class="row-icon" aria-hidden="true"></span>
        {/if}
        <div class="row-info">
          {#if file.project}
            <button type="button" class="row-title title" onclick={() => show(file, "about")}>
              {file.title ?? file.fileName}
            </button>
          {:else}
            <span class="row-title">{file.title ?? file.fileName}</span>
          {/if}
          <span class="row-meta">
            {file.versionNumber ?? file.fileName}
            {#if update}
              → <span class="update">{update.to}</span>
            {/if}
          </span>
          {#if file.issues.length > 0}
            <span class="issues">
              {#each file.issues as issue (issue.kind + issue.title)}
                <span class="issue {issue.kind}"
                  >{t(`content.${issue.kind}`, { title: issue.title })}</span
                >
              {/each}
            </span>
          {/if}
        </div>
        <input
          class="switch"
          type="checkbox"
          checked={file.enabled}
          aria-label={t("instance.enabled")}
          onchange={() => toggle(file)}
        >
        <button
          type="button"
          class="icon-button"
          aria-label={t("instances.actions")}
          onclick={(event) => openMenu(file, event)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.more} /></svg>
        </button>
      </li>
    {/each}
  </ul>
{/if}

<ContextMenu items={menu} bind:position={menuAt} />

{#if shown}
  <ProjectDialog
    bind:open={showing}
    {instanceId}
    {kind}
    project={shown.project}
    installedFile={shown.file.fileName}
    installedVersion={shown.file.versionNumber}
    {startOn}
    onchange={load}
  />
{/if}

<style>
  .row-item {
    transition: opacity 150ms var(--ease);
  }

  .row-item.disabled {
    opacity: 0.55;
  }

  .title {
    width: fit-content;
    max-width: 100%;
    padding: 0;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
  }

  .title:hover {
    color: var(--accent);
  }

  .update {
    color: var(--accent);
    font-weight: 600;
  }

  .issues {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
  }

  .issue {
    padding: 1px 8px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
  }

  .issue.requires {
    background: color-mix(in srgb, #e0a100 18%, transparent);
    color: #c98a00;
  }

  .issue.conflictsWith {
    background: color-mix(in srgb, var(--danger) 16%, transparent);
    color: var(--danger);
  }

  .icon-button svg {
    stroke-width: 3;
  }

  .muted {
    color: var(--text-muted);
    font-size: 13px;
  }

  .error {
    color: var(--danger);
  }
</style>
