<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import logo from "$lib/assets/logo.png";
  import AccountMenu from "$lib/components/AccountMenu.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { installUpdate, launcherUpdate } from "$lib/state/update.svelte";
  import { tooltip } from "$lib/tooltip";

  const appWindow = getCurrentWindow();
</script>

<header class="titlebar" data-tauri-drag-region>
  <a class="brand" href="/">
    <img class="logo" src={logo} alt="" width="22" height="22">
    <span class="wordmark">Selene</span>
  </a>

  <div class="controls">
    {#if launcherUpdate.available}
      <button
        type="button"
        class="update"
        disabled={launcherUpdate.installing}
        {@attach tooltip(launcherUpdate.error ?? launcherUpdate.available.notes)}
        onclick={installUpdate}
      >
        {launcherUpdate.installing
          ? t("update.installing")
          : t("update.available", { version: launcherUpdate.available.version })}
      </button>
    {/if}
    <AccountMenu />
    <a class="settings" href="/settings" {@attach tooltip(t("settings.title"))}>
      <span class="label">{t("settings.title")}</span>
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M4 7h9M17 7h3M4 17h3M11 17h9" />
        <circle cx="15" cy="7" r="2" />
        <circle cx="9" cy="17" r="2" />
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
    width: 22px;
    height: 22px;
    filter: drop-shadow(0 2px 6px rgb(109 59 222 / 0.45));
  }

  .wordmark {
    font-family: var(--font-display);
    font-size: 13px;
    font-weight: var(--display-weight);
    letter-spacing: 0.02em;
  }

  .update {
    width: auto;
    height: 26px;
    margin-right: 6px;
    padding: 0 12px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-text);
    font-size: 12px;
    font-weight: 600;
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

  .controls > button:hover {
    background: var(--surface-hover);
  }

  .settings {
    width: 32px;
    height: 32px;
    margin: 0 6px 0 4px;
    border-radius: 999px;
    color: inherit;
  }

  .settings:hover {
    background: var(--surface-hover);
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
    width: 16px;
    height: 16px;
    stroke-width: 1.6;
    stroke-linecap: round;
  }
</style>
