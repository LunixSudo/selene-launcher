<script lang="ts">
  import { api, type Server } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { t } from "$lib/i18n/index.svelte";
  import { UI_ICONS } from "$lib/icons";
  import { library, play } from "$lib/state/instances.svelte";
  import { tooltip } from "$lib/tooltip";
  import Dialog from "./Dialog.svelte";

  let { instanceId }: { instanceId: string } = $props();

  let servers = $state<Server[] | null>(null);
  let error = $state<string | null>(null);
  let editing = $state(false);
  let editIndex = $state<number | null>(null);
  let name = $state("");
  let address = $state("");

  const status = $derived(library.launches[instanceId]?.status);
  const busy = $derived(status !== undefined && status !== "failed");

  $effect(() => {
    void instanceId;
    void status;
    void load();
  });

  async function load() {
    try {
      servers = await api.servers(instanceId);
    } catch (err) {
      error = describeError(err);
    }
  }

  async function save(next: Server[]) {
    error = null;
    try {
      await api.saveServers(instanceId, next);
      servers = next;
    } catch (err) {
      error = describeError(err);
    }
  }

  function edit(index: number | null) {
    editIndex = index;
    const server = index === null ? null : servers?.[index];
    name = server?.name ?? "";
    address = server?.address ?? "";
    editing = true;
  }

  async function submit() {
    const list = [...(servers ?? [])];
    const server: Server = {
      name: name.trim() || t("servers.namePlaceholder"),
      address: address.trim(),
      icon: editIndex === null ? null : (list[editIndex]?.icon ?? null),
    };
    if (editIndex === null) {
      list.push(server);
    } else {
      list[editIndex] = server;
    }
    await save(list);
    editing = false;
  }
</script>

<div class="toolbar">
  <button type="button" class="pill-button" disabled={busy} onclick={() => edit(null)}>
    {t("servers.add")}
  </button>
</div>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

{#if servers && servers.length === 0}
  <div class="empty-state">
    <h3>{t("servers.empty")}</h3>
    <p>{t("servers.emptyHint")}</p>
  </div>
{:else if servers}
  <ul class="rows">
    {#each servers as server, index (index)}
      <li class="row-item">
        {#if server.icon}
          <img class="row-icon" src="data:image/png;base64,{server.icon}" alt="">
        {:else}
          <span class="row-icon" aria-hidden="true"></span>
        {/if}
        <div class="row-info">
          <span class="row-title">{server.name}</span>
          <span class="row-meta">{server.address}</span>
        </div>
        <button
          type="button"
          class="icon-button"
          {@attach tooltip(t("servers.edit"))}
          aria-label={t("servers.edit")}
          disabled={busy}
          onclick={() => edit(index)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.edit} /></svg>
        </button>
        <button
          type="button"
          class="icon-button danger"
          {@attach tooltip(t("servers.remove"))}
          aria-label={t("servers.remove")}
          disabled={busy}
          onclick={() => save(servers?.filter((_, other) => other !== index) ?? [])}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={UI_ICONS.delete} /></svg>
        </button>
        <button
          type="button"
          class="pill-button primary"
          disabled={busy}
          onclick={() => play(instanceId, server.address)}
        >
          {t("servers.join")}
        </button>
      </li>
    {/each}
  </ul>
{/if}

<Dialog
  bind:open={editing}
  title={editIndex === null ? t("servers.add") : t("servers.edit")}
  onsubmit={submit}
>
  <label class="form-field">
    <span>{t("servers.name")}</span>
    <input bind:value={name} maxlength="64" placeholder={t("servers.namePlaceholder")}>
  </label>
  <label class="form-field">
    <span>{t("servers.address")}</span>
    <input bind:value={address} required spellcheck="false" placeholder="play.example.com">
  </label>

  {#snippet actions()}
    <button type="button" onclick={() => (editing = false)}>{t("create.cancel")}</button>
    <button type="submit" class="primary">{t("servers.save")}</button>
  {/snippet}
</Dialog>

<style>
  .error {
    color: var(--danger);
  }
</style>
