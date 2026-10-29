<script lang="ts">
  import { api, type World } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";
  import Select from "./Select.svelte";

  let { instanceId, value = $bindable() }: { instanceId: string; value: string | null } = $props();

  let worlds = $state<World[] | null>(null);

  const options = $derived(
    (worlds ?? []).map((world) => ({ value: world.folder, label: world.name })),
  );

  $effect(() => {
    api
      .worlds(instanceId)
      .then((list) => {
        worlds = list;
        if (!list.some((world) => world.folder === value)) {
          value = list[0]?.folder ?? null;
        }
      })
      .catch(() => {
        worlds = [];
      });
  });
</script>

{#if worlds && worlds.length === 0}
  <p class="hint">{t("content.noWorlds")}</p>
{:else if worlds && value !== null}
  <div class="picker">
    <span>{t("content.world")}</span>
    <div class="select">
      <Select bind:value={() => value ?? "", (folder) => (value = folder)} {options} />
    </div>
  </div>
{/if}

<style>
  .picker {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .select {
    width: 260px;
  }

  .hint {
    margin: 0 0 12px;
    color: var(--text-muted);
    font-size: 13px;
  }
</style>
