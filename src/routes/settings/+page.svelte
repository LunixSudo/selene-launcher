<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { api } from "$lib/api";
  import Segmented from "$lib/components/Segmented.svelte";
  import { type Locale, locale, setLocale, t } from "$lib/i18n/index.svelte";
  import {
    ACCENTS,
    type Accent,
    AFTER_LAUNCH,
    FONTS,
    STYLES,
    settings,
    THEMES,
  } from "$lib/state/settings.svelte";

  const LANGUAGES: { value: Locale; label: string }[] = [
    { value: "ru", label: "Русский" },
    { value: "en", label: "English" },
  ];

  let language = $state(locale());
  let version = $state("");

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
    getVersion()
      .then((current) => {
        version = current;
      })
      .catch(() => {});
  });
</script>

<section class="page">
  <header>
    <a class="back" href="/">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
      {t("instance.back")}
    </a>
    <h1>{t("settings.title")}</h1>
  </header>

  <div class="content">
    <section class="group">
      <h2>{t("settings.appearance")}</h2>

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
              title={t(`settings.accents.${name as Accent}`)}
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

    <section class="group">
      <h2>{t("settings.fonts")}</h2>
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

    <section class="group">
      <h2>{t("settings.game")}</h2>
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

    <section class="group">
      <h2>{t("settings.general")}</h2>
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

    <p class="about">Selene Launcher {version}</p>
  </div>
</section>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
    height: 100%;
  }

  header {
    display: grid;
    gap: 4px;
    padding-top: 8px;
  }

  .back {
    display: flex;
    align-items: center;
    gap: 4px;
    width: fit-content;
    color: var(--text-muted);
    font-size: 13px;
    text-decoration: none;
  }

  .back:hover {
    color: var(--text);
  }

  .back svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 28px;
    font-weight: var(--display-weight);
    letter-spacing: -0.02em;
  }

  .content {
    display: grid;
    align-content: start;
    gap: 20px;
    max-width: 720px;
    min-height: 0;
    overflow-y: auto;
  }

  .group {
    display: grid;
    gap: 2px;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    backdrop-filter: blur(24px) saturate(1.4);
  }

  h2 {
    margin: 0;
    padding: 14px 18px 6px;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
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

  .switch-row {
    cursor: pointer;
  }

  .switch {
    position: relative;
    flex: none;
    width: 40px;
    height: 22px;
    margin: 0;
    border-radius: 999px;
    background: var(--field);
    box-shadow: inset 0 0 0 1px var(--border);
    appearance: none;
    cursor: pointer;
    transition: background 150ms var(--ease);
  }

  .switch::before {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #ffffff;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.3);
    transition: transform 150ms var(--ease);
    content: "";
  }

  .switch:checked {
    background: var(--accent);
  }

  .switch:checked::before {
    transform: translateX(18px);
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

  .about {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
    text-align: center;
  }
</style>
