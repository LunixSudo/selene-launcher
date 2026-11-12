<script lang="ts">
  import { flip } from "svelte/animate";
  import { fade, fly } from "svelte/transition";
  import { t } from "#lib/i18n/index.svelte.ts";
  import { settings } from "#lib/state/settings.svelte.ts";
  import { dismissToast, type Toast, toasts } from "#lib/state/toasts.svelte.ts";

  /** Long enough to read two lines; a pointer over the message holds it. */
  const SHOWN_MS = 5000;

  const duration = $derived(settings.motion ? 420 : 160);

  /** Starts the countdown, held while the pointer or focus is on the message. */
  function countdown(id: number) {
    return (node: HTMLElement) => {
      let timer: ReturnType<typeof setTimeout> | undefined;
      const start = () => {
        clearTimeout(timer);
        timer = setTimeout(() => dismissToast(id), SHOWN_MS);
      };
      const hold = () => clearTimeout(timer);
      start();
      node.addEventListener("pointerenter", hold);
      node.addEventListener("pointerleave", start);
      node.addEventListener("focusin", hold);
      node.addEventListener("focusout", start);
      return () => {
        clearTimeout(timer);
        node.removeEventListener("pointerenter", hold);
        node.removeEventListener("pointerleave", start);
        node.removeEventListener("focusin", hold);
        node.removeEventListener("focusout", start);
      };
    };
  }

  function act(entry: Toast) {
    entry.action?.run();
    dismissToast(entry.id);
  }
</script>

<section class="toaster" aria-live="polite">
  {#each toasts as entry (entry.id)}
    <div
      class="toast"
      role="status"
      animate:flip={{ duration }}
      in:fly={{ x: 48, duration, opacity: 0 }}
      out:fade={{ duration: 160 }}
      {@attach countdown(entry.id)}
    >
      <span class="mark" aria-hidden="true">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 12.5l4 4 8-9" /></svg>
      </span>
      <div class="text">
        <strong>{entry.title}</strong>
        <span>{entry.body}</span>
      </div>
      {#if entry.action}
        <button type="button" class="pill-button" onclick={() => act(entry)}>
          {entry.action.label}
        </button>
      {/if}
      <button
        type="button"
        class="icon-button"
        aria-label={t("window.close")}
        onclick={() => dismissToast(entry.id)}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7l10 10M17 7 7 17" /></svg>
      </button>
    </div>
  {/each}
</section>

<style>
  .toaster {
    position: fixed;
    right: 20px;
    bottom: 20px;
    z-index: 45;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 10px;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 12px;
    width: min(400px, calc(100vw - 40px));
    padding: 12px 10px 12px 12px;
    border-radius: var(--radius-lg);
    background: var(--surface-strong);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      0 0 0 1px var(--border),
      0 20px 40px -18px rgb(0 0 0 / 0.55);
    pointer-events: auto;
    backdrop-filter: var(--glass);
  }

  .mark {
    display: grid;
    flex: none;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 12px;
    background: var(--ok);
    color: var(--background);
  }

  svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .icon-button svg {
    width: 16px;
    height: 16px;
    stroke-width: 2;
  }

  .text {
    display: grid;
    flex: 1;
    gap: 2px;
    min-width: 0;
  }

  strong {
    font-size: 14.5px;
    font-weight: 800;
    overflow-wrap: anywhere;
  }

  .text span {
    color: var(--text-2);
    font-size: 13px;
    font-weight: 600;
  }
</style>
