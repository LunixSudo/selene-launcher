<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, type Instance, type MemoryInfo } from "#lib/api.ts";
  import { coverOf } from "#lib/appearance.ts";
  import { describeError } from "#lib/errors.ts";
  import { locale, t } from "#lib/i18n/index.svelte.ts";
  import { hasFlag, JAVA_FLAGS, type JavaFlag, toggleFlag } from "#lib/java-args.ts";
  import { LOADER_NAMES } from "#lib/loaders.ts";
  import { replaceInstance } from "#lib/state/instances.svelte.ts";
  import {
    INSTANCE_BACKDROPS,
    type InstanceBackdrop,
    settings,
  } from "#lib/state/settings.svelte.ts";
  import BackdropPicker from "./BackdropPicker.svelte";
  import ChangeVersionDialog from "./ChangeVersionDialog.svelte";

  let { instance }: { instance: Instance } = $props();

  function setBackdrop(backdrop: InstanceBackdrop) {
    if (backdrop === "app") {
      delete settings.instanceBackdrops[instance.id];
    } else {
      settings.instanceBackdrops[instance.id] = backdrop;
    }
  }

  const STEP_MB = 512;
  const MIN_MB = 1024;

  let info = $state<MemoryInfo | null>(null);
  let error = $state<string | null>(null);
  let manual = $derived(instance.maxMemoryMb);

  $effect(() => {
    api
      .memoryInfo(instance.id)
      .then((value) => {
        info = value;
      })
      .catch((err) => {
        error = describeError(err);
      });
  });

  function gigabytes(megabytes: number): string {
    const count = new Intl.NumberFormat(locale(), { maximumFractionDigits: 1 }).format(
      megabytes / 1024,
    );
    return t("instance.gigabytes", { count });
  }

  let changingVersion = $state(false);

  let javaArgs = $derived(instance.javaArgs);

  async function saveLaunch(
    changes: Partial<Pick<Instance, "autoJavaArgs" | "javaArgs" | "javaPath">>,
  ) {
    const next = {
      autoJavaArgs: instance.autoJavaArgs,
      javaArgs,
      javaPath: instance.javaPath,
      ...changes,
    };
    try {
      replaceInstance(
        await api.setLaunchOptions(instance.id, next.autoJavaArgs, next.javaArgs, next.javaPath),
      );
    } catch (err) {
      error = describeError(err);
    }
  }

  const FLAGS = Object.keys(JAVA_FLAGS) as JavaFlag[];

  let flagQuery = $state("");
  let shownFlags = $derived.by(() => {
    const query = flagQuery.trim().toLowerCase();
    if (!query) {
      return FLAGS;
    }
    return FLAGS.filter((flag) =>
      [
        t(`launchOptions.flags.${flag}`),
        t(`launchOptions.flagHints.${flag}`),
        ...JAVA_FLAGS[flag],
      ].some((text) => text.toLowerCase().includes(query)),
    );
  });

  function setFlag(flag: JavaFlag, on: boolean) {
    javaArgs = toggleFlag(javaArgs, flag, on);
    void saveLaunch({});
  }

  async function chooseJava() {
    const path = await open({
      filters: [{ name: "Java", extensions: ["exe"] }],
      title: t("launchOptions.chooseJava"),
    });
    if (path) {
      await saveLaunch({ javaPath: path });
    }
  }

  async function save(maxMemoryMb: number | null) {
    try {
      replaceInstance(await api.setInstanceMemory(instance.id, maxMemoryMb));
    } catch (err) {
      error = describeError(err);
    }
  }
</script>

<section class="group">
  <h2>{t("changeVersion.current")}</h2>
  <div class="row">
    <span>
      {instance.version}
      {#if instance.loader}
        <span class="hint">· {LOADER_NAMES[instance.loader.loader]} {instance.loader.version}</span>
      {/if}
    </span>
    <button type="button" class="pill-button" onclick={() => (changingVersion = true)}>
      {t("changeVersion.change")}
    </button>
  </div>
</section>

<section class="group">
  <h2>{t("launchOptions.javaTitle")}</h2>

  {#if info}
    {@const recommended = info.recommended}
    <label class="row">
      <span>{t("instance.memoryAuto")}</span>
      <input
        class="switch"
        type="checkbox"
        checked={manual === null}
        onchange={(event) => save(event.currentTarget.checked ? null : recommended)}
      >
    </label>

    {#if manual === null}
      <p class="hint">
        {t("instance.memoryAutoHint", {
          amount: gigabytes(info.recommended),
          total: gigabytes(info.total),
        })}
      </p>
    {:else}
      <div class="slider">
        <input
          type="range"
          min={MIN_MB}
          max={info.max}
          step={STEP_MB}
          value={manual}
          aria-label={t("instance.memory")}
          oninput={(event) => (manual = Number(event.currentTarget.value))}
          onchange={(event) => save(Number(event.currentTarget.value))}
        >
        <output>{gigabytes(manual)}</output>
      </div>
      <p class="hint">{t("instance.memoryManualHint")}</p>
    {/if}
  {/if}

  <div class="row">
    <div class="java">
      <span>{t("launchOptions.java")}</span>
      <span class="hint path">{instance.javaPath ?? t("launchOptions.javaAuto")}</span>
    </div>
    {#if instance.javaPath}
      <button type="button" class="pill-button" onclick={() => saveLaunch({ javaPath: null })}>
        {t("launchOptions.javaReset")}
      </button>
    {/if}
    <button type="button" class="pill-button" onclick={chooseJava}>
      {t("launchOptions.chooseJava")}
    </button>
  </div>

  <label class="row">
    <span>{t("launchOptions.autoArgs")}</span>
    <input
      class="switch"
      type="checkbox"
      checked={instance.autoJavaArgs}
      onchange={(event) => saveLaunch({ autoJavaArgs: event.currentTarget.checked })}
    >
  </label>

  {#if instance.autoJavaArgs}
    <p class="hint">{t("launchOptions.autoArgsHint")}</p>
  {:else}
    <label class="form-field">
      <span>{t("launchOptions.javaArgs")}</span>
      <input
        bind:value={javaArgs}
        spellcheck="false"
        placeholder="-XX:+UseZGC"
        onchange={() => saveLaunch({})}
      >
    </label>
    <p class="hint">{t("launchOptions.javaArgsHint")}</p>
    <input
      class="search"
      type="search"
      bind:value={flagQuery}
      spellcheck="false"
      placeholder={t("launchOptions.flagSearch")}
      aria-label={t("launchOptions.flagSearch")}
    >
    <div class="flags">
      {#each shownFlags as flag (flag)}
        <label class="flag">
          <input
            type="checkbox"
            checked={hasFlag(javaArgs, flag)}
            onchange={(event) => setFlag(flag, event.currentTarget.checked)}
          >
          <span>
            {t(`launchOptions.flags.${flag}`)}
            <span class="hint">{t(`launchOptions.flagHints.${flag}`)}</span>
            <code>{JAVA_FLAGS[flag].join(" ")}</code>
          </span>
        </label>
      {:else}
        <p class="hint">{t("launchOptions.flagsEmpty")}</p>
      {/each}
    </div>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</section>

<section class="group">
  <h2>{t("instance.backdrop")}</h2>
  <p class="hint">{t("instance.backdropHint")}</p>
  <BackdropPicker
    bind:value={() => settings.instanceBackdrops[instance.id] ?? "app", setBackdrop}
    options={INSTANCE_BACKDROPS}
    name="instance-backdrop"
    label={t("instance.backdrop")}
    cover={coverOf(instance.id, instance.cover)}
  />
</section>

<ChangeVersionDialog {instance} bind:open={changingVersion} />

<style>
  .group {
    display: grid;
    gap: 10px;
    max-width: 560px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .group + .group {
    margin-top: 12px;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: var(--display-weight);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  .form-field {
    flex: 1;
  }

  .slider {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .slider input {
    flex: 1;
    accent-color: var(--accent);
  }

  output {
    min-width: 56px;
    font-weight: 600;
    text-align: right;
  }

  .row {
    gap: 8px;
  }

  .flags {
    display: grid;
    gap: 10px;
    max-height: 280px;
    padding: 4px 10px 4px 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .search {
    width: 100%;
  }

  .flag code {
    color: var(--text-muted);
    font-size: 11px;
    word-break: break-all;
  }

  .flag {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    cursor: pointer;
  }

  .flag input {
    margin-top: 3px;
    accent-color: var(--accent);
  }

  .flag > span {
    display: grid;
  }

  .java {
    display: grid;
    flex: 1;
    min-width: 0;
  }

  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .error {
    margin: 0;
    color: var(--danger);
  }
</style>
