<script lang="ts">
  import type { InstallPreview } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";
  import { scrollFade } from "$lib/scroll-fade";
  import Dialog from "./Dialog.svelte";

  let {
    preview,
    open = $bindable(),
    onconfirm,
    oncancel,
  }: {
    preview: InstallPreview;
    open: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();

  let confirmed = false;

  $effect(() => {
    if (open) {
      confirmed = false;
    } else if (!confirmed) {
      oncancel();
    }
  });

  const requested = $derived(preview.mods.filter((planned) => planned.requested));
  const dependencies = $derived(preview.mods.filter((planned) => !planned.requested));

  function confirm() {
    confirmed = true;
    open = false;
    onconfirm();
  }
</script>

<Dialog bind:open title={t("install.title")} onsubmit={confirm}>
  {#if requested.length > 0}
    <section>
      <h3>{t("install.requested", { count: requested.length })}</h3>
      <ul {@attach scrollFade}>
        {#each requested as planned, index (index)}
          <li>{planned.title}</li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if dependencies.length > 0}
    <section>
      <h3>{t("install.dependencies", { count: dependencies.length })}</h3>
      <p class="hint">{t("install.dependenciesHint")}</p>
      <ul {@attach scrollFade}>
        {#each dependencies as planned, index (index)}
          <li class="dependency">{planned.title}</li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if preview.manual.length > 0}
    <section>
      <h3>{t("install.manual")}</h3>
      <p class="hint">{t("instance.manualDownload")}</p>
      <ul {@attach scrollFade}>
        {#each preview.manual as download (download.url)}
          <li class="manual">{download.title}</li>
        {/each}
      </ul>
    </section>
  {/if}

  {#snippet actions()}
    <button type="button" onclick={() => (open = false)}>{t("create.cancel")}</button>
    <button type="submit" class="primary">
      {t("install.confirm", { count: preview.mods.length })}
    </button>
  {/snippet}
</Dialog>

<style>
  section {
    display: grid;
    gap: 6px;
  }

  h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
  }

  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  ul {
    display: grid;
    gap: 4px;
    max-height: 180px;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    background: var(--field);
    font-size: 13px;
  }

  li::before {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    content: "";
  }

  .dependency::before {
    background: var(--text-muted);
  }

  .manual::before {
    background: var(--danger);
  }
</style>
