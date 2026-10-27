<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import AccountMenu from "$lib/components/AccountMenu.svelte";
  import { t } from "$lib/i18n/index.svelte";

  const appWindow = getCurrentWindow();
</script>

<header class="titlebar" data-tauri-drag-region>
  <a class="brand" href="/">
    <span class="logo">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M19 14.5A7.5 7.5 0 1 1 9.5 5a6 6 0 0 0 9.5 9.5z" />
      </svg>
    </span>
    <span class="wordmark">Selene</span>
  </a>

  <div class="controls">
    <AccountMenu />
    <a class="settings" href="/settings" title={t("settings.title")}>
      <span class="label">{t("settings.title")}</span>
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <circle cx="8" cy="8" r="2.2" />
        <path
          d="M8 1.5v1.8M8 12.7v1.8M14.5 8h-1.8M3.3 8H1.5M12.6 3.4l-1.3 1.3M4.7 11.3l-1.3 1.3M12.6 12.6l-1.3-1.3M4.7 4.7L3.4 3.4"
        />
      </svg>
    </a>
    <button type="button" aria-label={t("window.minimize")} onclick={() => appWindow.minimize()}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1 5h8" /></svg>
    </button>
    <button
      type="button"
      aria-label={t("window.maximize")}
      onclick={() => appWindow.toggleMaximize()}
    >
      <svg viewBox="0 0 10 10" aria-hidden="true">
        <rect x="1.5" y="1.5" width="7" height="7" rx="1" />
      </svg>
    </button>
    <button
      type="button"
      class="close"
      aria-label={t("window.close")}
      onclick={() => appWindow.close()}
    >
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
    padding-left: 14px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    color: inherit;
    text-decoration: none;
  }

  .logo {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 7px;
    background: linear-gradient(135deg, #8fa2ff, var(--accent) 55%, #c45bff);
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .logo svg {
    width: 14px;
    height: 14px;
    fill: #ffffff;
    stroke: none;
  }

  .wordmark {
    font-family: var(--font-display);
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .controls {
    display: flex;
    align-items: center;
    height: 100%;
  }

  .controls > button,
  .settings {
    display: grid;
    place-items: center;
    width: 46px;
    height: 100%;
    border: none;
    background: transparent;
    transition: background 120ms var(--ease);
  }

  .controls > button:hover,
  .settings:hover {
    background: var(--surface-hover);
  }

  .settings {
    color: inherit;
  }

  .settings .label {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .controls .close:hover {
    background: var(--danger);
    color: #ffffff;
  }

  .controls svg {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1;
  }

  .settings svg {
    width: 15px;
    height: 15px;
    stroke-width: 1.3;
    stroke-linecap: round;
  }
</style>
