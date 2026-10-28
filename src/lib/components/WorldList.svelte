<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { api, type World, type WorldList } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { relativeTime } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import { UI_ICONS } from "$lib/icons";
  import { library, play } from "$lib/state/instances.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  let { instanceId }: { instanceId: string } = $props();

  let list = $state<WorldList | null>(null);
  let error = $state<string | null>(null);
  let backingUp = $state<string | null>(null);
  let deleting = $state<World | null>(null);
  let confirming = $state(false);

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
      list = await api.worlds(instanceId);
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

{#if list && list.worlds.length === 0}
  <div class="empty-state">
    <h3>{t("worlds.empty")}</h3>
    <p>{t("worlds.emptyHint")}</p>
  </div>
{:else if list}
  {#if !list.quickPlay}
    <p class="hint">{t("worlds.noQuickPlay")}</p>
  {/if}
  <ul class="rows">
    {#each list.worlds as world (world.folder)}
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
          title={t("worlds.openFolder")}
          aria-label={t("worlds.openFolder")}
          onclick={() => api.openWorldFolder(instanceId, world.folder)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.folder} /></svg>
        </button>
        <button
          type="button"
          class="icon-button"
          title={backingUp === world.folder ? t("worlds.backingUp") : t("worlds.backup")}
          aria-label={t("worlds.backup")}
          disabled={backingUp !== null}
          onclick={() => backUp(world)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.backup} /></svg>
        </button>
        <button
          type="button"
          class="icon-button danger"
          title={t("worlds.delete")}
          aria-label={t("worlds.delete")}
          disabled={busy}
          onclick={() => askDelete(world)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.delete} /></svg>
        </button>
        {#if list.quickPlay}
          <button
            type="button"
            class="pill-button primary"
            disabled={busy}
            onclick={() => play(instanceId, { kind: "world", value: world.folder })}
          >
            {t("worlds.play")}
          </button>
        {/if}
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

<style>
  .hint {
    margin: 0 0 10px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .error {
    color: var(--danger);
  }
</style>
