<script lang="ts">
  import { describeError } from "$lib/errors";
  import { t } from "$lib/i18n/index.svelte";
  import Dialog from "./Dialog.svelte";

  let {
    open = $bindable(),
    title,
    message,
    confirmLabel,
    onconfirm,
  }: {
    open: boolean;
    title: string;
    message: string;
    confirmLabel: string;
    onconfirm: () => Promise<void>;
  } = $props();

  let working = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    if (open) {
      error = null;
    }
  });

  async function confirm() {
    working = true;
    error = null;
    try {
      await onconfirm();
      open = false;
    } catch (err) {
      error = describeError(err);
    } finally {
      working = false;
    }
  }
</script>

<Dialog bind:open {title} onsubmit={confirm}>
  <p>{message}</p>
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#snippet actions()}
    <button type="button" onclick={() => (open = false)}>{t("create.cancel")}</button>
    <button type="submit" class="danger" disabled={working}>{confirmLabel}</button>
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
