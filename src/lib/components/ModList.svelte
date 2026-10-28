<script lang="ts">
  import { api, type InstalledMod, type ModUpdate } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { t } from "$lib/i18n/index.svelte";

  let { instanceId, refresh }: { instanceId: string; refresh: number } = $props();

  let mods = $state<InstalledMod[] | null>(null);
  let updates = $state<ModUpdate[] | null>(null);
  let checking = $state(false);
  let updating = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    void refresh;
    void load();
  });

  async function load() {
    try {
      mods = await api.installedMods(instanceId);
    } catch (err) {
      error = describeError(err);
    }
  }

  async function toggle(mod: InstalledMod) {
    await api.setModEnabled(instanceId, mod.fileName, !mod.enabled);
    await load();
  }

  async function remove(mod: InstalledMod) {
    await api.removeMod(instanceId, mod.fileName);
    updates = null;
    await load();
  }

  async function checkUpdates() {
    checking = true;
    error = null;
    try {
      updates = await api.checkModUpdates(instanceId);
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
      await api.updateMods(instanceId);
      updates = [];
      await load();
    } catch (err) {
      error = describeError(err);
    } finally {
      updating = false;
    }
  }
</script>

<div class="toolbar">
  {#if updates && updates.length > 0}
    <button type="button" class="primary" disabled={updating} onclick={updateAll}>
      {updating ? t("instance.updating") : t("instance.updateAll", { count: updates.length })}
    </button>
  {:else if updates}
    <span class="muted">{t("instance.upToDate")}</span>
  {:else}
    <button type="button" disabled={checking || !mods?.length} onclick={checkUpdates}>
      {checking ? t("instance.checking") : t("instance.checkUpdates")}
    </button>
  {/if}
</div>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

{#if mods && mods.length === 0}
  <div class="empty">
    <h3>{t("instance.noMods")}</h3>
    <p>{t("instance.noModsHint")}</p>
  </div>
{:else if mods}
  <ul>
    {#each mods as mod (mod.fileName)}
      {@const update = updates?.find((entry) => entry.fileName === mod.fileName)}
      <li class:disabled={!mod.enabled}>
        {#if mod.project?.icon_url}
          <img src={mod.project.icon_url} alt="" loading="lazy">
        {:else}
          <span class="icon-placeholder" aria-hidden="true"></span>
        {/if}
        <div class="info">
          <span class="title">{mod.project?.title ?? mod.fileName}</span>
          <span class="muted">
            {mod.versionNumber ?? mod.fileName}
            {#if update}
              → <span class="update">{update.to}</span>
            {/if}
          </span>
        </div>
        <label class="switch">
          <input
            type="checkbox"
            role="switch"
            checked={mod.enabled}
            aria-checked={mod.enabled}
            aria-label={t("instance.enabled")}
            onchange={() => toggle(mod)}
          >
          <span aria-hidden="true"></span>
        </label>
        <button
          type="button"
          class="remove"
          aria-label={t("instance.remove")}
          title={t("instance.remove")}
          onclick={() => remove(mod)}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5" />
          </svg>
        </button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .toolbar {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 12px;
  }

  .toolbar button {
    height: 32px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    font-weight: 600;
  }

  .toolbar .primary {
    border: none;
    background: var(--accent);
    color: var(--accent-text);
  }

  .toolbar button:disabled {
    opacity: 0.6;
  }

  ul {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    transition: opacity 150ms var(--ease);
  }

  li.disabled {
    opacity: 0.55;
  }

  img,
  .icon-placeholder {
    flex: none;
    width: 36px;
    height: 36px;
    border-radius: 9px;
    background: var(--field);
  }

  .info {
    display: grid;
    flex: 1;
    min-width: 0;
  }

  .title {
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .muted {
    overflow: hidden;
    color: var(--text-muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .update {
    color: var(--accent);
    font-weight: 600;
  }

  .switch {
    position: relative;
    flex: none;
    width: 38px;
    height: 22px;
  }

  .switch input {
    position: absolute;
    inset: 0;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }

  .switch span {
    position: absolute;
    inset: 0;
    border-radius: 999px;
    background: var(--field);
    box-shadow: inset 0 0 0 1px var(--border);
    transition: background 150ms var(--ease);
    pointer-events: none;
  }

  .switch span::after {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #ffffff;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.25);
    transition: transform 150ms var(--ease);
    content: "";
  }

  .switch input:checked + span {
    background: var(--accent);
  }

  .switch input:checked + span::after {
    transform: translateX(16px);
  }

  .switch input:focus-visible + span {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .remove {
    display: grid;
    flex: none;
    place-items: center;
    width: 30px;
    height: 30px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text-muted);
  }

  .remove:hover {
    background: var(--field);
    color: var(--danger);
  }

  .remove svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .empty {
    padding: 48px 0;
    text-align: center;
  }

  .empty h3 {
    margin: 0 0 4px;
    font-family: var(--font-display);
    font-weight: var(--display-weight);
    font-size: 16px;
  }

  .empty p {
    margin: 0;
    color: var(--text-muted);
  }

  .error {
    color: var(--danger);
  }
</style>
