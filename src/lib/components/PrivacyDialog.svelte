<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api } from "$lib/api";
  import { externalLinks } from "$lib/external-links";
  import { locale, t } from "$lib/i18n/index.svelte";
  import { renderMarkup } from "$lib/markup";
  import privacyEn from "../../../PRIVACY.md?raw";
  import privacyRu from "../../../PRIVACY.ru.md?raw";

  const titleId = $props.id();

  let dialog: HTMLDialogElement;
  let open = $state(false);
  let updated = $state(false);
  let saving = $state(false);

  const policy = $derived(renderMarkup(locale() === "ru" ? privacyRu : privacyEn, "markdown"));

  $effect(() => {
    api
      .privacyStatus()
      .then((status) => {
        updated = status.updated;
        open = !status.accepted;
      })
      .catch(() => {});
  });

  $effect(() => {
    if (open) {
      dialog.showModal();
    } else {
      dialog.close();
    }
  });

  async function accept() {
    saving = true;
    try {
      await api.acceptPrivacy();
      open = false;
    } finally {
      saving = false;
    }
  }
</script>

<dialog bind:this={dialog} aria-labelledby={titleId} oncancel={(event) => event.preventDefault()}>
  <h2 id={titleId}>{t("privacy.title")}</h2>
  <p class="lead">{updated ? t("privacy.updated") : t("privacy.lead")}</p>
  <div class="policy" {@attach externalLinks}>{@html policy}</div>
  <div class="actions">
    <button type="button" onclick={() => getCurrentWindow().close()}>{t("privacy.decline")}</button>
    <button type="button" class="primary" disabled={saving} onclick={accept}>
      {t("privacy.accept")}
    </button>
  </div>
</dialog>

<style>
  dialog {
    display: none;
    width: min(640px, calc(100vw - 32px));
    max-height: calc(100vh - 64px);
    padding: 24px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-solid);
    color: var(--text);
    box-shadow: 0 24px 64px rgb(0 0 0 / 0.24);
  }

  dialog[open] {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
    gap: 12px;
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.35);
    backdrop-filter: blur(6px);
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: var(--display-weight);
  }

  .lead {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  .policy {
    min-height: 0;
    overflow-y: auto;
    padding: 4px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--field);
    font-size: 13px;
    line-height: 1.5;
  }

  .policy :global(h1) {
    display: none;
  }

  .policy :global(h2) {
    margin: 16px 0 6px;
    font-size: 14px;
  }

  .policy :global(table) {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  .policy :global(th),
  .policy :global(td) {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    text-align: left;
    vertical-align: top;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .actions button {
    height: 36px;
    padding: 0 16px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--field);
    color: var(--text);
    font-weight: 600;
  }

  .actions .primary {
    border-color: transparent;
    background: var(--accent);
    color: var(--accent-text);
  }
</style>
