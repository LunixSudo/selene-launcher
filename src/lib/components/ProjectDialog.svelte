<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    api,
    type ContentKind,
    type ProjectPage,
    type ProjectRef,
    type ProjectVersion,
  } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { relativeTime } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import { renderMarkup } from "$lib/markup";
  import Dialog from "./Dialog.svelte";
  import Segmented from "./Segmented.svelte";

  let {
    open = $bindable(),
    instanceId,
    kind,
    project,
    installedFile = null,
    installedVersion = null,
    startOn = "about",
    onchange,
  }: {
    open: boolean;
    instanceId: string;
    kind: ContentKind;
    project: ProjectRef;
    /** File to replace when a version is picked; without it the dialog only shows info. */
    installedFile?: string | null;
    installedVersion?: string | null;
    startOn?: "about" | "versions";
    onchange?: () => void;
  } = $props();

  type View = "about" | "versions";

  let view = $state<View>("about");
  let page = $state<ProjectPage | null>(null);
  let versions = $state<ProjectVersion[] | null>(null);
  let expanded = $state<string | null>(null);
  let changelogs = $state<Record<string, string>>({});
  let busy = $state<string | null>(null);
  let error = $state<string | null>(null);
  let scroller = $state<HTMLDivElement>();

  const views = $derived([
    { value: "about" as View, label: t("project.about") },
    { value: "versions" as View, label: t("project.versions") },
  ]);

  $effect(() => {
    if (!open) {
      return;
    }
    view = startOn;
    page = null;
    versions = null;
    expanded = null;
    error = null;
    const ref = project;
    api
      .projectPage(kind, ref)
      .then((loaded) => {
        page = loaded;
      })
      .catch((err) => {
        error = describeError(err);
      });
    api
      .projectVersions(instanceId, kind, ref)
      .then((loaded) => {
        versions = loaded;
      })
      .catch(() => {
        versions = [];
      });
  });

  async function toggleChangelog(version: ProjectVersion) {
    expanded = expanded === version.id ? null : version.id;
    if (expanded && !(version.id in changelogs)) {
      changelogs[version.id] =
        version.changelog ??
        (project.source === "curseforge"
          ? await api.curseforgeChangelog(project.id, version.id).catch(() => "")
          : "");
    }
  }

  async function install(version: ProjectVersion) {
    if (!installedFile) {
      return;
    }
    busy = version.id;
    error = null;
    try {
      await api.installVersion(instanceId, kind, project, version.id, installedFile);
      onchange?.();
      open = false;
    } catch (err) {
      error = describeError(err);
    } finally {
      busy = null;
    }
  }

  // Links in descriptions open in the browser instead of inside the launcher.
  $effect(() => {
    const element = scroller;
    if (!element) {
      return;
    }
    const onclick = (event: MouseEvent) => {
      const link = (event.target as HTMLElement).closest("a");
      if (link) {
        event.preventDefault();
        if (link.href.startsWith("http")) {
          void openUrl(link.href);
        }
      }
    };
    element.addEventListener("click", onclick);
    return () => element.removeEventListener("click", onclick);
  });
</script>

<Dialog bind:open title={page?.title ?? ""} width={720} onsubmit={() => (open = false)}>
  <div class="head">
    {#if page?.iconUrl}
      <img class="icon" src={page.iconUrl} alt="">
    {/if}
    <p class="summary">{page?.description ?? ""}</p>
    <Segmented bind:value={view} options={views} name="project-view" label={page?.title ?? ""} />
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <div class="scroll" bind:this={scroller}>
    {#if view === "about"}
      {#if page}
        {#if page.gallery.length > 0}
          <div class="gallery">
            {#each page.gallery as url (url)}
              <img src={url} alt="" loading="lazy">
            {/each}
          </div>
        {/if}
        <div class="markup">{@html renderMarkup(page.body, page.format)}</div>
      {:else if !error}
        <p class="muted">…</p>
      {/if}
    {:else if versions && versions.length === 0}
      <p class="muted">{t("project.noVersions")}</p>
    {:else if versions}
      <ul class="versions">
        {#each versions as version (version.id)}
          {@const current = version.versionNumber === installedVersion}
          <li>
            <div class="version">
              <div class="row-info">
                <span class="row-title">
                  {version.name}
                  {#if version.channel !== "release"}
                    <span class="channel {version.channel}">
                      {t(`project.channels.${version.channel}`)}
                    </span>
                  {/if}
                </span>
                <span class="row-meta">
                  {version.published ? relativeTime(version.published) : version.versionNumber}
                </span>
              </div>
              <button type="button" class="pill-button" onclick={() => toggleChangelog(version)}>
                {t("project.showChangelog")}
              </button>
              {#if installedFile}
                <button
                  type="button"
                  class="pill-button primary"
                  disabled={current || busy !== null}
                  onclick={() => install(version)}
                >
                  {current
                    ? t("project.installed")
                    : busy === version.id
                      ? t("project.installing")
                      : t("project.install")}
                </button>
              {/if}
            </div>
            {#if expanded === version.id}
              {@const changelog = changelogs[version.id]}
              {#if changelog === undefined}
                <p class="muted">…</p>
              {:else if changelog.trim()}
                <div class="markup changelog">
                  {@html renderMarkup(
                    changelog,
                    project.source === "modrinth" ? "markdown" : "html",
                  )}
                </div>
              {:else}
                <p class="muted">{t("project.noChangelog")}</p>
              {/if}
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#snippet actions()}
    {#if page}
      <button type="button" onclick={() => page && openUrl(page.pageUrl)}>
        {t("project.openSite")}
      </button>
    {/if}
    <button type="submit" class="primary">{t("project.close")}</button>
  {/snippet}
</Dialog>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .icon {
    flex: none;
    width: 40px;
    height: 40px;
    border-radius: 10px;
  }

  .summary {
    flex: 1;
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  .scroll {
    max-height: min(60vh, 520px);
    overflow-y: auto;
    user-select: text;
  }

  .gallery {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
    overflow-x: auto;
  }

  .gallery img {
    flex: none;
    height: 140px;
    border-radius: var(--radius-sm);
  }

  .markup {
    font-size: 13px;
    line-height: 1.55;
    overflow-wrap: anywhere;
  }

  .markup :global(img) {
    max-width: 100%;
    height: auto;
  }

  .markup :global(a) {
    color: var(--accent);
  }

  .markup :global(h1),
  .markup :global(h2),
  .markup :global(h3) {
    font-family: var(--font-display);
    font-weight: var(--display-weight);
  }

  .markup :global(pre),
  .markup :global(code) {
    font-family: "Cascadia Mono", Consolas, monospace;
    font-size: 12px;
  }

  .versions {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .versions li {
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .version {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .channel {
    margin-left: 6px;
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--field);
    color: var(--text-muted);
    font-size: 11px;
  }

  .channel.beta {
    color: #c98a00;
  }

  .channel.alpha {
    color: var(--danger);
  }

  .changelog {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }

  .muted {
    color: var(--text-muted);
  }

  .error {
    margin: 0;
    color: var(--danger);
  }
</style>
