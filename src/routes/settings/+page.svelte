<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type LauncherConfig } from "$lib/api";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import Segmented from "$lib/components/Segmented.svelte";
  import Select from "$lib/components/Select.svelte";
  import { describeError } from "$lib/errors";
  import { type Locale, locale, setLocale, t } from "$lib/i18n/index.svelte";
  import { scrollFade } from "$lib/scroll-fade";
  import {
    ACCENTS,
    type Accent,
    AFTER_LAUNCH,
    FONTS,
    STYLES,
    settings,
    THEMES,
  } from "$lib/state/settings.svelte";
  import { checkForUpdate, installUpdate, launcherUpdate } from "$lib/state/update.svelte";
  import { tooltip } from "$lib/tooltip";

  const LANGUAGES: { value: Locale; label: string }[] = [
    { value: "ru", label: "Русский" },
    { value: "en", label: "English" },
  ];

  const REPOSITORY_URL = "https://github.com/LunixSudo/selene-launcher/blob/main";
  const PARALLEL = ["auto", "2", "4", "8", "16"];
  const SPEEDS_MB = ["unlimited", "1", "2", "5", "10", "25"];

  let language = $state(locale());
  let version = $state("");
  let config = $state<LauncherConfig | null>(null);
  let saveError = $state<string | null>(null);
  let canUninstall = $state(false);
  let confirmingUninstall = $state(false);

  const parallelOptions = $derived(
    PARALLEL.map((value) => ({
      value,
      label: value === "auto" ? t("settings.auto") : value,
    })),
  );
  const speedOptions = $derived(
    SPEEDS_MB.map((value) => ({
      value,
      label:
        value === "unlimited"
          ? t("settings.unlimited")
          : t("settings.megabytesPerSecond", { count: value }),
    })),
  );

  $effect(() => {
    api
      .config()
      .then((loaded) => {
        config = loaded;
      })
      .catch(() => {});
  });

  function saveConfig(changes: Partial<LauncherConfig>) {
    if (!config) {
      return;
    }
    config = { ...config, ...changes };
    api
      .setConfig(config)
      .then(() => (saveError = null))
      .catch((err) => (saveError = describeError(err)));
  }

  const themes = $derived(THEMES.map((value) => ({ value, label: t(`settings.themes.${value}`) })));
  const afterLaunch = $derived(
    AFTER_LAUNCH.map((value) => ({ value, label: t(`settings.afterLaunchOptions.${value}`) })),
  );

  $effect(() => {
    if (language !== locale()) {
      setLocale(language);
    }
  });

  $effect(() => {
    api
      .canUninstall()
      .then((available) => {
        canUninstall = available;
      })
      .catch(() => {});
  });

  $effect(() => {
    getVersion()
      .then((current) => {
        version = current;
      })
      .catch(() => {});
  });
</script>

<section class="page">
  <header>
    <a class="back-link" href="/">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
      {t("instance.back")}
    </a>
    <h1 class="page-title">{t("settings.title")}</h1>
  </header>

  {#if saveError}
    <p class="save-error" role="alert">{saveError}</p>
  {/if}

  <div class="content" {@attach scrollFade}>
    <section class="panel">
      <h2 class="panel-title">{t("settings.appearance")}</h2>

      <div class="row">
        <div>
          <h3>{t("settings.theme")}</h3>
        </div>
        <Segmented
          bind:value={settings.theme}
          options={themes}
          name="theme"
          label={t("settings.theme")}
        />
      </div>

      <div class="row column">
        <div>
          <h3>{t("settings.style")}</h3>
          <p>{t("settings.styleHint")}</p>
        </div>
        <div class="styles" role="radiogroup" aria-label={t("settings.style")}>
          {#each STYLES as style (style)}
            <label class="style {style}">
              <input type="radio" name="style" value={style} bind:group={settings.style}>
              <span class="preview" aria-hidden="true">
                <span></span>
                <span></span>
              </span>
              <span class="name">{t(`settings.styles.${style}`)}</span>
            </label>
          {/each}
        </div>
      </div>

      <div class="row">
        <div>
          <h3>{t("settings.accent")}</h3>
        </div>
        <div class="accents" role="radiogroup" aria-label={t("settings.accent")}>
          {#each Object.entries(ACCENTS) as [name, color] (name)}
            <label
              class="accent"
              style:--swatch={color}
              {@attach tooltip(t(`settings.accents.${name as Accent}`))}
            >
              <input
                type="radio"
                name="accent"
                value={name}
                bind:group={settings.accent}
                aria-label={t(`settings.accents.${name as Accent}`)}
              >
            </label>
          {/each}
        </div>
      </div>
    </section>

    <section class="panel">
      <h2 class="panel-title">{t("settings.fonts")}</h2>
      <div class="row column">
        <div class="fonts" role="radiogroup" aria-label={t("settings.fonts")}>
          {#each FONTS as font (font)}
            <label class="font" data-font={font}>
              <input type="radio" name="font" value={font} bind:group={settings.font}>
              <span class="sample" aria-hidden="true">Аа</span>
              <span class="font-name">{t(`settings.fontNames.${font}`)}</span>
            </label>
          {/each}
        </div>
      </div>
    </section>

    <section class="panel">
      <h2 class="panel-title">{t("settings.game")}</h2>
      <div class="row">
        <div>
          <h3>{t("settings.afterLaunch")}</h3>
          <p>{t("settings.afterLaunchHint")}</p>
        </div>
        <Segmented
          bind:value={settings.afterLaunch}
          options={afterLaunch}
          name="after-launch"
          label={t("settings.afterLaunch")}
        />
      </div>
      {#if settings.afterLaunch === "minimize"}
        <label class="row switch-row">
          <div>
            <h3>{t("settings.restoreAfterGame")}</h3>
            <p>{t("settings.restoreAfterGameHint")}</p>
          </div>
          <input class="switch" type="checkbox" bind:checked={settings.restoreAfterGame}>
        </label>
      {/if}
      {#if config}
        <label class="row switch-row">
          <div>
            <h3>{t("settings.autoUpdateMods")}</h3>
            <p>{t("settings.autoUpdateModsHint")}</p>
          </div>
          <input
            class="switch"
            type="checkbox"
            checked={config.autoUpdateMods}
            onchange={(event) => saveConfig({ autoUpdateMods: event.currentTarget.checked })}
          >
        </label>
      {/if}
      <div class="row">
        <div>
          <h3>{t("settings.dataFolder")}</h3>
          <p>{t("settings.dataFolderHint")}</p>
        </div>
        <button type="button" class="button" onclick={() => api.openDataFolder()}>
          {t("instances.openFolder")}
        </button>
      </div>
    </section>

    {#if config}
      {@const current = config}
      <section class="panel">
        <h2 class="panel-title">{t("settings.downloads")}</h2>
        <div class="row">
          <div>
            <h3>{t("settings.maxDownloads")}</h3>
            <p>{t("settings.maxDownloadsHint")}</p>
          </div>
          <div class="select">
            <Select
              bind:value={
                () => String(current.maxDownloads ?? "auto"),
                (value) => saveConfig({ maxDownloads: value === "auto" ? null : Number(value) })
              }
              options={parallelOptions}
            />
          </div>
        </div>
        <div class="row">
          <div>
            <h3>{t("settings.speedLimit")}</h3>
            <p>{t("settings.speedLimitHint")}</p>
          </div>
          <div class="select">
            <Select
              bind:value={
                () =>
                  current.speedLimitKib === null
                    ? "unlimited"
                    : String(current.speedLimitKib / 1024),
                (value) =>
    saveConfig({
      speedLimitKib: value === "unlimited" ? null : Number(value) * 1024,
    })
              }
              options={speedOptions}
            />
          </div>
        </div>
      </section>
    {/if}

    <section class="panel">
      <h2 class="panel-title">{t("settings.general")}</h2>
      <label class="row switch-row">
        <div>
          <h3>{t("settings.notifications")}</h3>
          <p>{t("settings.notificationsHint")}</p>
        </div>
        <input class="switch" type="checkbox" bind:checked={settings.notifications}>
      </label>
      <label class="row switch-row">
        <div>
          <h3>{t("settings.discordStatus")}</h3>
          <p>{t("settings.discordStatusHint")}</p>
        </div>
        <input class="switch" type="checkbox" bind:checked={settings.discordStatus}>
      </label>
      <div class="row">
        <div>
          <h3>{t("settings.language")}</h3>
        </div>
        <Segmented
          bind:value={language}
          options={LANGUAGES}
          name="language"
          label={t("settings.language")}
        />
      </div>
    </section>

    <section class="panel">
      <h2 class="panel-title">{t("update.title")}</h2>
      <label class="row switch-row">
        <div>
          <h3>{t("update.checkOnStart")}</h3>
          <p>{t("update.checkOnStartHint")}</p>
        </div>
        <input class="switch" type="checkbox" bind:checked={settings.checkUpdates}>
      </label>
      <div class="row">
        <div>
          <h3>Selene Launcher {version}</h3>
          {#if launcherUpdate.error}
            <p class="error">{launcherUpdate.error}</p>
          {:else if launcherUpdate.checked && !launcherUpdate.available}
            <p>{t("update.upToDate")}</p>
          {/if}
        </div>
        {#if launcherUpdate.available}
          <button
            type="button"
            class="button primary"
            disabled={launcherUpdate.installing}
            onclick={installUpdate}
          >
            {launcherUpdate.installing
              ? t("update.installing")
              : t("update.available", { version: launcherUpdate.available.version })}
          </button>
        {:else}
          <button
            type="button"
            class="button"
            disabled={launcherUpdate.checking}
            onclick={checkForUpdate}
          >
            {launcherUpdate.checking ? t("update.checking") : t("update.checkNow")}
          </button>
        {/if}
      </div>
    </section>

    {#if canUninstall}
      <section class="panel">
        <div class="row">
          <div>
            <h3>{t("settings.uninstall")}</h3>
            <p>{t("settings.uninstallHint")}</p>
          </div>
          <button type="button" class="button danger" onclick={() => (confirmingUninstall = true)}>
            {t("settings.uninstallButton")}
          </button>
        </div>
      </section>
    {/if}

    <p class="legal">
      {t("settings.legal")}
      <button
        type="button"
        class="link"
        onclick={() =>
          openUrl(`${REPOSITORY_URL}/${locale() === "ru" ? "PRIVACY.ru.md" : "PRIVACY.md"}`)}
      >
        {t("settings.privacy")}
      </button>
      ·
      <a class="link" href="/licenses">{t("licenses.title")}</a>
    </p>
  </div>
</section>

<ConfirmDialog
  bind:open={confirmingUninstall}
  title={t("settings.uninstallTitle")}
  message={t("settings.uninstallMessage")}
  confirmLabel={t("settings.uninstallButton")}
  onconfirm={() => api.uninstallLauncher()}
/>

<style>
  header {
    display: grid;
    gap: 4px;
    padding-top: 8px;
  }

  .button.danger {
    color: var(--danger);
  }

  .button.danger:hover:enabled {
    background: var(--danger);
    color: #ffffff;
  }

  .legal {
    margin: 0 0 8px;
    padding: 0 18px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: inherit;
    cursor: pointer;
  }

  .content {
    display: grid;
    grid-auto-rows: max-content;
    align-content: start;
    gap: 20px;
    min-height: 0;
    margin-right: -12px;
    padding-right: 12px;
    padding-bottom: 16px;
    overflow-y: auto;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 18px;
  }

  .row + .row {
    border-top: 1px solid var(--border);
  }

  .row.column {
    flex-direction: column;
    align-items: stretch;
  }

  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .row p {
    margin: 2px 0 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  .row p.error {
    color: var(--danger);
  }

  .save-error {
    margin: 0;
    color: var(--danger);
    font-size: 13px;
  }

  input[type="radio"] {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .styles {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }

  .style {
    display: grid;
    gap: 8px;
    cursor: pointer;
  }

  .preview {
    position: relative;
    display: block;
    height: 84px;
    overflow: hidden;
    border: 2px solid var(--border);
    border-radius: var(--radius-md);
    background: linear-gradient(135deg, #5b7cff, #c45bff 60%, #ff8a5b);
    transition: border-color 150ms var(--ease);
  }

  .preview span {
    position: absolute;
    border-radius: 8px;
  }

  .preview span:first-child {
    inset: 12px 40% 12px 12px;
  }

  .preview span:last-child {
    inset: 12px 12px 12px 64%;
  }

  .glass .preview span {
    border: 1px solid rgb(255 255 255 / 0.4);
    background: rgb(255 255 255 / 0.25);
    backdrop-filter: blur(6px);
  }

  .mica .preview {
    background: linear-gradient(135deg, #8b97c9, #a88fc4);
  }

  .mica .preview span {
    background: rgb(255 255 255 / 0.55);
  }

  .solid .preview {
    background: var(--background);
  }

  .solid .preview span {
    background: var(--surface-solid);
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.15);
  }

  .style:has(input:checked) .preview {
    border-color: var(--accent);
  }

  .style:has(input:focus-visible) .preview {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .name {
    font-size: 13px;
    text-align: center;
  }

  .style:has(input:checked) .name {
    font-weight: 600;
  }

  .accents {
    display: flex;
    gap: 10px;
  }

  .accent {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--swatch);
    cursor: pointer;
    transition: box-shadow 150ms var(--ease);
  }

  .accent:has(input:checked) {
    box-shadow:
      0 0 0 2px var(--surface-solid),
      0 0 0 4px var(--swatch);
  }

  .accent:has(input:focus-visible) {
    outline: 2px solid var(--text);
    outline-offset: 4px;
  }

  .fonts {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
  }

  .font {
    display: grid;
    justify-items: center;
    gap: 4px;
    padding: 12px 8px 10px;
    border: 2px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: border-color 150ms var(--ease);
  }

  .font:has(input:checked) {
    border-color: var(--accent);
  }

  .font:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .sample {
    font-size: 30px;
    line-height: 1.1;
  }

  .font[data-font="rounded"] .sample {
    font-family: "Nunito Variable", sans-serif;
    font-weight: 850;
  }

  .font[data-font="modern"] .sample {
    font-family: "Unbounded Variable", sans-serif;
    font-weight: 600;
  }

  .font[data-font="rubik"] .sample {
    font-family: "Rubik Variable", sans-serif;
    font-weight: 700;
  }

  .font[data-font="system"] .sample {
    font-family: "Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif;
    font-weight: 600;
  }

  .font-name {
    color: var(--text-muted);
    font-size: 12px;
  }

  .select {
    flex: none;
    width: 200px;
  }

  .switch-row {
    cursor: pointer;
  }

  .button {
    flex: none;
    height: 34px;
    padding: 0 16px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--field);
    font-weight: 600;
  }

  .button:hover {
    background: var(--surface-hover);
  }

  .button.primary {
    border-color: transparent;
    background: var(--accent);
    color: var(--accent-text);
  }
</style>
