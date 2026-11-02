<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";
  import {
    api,
    type ContentKind,
    type ContentUpdate,
    type InstalledFile,
    type ProjectRef,
  } from "#lib/api.ts";
  import { describeError } from "#lib/errors.ts";
  import { t } from "#lib/i18n/index.svelte.ts";
  import { UI_ICONS } from "#lib/icons.ts";
  import { notify } from "#lib/notify.ts";
  import { hasOptifineBuild } from "#lib/optifine.ts";
  import { library } from "#lib/state/instances.svelte.ts";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";
  import ProjectDialog from "./ProjectDialog.svelte";

  let {
    instanceId,
    kind,
    world = null,
    refresh,
  }: { instanceId: string; kind: ContentKind; world?: string | null; refresh: number } = $props();

  const RESULT_SHOWN_MS = 4000;

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
  let enabling = $state<InstalledFile | null>(null);
  let warning = $state(false);

  const gameVersion = $derived(
    library.instances.find((instance) => instance.id === instanceId)?.version ?? "",
  );

  $effect(() => {
    void refresh;
    void kind;
    void world;
    updates = null;
    void load();
  });

  // Files copied in through the file manager show up on their own.
  $effect(() => {
    void api.watchContent(instanceId, kind, world).catch(() => {});
  });

  const unlisten = listen("content-changed", () => void load());
  let optifineStatus = $state<string | null>(null);

  let optifineTimer: ReturnType<typeof setTimeout> | undefined;

  /** Shows the result on the button itself for a few seconds, where the eye already is. */
  function showOptifineResult(text: string) {
    clearTimeout(optifineTimer);
    optifineStatus = text;
    optifineTimer = setTimeout(() => (optifineStatus = null), RESULT_SHOWN_MS);
  }

  const unlistenOptifine = listen<string>("optifine-installed", (event) => {
    if (event.payload === instanceId) {
      showOptifineResult(t("content.optifineAdded"));
      void load();
    }
  });
  const unlistenOptifineFailed = listen<string>("optifine-failed", (event) => {
    if (event.payload === instanceId) {
      showOptifineResult(t("content.optifineFailed"));
    }
  });

  onDestroy(() => {
    clearTimeout(optifineTimer);
    void unlisten.then((stop) => stop());
    void unlistenOptifine.then((stop) => stop());
    void unlistenOptifineFailed.then((stop) => stop());
    void api.unwatchContent();
  });

  async function load() {
    const request = { kind, world };
    error = null;
    try {
      const loaded = await api.installedContent(instanceId, request.kind, request.world);
      // Switching tabs quickly must not show the previous kind's files.
      if (request.kind === kind && request.world === world) {
        files = loaded;
      }
    } catch (err) {
      error = describeError(err);
    }
  }

  async function toggle(file: InstalledFile, input: HTMLInputElement) {
    if (!file.enabled && file.fitsVersion === false) {
      input.checked = false;
      enabling = file;
      warning = true;
      return;
    }
    await setEnabled(file, !file.enabled);
  }

  async function setEnabled(file: InstalledFile, enabled: boolean) {
    await api.setContentEnabled(instanceId, kind, world, file.fileName, enabled);
    await load();
  }

  async function remove(file: InstalledFile) {
    await api.removeContent(instanceId, kind, world, file.fileName);
    updates = null;
    await load();
  }

  const instance = $derived(library.instances.find((entry) => entry.id === instanceId));
  let optifineBuild = $state(false);
  let clashing = $state(false);

  $effect(() => {
    const version = instance?.version;
    optifineBuild = false;
    if (version && instance?.loader?.loader === "forge") {
      void hasOptifineBuild(version).then((found) => {
        optifineBuild = found && instance?.version === version;
      });
    }
  });

  function installOptifine() {
    const embeddium = (files ?? []).some((file) =>
      /embeddium|rubidium/i.test(`${file.fileName} ${file.title ?? ""}`),
    );
    if (embeddium) {
      clashing = true;
    } else {
      void openOptifine();
    }
  }

  async function openOptifine() {
    try {
      await api.openOptifine(instanceId);
    } catch (err) {
      error = describeError(err);
    }
  }

  async function checkUpdates() {
    checking = true;
    error = null;
    try {
      updates = await api.checkUpdates(instanceId, kind, world);
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
      const count = await api.updateContent(instanceId, kind, world);
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

  const missing = $derived.by(() => {
    const projects = new Map<string, ProjectRef>();
    for (const file of files ?? []) {
      if (!file.enabled) {
        continue;
      }
      for (const issue of file.issues) {
        if (issue.kind === "requires") {
          projects.set(issue.project.id, issue.project);
        }
      }
    }
    return [...projects.values()];
  });
  let installingMissing = $state(false);

  async function installMissing(projects: ProjectRef[]) {
    installingMissing = true;
    error = null;
    try {
      await api.installContent(instanceId, kind, world, projects);
      await load();
    } catch (err) {
      error = describeError(err);
    } finally {
      installingMissing = false;
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
  {#if kind === "mod" && optifineBuild}
    <button type="button" class="pill-button" onclick={installOptifine}>
      {optifineStatus ?? t("content.optifine")}
    </button>
  {/if}
  {#if missing.length > 0}
    <button
      type="button"
      class="pill-button"
      disabled={installingMissing}
      onclick={() => installMissing(missing)}
    >
      {installingMissing
        ? t("content.installingMissing")
        : t("content.installMissing", { count: missing.length })}
    </button>
  {/if}
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
          {#if file.issues.length > 0 || (!file.enabled && file.fitsVersion === false)}
            <span class="issues">
              {#if !file.enabled && file.fitsVersion === false}
                <span class="issue requires">
                  {t("content.notForVersion", { version: gameVersion })}
                </span>
              {/if}
              {#each file.issues as issue, index (index)}
                {#if issue.kind === "requires"}
                  <button
                    type="button"
                    class="issue requires"
                    disabled={installingMissing}
                    onclick={() => installMissing([issue.project])}
                  >
                    {t("content.requiresInstall", { title: issue.title })}
                  </button>
                {:else}
                  <span class="issue {issue.kind}"
                    >{t(`content.${issue.kind}`, { title: issue.title })}</span
                  >
                {/if}
              {/each}
            </span>
          {/if}
        </div>
        <input
          class="switch"
          type="checkbox"
          checked={file.enabled}
          aria-label={t("instance.enabled")}
          onchange={(event) => toggle(file, event.currentTarget)}
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

{#if enabling}
  {@const file = enabling}
  <ConfirmDialog
    bind:open={warning}
    title={t("content.enableAnyway.title", { name: file.title ?? file.fileName })}
    message={t(`content.enableAnyway.${kind}`, { version: gameVersion })}
    confirmLabel={t("content.enableAnyway.confirm")}
    onconfirm={() => setEnabled(file, true)}
  />
{/if}

<ConfirmDialog
  bind:open={clashing}
  title={t("content.optifineClashTitle")}
  message={t("content.optifineClash")}
  confirmLabel={t("content.optifineClashConfirm")}
  onconfirm={openOptifine}
/>

{#if shown}
  <ProjectDialog
    bind:open={showing}
    {instanceId}
    {kind}
    {world}
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

  button.issue {
    border: 0;
    font: inherit;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  button.issue:hover:not(:disabled) {
    background: color-mix(in srgb, #e0a100 30%, transparent);
  }

  .issue.conflictsWith,
  .issue.duplicate {
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
