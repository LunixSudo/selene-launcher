<script lang="ts">
  import type { Instance } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { t } from "$lib/i18n/index.svelte";
  import { deleteInstance } from "$lib/state/instances.svelte";
  import Dialog from "./Dialog.svelte";

  let { instance, open = $bindable() }: { instance: Instance; open: boolean } = $props();

  let deleting = $state(false);
  let error = $state<string | null>(null);

  async function confirm() {
    deleting = true;
    error = null;
    try {
      await deleteInstance(instance.id);
      open = false;
    } catch (err) {
      error = describeError(err);
    } finally {
      deleting = false;
    }
  }
</script>

<Dialog bind:open title={t("delete.title", { name: instance.name })} onsubmit={confirm}>
  <p>{t("delete.message")}</p>
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#snippet actions()}
    <button type="button" onclick={() => (open = false)}>{t("create.cancel")}</button>
    <button type="submit" class="danger" disabled={deleting}>{t("delete.confirm")}</button>
  {/snippet}
</Dialog>

<style>
  p {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.5;
  }

  .error {
    color: var(--danger);
  }
</style>
