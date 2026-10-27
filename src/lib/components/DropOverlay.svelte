<script lang="ts">
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { t } from "$lib/i18n/index.svelte";
  import { importPack } from "$lib/state/instances.svelte";

  let dragging = $state(false);

  $effect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "over") {
        dragging = true;
      } else if (payload.type === "drop") {
        dragging = false;
        for (const path of payload.paths) {
          if (/\.(mrpack|zip)$/i.test(path)) {
            void importPack(path);
          }
        }
      } else if (payload.type === "leave") {
        dragging = false;
      }
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  });
</script>

{#if dragging}
  <div class="overlay">
    <div class="target">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v12M6 10l6 6 6-6M5 20h14" /></svg>
      <p>{t("instances.dropHint")}</p>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: var(--titlebar-height) 0 0;
    z-index: 100;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgb(0 0 0 / 0.18);
    backdrop-filter: blur(8px);
    animation: fade 150ms var(--ease);
    pointer-events: none;
  }

  .target {
    display: grid;
    place-items: center;
    gap: 12px;
    width: 100%;
    height: 100%;
    border: 2px dashed var(--accent);
    border-radius: var(--radius-lg);
    background: var(--surface);
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 600;
  }

  svg {
    width: 40px;
    height: 40px;
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  p {
    margin: 0;
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }
</style>
