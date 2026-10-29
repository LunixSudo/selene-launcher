<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { api, type World } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { relativeTime } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import { UI_ICONS } from "$lib/icons";
  import { library } from "$lib/state/instances.svelte";
  import { tooltip } from "$lib/tooltip";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import Dialog from "./Dialog.svelte";

  let { instanceId }: { instanceId: string } = $props();

  let worlds = $state<World[] | null>(null);
  let error = $state<string | null>(null);
  let backingUp = $state<string | null>(null);
  let deleting = $state<World | null>(null);
  let confirming = $state(false);
  let renaming = $state<World | null>(null);
  let renameOpen = $state(false);
  let newName = $state("");
  let renameError = $state<string | null>(null);

  const launch = $derived(library.launches[instanceId]);
  const busy = $derived(launch !== undefined && launch.status !== "failed");

  $effect(() => {
    void instanceId;
    // Worlds change while the game runs; reload once it closes.
    void launch;
    void load();
  });

  async function load() {
    try {
      worlds = await api.worlds(instanceId);
    } catch (err) {
      error = describeError(err);
    }
  }

  async function backUp(world: World) {
    backingUp = world.folder;
    error = null;
    try {
      await api.backUpWorld(instanceId, world.folder);
    } catch (err) {
      error = describeError(err);
    } finally {
      backingUp = null;
    }
  }

  function askDelete(world: World) {
    deleting = world;
    confirming = true;
  }

  async function remove(world: World) {
    await api.deleteWorld(instanceId, world.folder);
    await load();
  }

  function askRename(world: World) {
    renaming = world;
    newName = world.name;
    renameError = null;
    renameOpen = true;
  }

  async function rename() {
    if (!renaming) {
      return;
    }
    renameError = null;
    try {
      await api.renameWorld(instanceId, renaming.folder, newName);
      renameOpen = false;
      await load();
    } catch (err) {
      renameError = describeError(err);
    }
  }

  function details(world: World): string {
    const parts = [];
    if (world.gameMode) {
      parts.push(t(`worlds.modes.${world.gameMode}`));
    }
    if (world.lastPlayed) {
      parts.push(relativeTime(world.lastPlayed));
    }
    return parts.join(" · ");
  }
</script>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

{#if worlds && worlds.length === 0}
  <div class="empty-state">
    <h3>{t("worlds.empty")}</h3>
    <p>{t("worlds.emptyHint")}</p>
  </div>
{:else if worlds}
  <ul class="rows">
    {#each worlds as world (world.folder)}
      <li class="row-item">
        {#if world.icon}
          <img class="row-icon" src={convertFileSrc(world.icon)} alt="">
        {:else}
          <span class="row-icon" aria-hidden="true"></span>
        {/if}
        <div class="row-info">
          <span class="row-title">{world.name}</span>
          <span class="row-meta">{details(world)}</span>
        </div>
        <button
          type="button"
          class="icon-button"
          {@attach tooltip(t("worlds.rename"))}
          aria-label={t("worlds.rename")}
          disabled={busy}
          onclick={() => askRename(world)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.edit} /></svg>
        </button>
        <button
          type="button"
          class="icon-button"
          {@attach tooltip(t("worlds.openFolder"))}
          aria-label={t("worlds.openFolder")}
          onclick={() => api.openWorldFolder(instanceId, world.folder)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.folder} /></svg>
        </button>
        <button
          type="button"
          class="icon-button"
          {@attach tooltip(backingUp === world.folder ? t("worlds.backingUp") : t("worlds.backup"))}
          aria-label={t("worlds.backup")}
          disabled={backingUp !== null}
          onclick={() => backUp(world)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.backup} /></svg>
        </button>
        <button
          type="button"
          class="icon-button danger"
          {@attach tooltip(t("worlds.delete"))}
          aria-label={t("worlds.delete")}
          disabled={busy}
          onclick={() => askDelete(world)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.delete} /></svg>
        </button>
      </li>
    {/each}
  </ul>
{/if}

{#if deleting}
  {@const world = deleting}
  <ConfirmDialog
    bind:open={confirming}
    title={t("worlds.deleteTitle", { name: world.name })}
    message={t("worlds.deleteMessage")}
    confirmLabel={t("worlds.delete")}
    onconfirm={() => remove(world)}
  />
{/if}

<Dialog bind:open={renameOpen} title={t("worlds.renameTitle")} onsubmit={rename}>
  <label class="form-field">
    <span>{t("worlds.name")}</span>
    <input bind:value={newName} required maxlength="64" spellcheck="false">
  </label>
  {#if renameError}
    <p class="error" role="alert">{renameError}</p>
  {/if}

  {#snippet actions()}
    <button type="button" onclick={() => (renameOpen = false)}>{t("create.cancel")}</button>
    <button type="submit" class="primary" disabled={!newName.trim()}>
      {t("servers.save")}
    </button>
  {/snippet}
</Dialog>

<style>
  .error {
    margin: 0;
    color: var(--danger);
    font-size: 13px;
  }
</style>
