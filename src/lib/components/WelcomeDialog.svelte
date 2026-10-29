<script lang="ts">
  import { t } from "$lib/i18n/index.svelte";
  import { BEAUTIFUL, FAST, settings } from "$lib/state/settings.svelte";

  let { open }: { open: boolean } = $props();

  const titleId = $props.id();

  let dialog: HTMLDialogElement;

  $effect(() => {
    if (open) {
      dialog.showModal();
    } else {
      dialog.close();
    }
  });

  function choose(beautiful: boolean) {
    Object.assign(settings, beautiful ? BEAUTIFUL : FAST);
    settings.setupDone = true;
  }
</script>

<dialog bind:this={dialog} aria-labelledby={titleId} oncancel={(event) => event.preventDefault()}>
  <h2 id={titleId}>{t("welcome.title")}</h2>
  <p class="lead">{t("welcome.lead")}</p>
  <div class="choices">
    <button type="button" class="choice fast" onclick={() => choose(false)}>
      <span class="art" aria-hidden="true">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M13 3L5 13.5h6L10 21l8-10.5h-6L13 3z" />
        </svg>
      </span>
      <strong>{t("welcome.fast")}</strong>
      <span>{t("welcome.fastHint")}</span>
    </button>
    <button type="button" class="choice beautiful" onclick={() => choose(true)}>
      <span class="art" aria-hidden="true">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M12 3l1.8 4.6L18.5 9l-4.7 1.6L12 15l-1.8-4.4L5.5 9l4.7-1.4L12 3zM18 15l.9 2.1L21 18l-2.1.9L18 21l-.9-2.1L15 18l2.1-.9L18 15z"
          />
        </svg>
      </span>
      <strong>{t("welcome.beautiful")}</strong>
      <span>{t("welcome.beautifulHint")}</span>
    </button>
  </div>
  <p class="later">{t("welcome.later")}</p>
</dialog>

<style>
  dialog {
    width: min(560px, calc(100vw - 32px));
    padding: 28px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-solid);
    color: var(--text);
    box-shadow: 0 24px 64px rgb(0 0 0 / 0.3);
  }

  dialog[open] {
    animation: pop 320ms var(--ease);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.35);
    backdrop-filter: blur(8px);
  }

  h2 {
    margin: 0 0 6px;
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: var(--display-weight);
    text-align: center;
  }

  .lead,
  .later {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }

  .choices {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    margin: 22px 0 16px;
  }

  .choice {
    display: grid;
    justify-items: center;
    gap: 8px;
    padding: 22px 16px 18px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--field);
    text-align: center;
    transition:
      border-color 200ms var(--ease),
      transform 300ms var(--ease),
      box-shadow 300ms var(--ease);
  }

  .choice:hover {
    border-color: var(--accent);
    transform: translateY(-3px);
    box-shadow: 0 14px 32px color-mix(in srgb, var(--accent) 25%, transparent);
  }

  .choice strong {
    font-family: var(--font-display);
    font-size: 16px;
  }

  .choice span:last-child {
    color: var(--text-muted);
    font-size: 12px;
  }

  .art {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: 18px;
    background: var(--surface);
  }

  .beautiful .art {
    background: linear-gradient(135deg, #34e0d2, var(--accent) 55%, #ff4fd8);
    color: #ffffff;
  }

  .art svg {
    width: 26px;
    height: 26px;
    fill: currentColor;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.94);
    }
  }
</style>
