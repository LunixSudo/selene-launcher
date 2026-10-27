<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const appWindow = getCurrentWindow();
</script>

<header class="titlebar" data-tauri-drag-region>
  <span class="title" data-tauri-drag-region>Selene Launcher</span>

  <div class="controls">
    <button type="button" aria-label="Свернуть" onclick={() => appWindow.minimize()}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1 5h8" /></svg>
    </button>
    <button type="button" aria-label="Развернуть" onclick={() => appWindow.toggleMaximize()}>
      <svg viewBox="0 0 10 10" aria-hidden="true">
        <rect x="1.5" y="1.5" width="7" height="7" rx="1" />
      </svg>
    </button>
    <button type="button" class="close" aria-label="Закрыть" onclick={() => appWindow.close()}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: var(--titlebar-height);
    padding-left: 16px;
  }

  .title {
    font-size: 12px;
    color: var(--text-muted);
  }

  .controls {
    display: flex;
    height: 100%;
  }

  .controls button {
    display: grid;
    place-items: center;
    width: 46px;
    height: 100%;
    border: none;
    background: transparent;
    transition: background 120ms var(--ease);
  }

  .controls button:hover {
    background: var(--surface-hover);
  }

  .controls .close:hover {
    background: var(--danger);
    color: #ffffff;
  }

  svg {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1;
  }
</style>
