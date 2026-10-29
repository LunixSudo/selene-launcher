<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, type Instance, type MemoryInfo } from "$lib/api";
  import { coverOf } from "$lib/appearance";
  import { describeError } from "$lib/errors";
  import { locale, t } from "$lib/i18n/index.svelte";
  import { LOADER_NAMES } from "$lib/loaders";
  import { replaceInstance } from "$lib/state/instances.svelte";
  import { INSTANCE_BACKDROPS, type InstanceBackdrop, settings } from "$lib/state/settings.svelte";
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

  const DEFAULT_WINDOW = { width: 1280, height: 720 };

  let changingVersion = $state(false);

  let javaArgs = $derived(instance.javaArgs);

  async function saveLaunch(
    changes: Partial<Pick<Instance, "window" | "fullscreen" | "javaArgs" | "javaPath">>,
  ) {
    const next = {
      window: instance.window,
      fullscreen: instance.fullscreen,
      javaArgs,
      javaPath: instance.javaPath,
      ...changes,
    };
    try {
      replaceInstance(
        await api.setLaunchOptions(
          instance.id,
          next.window,
          next.fullscreen,
          next.javaArgs,
          next.javaPath,
        ),
      );
    } catch (err) {
      error = describeError(err);
    }
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

  function resize(side: "width" | "height", value: number) {
    const window = instance.window ?? DEFAULT_WINDOW;
    if (Number.isFinite(value) && value >= 200) {
      void saveLaunch({ window: { ...window, [side]: Math.round(value) } });
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

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</section>

<section class="group">
  <h2>{t("launchOptions.title")}</h2>

  <label class="row">
    <span>{t("launchOptions.fullscreen")}</span>
    <input
      class="switch"
      type="checkbox"
      checked={instance.fullscreen}
      onchange={(event) => saveLaunch({ fullscreen: event.currentTarget.checked })}
    >
  </label>

  <label class="row">
    <span>{t("launchOptions.customSize")}</span>
    <input
      class="switch"
      type="checkbox"
      checked={instance.window !== null}
      disabled={instance.fullscreen}
      onchange={(event) =>
        saveLaunch({ window: event.currentTarget.checked ? DEFAULT_WINDOW : null })}
    >
  </label>

  {#if instance.window && !instance.fullscreen}
    <div class="size">
      <label class="form-field">
        <span>{t("launchOptions.width")}</span>
        <input
          type="number"
          min="200"
          step="1"
          value={instance.window.width}
          onchange={(event) => resize("width", event.currentTarget.valueAsNumber)}
        >
      </label>
      <span class="times">×</span>
      <label class="form-field">
        <span>{t("launchOptions.height")}</span>
        <input
          type="number"
          min="200"
          step="1"
          value={instance.window.height}
          onchange={(event) => resize("height", event.currentTarget.valueAsNumber)}
        >
      </label>
    </div>
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

  .size {
    display: flex;
    align-items: flex-end;
    gap: 10px;
  }

  .times {
    padding-bottom: 8px;
    color: var(--text-muted);
  }

  .error {
    margin: 0;
    color: var(--danger);
  }
</style>
