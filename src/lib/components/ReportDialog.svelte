<script lang="ts">
  import { api } from "#lib/api.ts";
  import { describeError } from "#lib/errors.ts";
  import { t } from "#lib/i18n/index.svelte.ts";
  import { library } from "#lib/state/instances.svelte.ts";
  import Dialog from "./Dialog.svelte";
  import Select from "./Select.svelte";

  let { open = $bindable(), instanceId: preset = "" }: { open: boolean; instanceId?: string } =
    $props();

  let description = $state("");
  let instanceId = $state("");
  let withLog = $state(true);
  let working = $state(false);
  let sent = $state(false);
  let error = $state<string | null>(null);

  const options = $derived([
    { value: "", label: t("report.noInstance") },
    ...library.instances.map((instance) => ({ value: instance.id, label: instance.name })),
  ]);

  $effect(() => {
    if (open) {
      sent = false;
      error = null;
      instanceId = preset;
    }
  });

  function game(): string {
    const instance = library.instances.find((entry) => entry.id === instanceId);
    if (!instance) {
      return "";
    }
    const loader = instance.loader;
    return loader ? `${instance.version}, ${loader.loader} ${loader.version}` : instance.version;
  }

  async function submit() {
    if (!description.trim()) {
      return;
    }
    working = true;
    error = null;
    try {
      await api.reportProblem(description, game(), withLog);
      sent = true;
      description = "";
    } catch (err) {
      error = describeError(err);
    } finally {
      working = false;
    }
  }
</script>

<Dialog bind:open title={t("report.title")} width={520} onsubmit={submit}>
  {#if sent}
    <p class="muted">{withLog ? t("report.sentWithLog") : t("report.sent")}</p>
  {:else}
    <label class="form-field">
      <span>{t("report.what")}</span>
      <textarea bind:value={description} rows="6" placeholder={t("report.whatHint")}></textarea>
    </label>
    {#if library.instances.length > 0}
      <div class="form-field">
        <span>{t("report.instance")}</span>
        <Select bind:value={instanceId} {options} />
      </div>
    {/if}
    <label class="toggle">
      <input type="checkbox" bind:checked={withLog}>
      <span>{t("report.withLog")}</span>
    </label>
    <p class="muted">{t("report.hint")}</p>
  {/if}
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#snippet actions()}
    {#if sent}
      <button type="button" class="primary" onclick={() => (open = false)}>
        {t("report.done")}
      </button>
    {:else}
      <button type="button" onclick={() => (open = false)}>{t("create.cancel")}</button>
      <button type="submit" class="primary" disabled={working || !description.trim()}>
        {t("report.send")}
      </button>
    {/if}
  {/snippet}
</Dialog>

<style>
  textarea {
    min-height: 120px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--field);
    color: var(--text);
    font: inherit;
    font-size: 14px;
    resize: vertical;
  }

  .muted {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.5;
  }

  .error {
    margin: 0;
    color: var(--danger);
  }
</style>
