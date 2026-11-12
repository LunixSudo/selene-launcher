<script lang="ts">
  import { open as openFile } from "@tauri-apps/plugin-dialog";
  import { tick } from "svelte";
  import {
    api,
    isCommandError,
    type Loader,
    type LoaderRelease,
    type VersionSummary,
  } from "#lib/api.ts";
  import { COVERS } from "#lib/appearance.ts";
  import MoonPhase from "#lib/components/MoonPhase.svelte";
  import PlayerHead from "#lib/components/PlayerHead.svelte";
  import Select from "#lib/components/Select.svelte";
  import { openModal } from "#lib/dialog.ts";
  import { detectGpu } from "#lib/gpu.ts";
  import {
    LANGUAGES,
    type Locale,
    locale,
    type MessageKey,
    setLocale,
    t,
  } from "#lib/i18n/index.svelte.ts";
  import { LOADER_NAMES, LOADERS, preferredRelease } from "#lib/loaders.ts";
  import {
    activeAccount,
    cancelSignIn,
    loadAccounts,
    session,
    signIn,
  } from "#lib/state/accounts.svelte.ts";
  import { createInstance, importPack } from "#lib/state/instances.svelte.ts";
  import { BEAUTIFUL, FAST, matchesLook, settings } from "#lib/state/settings.svelte.ts";
  import { toast } from "#lib/state/toasts.svelte.ts";
  import { goto } from "$app/navigation";

  let { open }: { open: boolean } = $props();

  type Step = "look" | "account" | "pack";
  type Pick = "vanilla" | "fps" | "pack" | "file";

  const STEPS: Step[] = ["look", "account", "pack"];
  /** Versions people play the most, offered next to the newest one. */
  const POPULAR = ["1.21.1", "1.20.1", "1.16.5", "1.12.2"];
  const VERSION_HINTS: Record<string, "hint121" | "hint120" | "hint116" | "hint112"> = {
    "1.21.1": "hint121",
    "1.20.1": "hint120",
    "1.16.5": "hint116",
    "1.12.2": "hint112",
  };
  /** A release this young has few mods yet. */
  const FRESH_DAYS = 60;
  /** Modrinth projects that make the game run faster, by loader. Missing ones are skipped. */
  const FPS_MODS: Record<Loader, string[]> = {
    fabric: ["sodium", "lithium", "iris", "ferrite-core"],
    quilt: ["sodium", "lithium", "iris", "ferrite-core"],
    neoforge: ["sodium", "lithium", "iris", "ferrite-core"],
    forge: ["embeddium", "oculus", "ferrite-core", "modernfix"],
  };
  const PICKS: { value: Pick; cover: string | null }[] = [
    { value: "vanilla", cover: "plains" },
    { value: "fps", cover: "snowfall" },
    { value: "pack", cover: "dusk" },
    { value: "file", cover: null },
  ];

  const PICK_TEXT = {
    vanilla: { title: "welcome.vanilla", hint: "welcome.vanillaHint" },
    fps: { title: "welcome.fps", hint: "welcome.fpsHint" },
    pack: { title: "welcome.modpack", hint: "welcome.modpackHint" },
    file: { title: "welcome.file", hint: "welcome.fileHint" },
  } as const satisfies Record<Pick, { title: MessageKey; hint: MessageKey }>;

  const titleId = $props.id();
  const gpu = detectGpu();

  let dialog: HTMLDialogElement;
  let step = $state<Step>("look");
  let language = $state<Locale>(locale());
  let pick = $state<Pick>("vanilla");
  let versions = $state.raw<VersionSummary[] | null>(null);
  let version = $state("");
  let chosenLoader = $state<Loader>("fabric");
  let loaders = $state.raw<Partial<Record<Loader, LoaderRelease[]>> | null>(null);
  let error = $state<string | null>(null);
  let working = $state(false);

  const beautiful = $derived(matchesLook(BEAUTIFUL));
  const account = $derived(activeAccount());
  const releases = $derived((versions ?? []).filter((entry) => entry.kind === "release"));
  const latest = $derived(releases[0]);
  const chips = $derived(
    latest
      ? [
          latest.id,
          ...POPULAR.filter((id) => id !== latest.id && releases.some((r) => r.id === id)),
        ]
      : [],
  );
  const other = $derived(Boolean(version) && !chips.includes(version));
  const versionHint = $derived.by(() => {
    if (version === latest?.id) {
      const days = (Date.now() - new Date(latest.releaseTime).getTime()) / 86_400_000;
      return days < FRESH_DAYS ? t("welcome.versionFresh") : "";
    }
    return VERSION_HINTS[version] ? t(`welcome.${VERSION_HINTS[version]}`) : "";
  });
  const available = $derived(LOADERS.filter((loader) => (loaders?.[loader]?.length ?? 0) > 0));
  const loader = $derived<Loader | null>(
    available.includes(chosenLoader) ? chosenLoader : (available[0] ?? null),
  );
  const loaderHint = $derived(
    loader === "forge"
      ? t("welcome.loaderForge")
      : loader === "quilt"
        ? t("welcome.loaderQuilt")
        : t("welcome.loaderSodium"),
  );
  const needsVersion = $derived(pick === "vanilla" || pick === "fps");
  const ready = $derived(
    !working && (!needsVersion || (Boolean(version) && (pick === "vanilla" || loader !== null))),
  );

  $effect(() => {
    if (open) {
      openModal(dialog);
      // Weak or unknown graphics start on the light look; it is a click away either way.
      Object.assign(settings, gpu && !gpu.weak ? BEAUTIFUL : FAST);
      void loadAccounts().catch(() => {});
    } else {
      dialog.close();
    }
  });

  // The button that moved on is gone, so keyboard focus goes to the next step's main action.
  $effect(() => {
    if (step !== "look") {
      void tick().then(() => dialog.querySelector<HTMLElement>(".go, .microsoft")?.focus());
    }
  });

  $effect(() => {
    if (step === "pack" && !versions) {
      api
        .versions()
        .then((list) => {
          versions = list;
          version ||= list.find((entry) => entry.kind === "release")?.id ?? "";
        })
        .catch((err) => (error = describe(err)));
    }
  });

  // Which loaders have builds for the chosen version decides what "More FPS" can offer.
  $effect(() => {
    const gameVersion = version;
    loaders = null;
    if (pick !== "fps" || !gameVersion) {
      return;
    }
    let stale = false;
    void Promise.all(
      LOADERS.map((loader) =>
        api
          .loaderVersions(loader, gameVersion)
          .then((list) => [loader, list] as const)
          .catch(() => [loader, []] as const),
      ),
    ).then((found) => {
      if (!stale) {
        loaders = Object.fromEntries(found);
      }
    });
    return () => {
      stale = true;
    };
  });

  function describe(err: unknown): string {
    return t(`errors.${isCommandError(err) ? err.kind : "core"}`);
  }

  function chooseLook(pretty: boolean) {
    Object.assign(settings, pretty ? BEAUTIFUL : FAST);
  }

  function chooseLanguage(next: Locale) {
    language = next;
    setLocale(next);
  }

  function finish() {
    settings.setupDone = true;
  }

  async function start() {
    working = true;
    error = null;
    try {
      if (pick === "pack") {
        finish();
        await goto("/modpacks");
      } else if (pick === "file") {
        const path = await openFile({
          filters: [{ name: t("instances.packFiles"), extensions: ["mrpack", "zip"] }],
        });
        if (path) {
          finish();
          await importPack(path);
        }
      } else {
        const chosen = pick === "fps" ? loader : null;
        const build = chosen ? preferredRelease(loaders?.[chosen] ?? []) : undefined;
        const instance = await createInstance(
          chosen ? t("welcome.fpsName", { version }) : version,
          version,
          chosen && build ? { loader: chosen, version: build.version } : null,
        );
        finish();
        await goto(`/instance/${instance.id}`);
        if (chosen) {
          void addFpsMods(instance.id, chosen);
        }
      }
    } catch (err) {
      error = describe(err);
    } finally {
      working = false;
    }
  }

  async function addFpsMods(id: string, chosen: Loader) {
    const items = FPS_MODS[chosen].map((project) => ({ source: "modrinth" as const, id: project }));
    try {
      await api.installContent(id, "mod", null, items);
    } catch (err) {
      const none = isCommandError(err) && err.kind === "noCompatibleVersion";
      toast({ title: t("welcome.fps"), body: none ? t("welcome.noFpsMods") : describe(err) });
    }
  }
</script>

{#snippet arrow()}
  <svg class="arrow" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14M13 6l6 6-6 6" /></svg>
{/snippet}

{#snippet check()}
  <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 12.5l4 4 8-9" /></svg>
{/snippet}

<dialog bind:this={dialog} aria-labelledby={titleId} oncancel={(event) => event.preventDefault()}>
  <ol class="steps">
    {#each STEPS as name, index (name)}
      {@const done = STEPS.indexOf(step) > index}
      <li class:current={step === name} class:done>
        <span class="dot">
          {#if done}
            {@render check()}
          {:else}
            {index + 1}
          {/if}
        </span>
        {t(`welcome.steps.${name}`)}
      </li>
    {/each}
  </ol>

  {#if step === "look"}
    <h2 id={titleId}>{t("welcome.title")}</h2>
    <p class="lead">{t("welcome.lead")}</p>
    <div class="looks">
      <button type="button" class="look" aria-pressed={beautiful} onclick={() => chooseLook(true)}>
        <span class="preview pretty" aria-hidden="true">
          <span class="bar"></span>
          {#each { length: 6 } as _, index (index)}
            <span class="tile"></span>
          {/each}
        </span>
        <span class="tick">{@render check()}</span>
        <strong>{t("welcome.beautiful")}</strong>
        <span class="hint">{t("welcome.beautifulHint")}</span>
        {#if gpu && !gpu.weak}
          <span class="tag">{t("welcome.gpuFine")}</span>
        {/if}
      </button>
      <button
        type="button"
        class="look"
        aria-pressed={!beautiful}
        onclick={() => chooseLook(false)}
      >
        <span class="preview plain" aria-hidden="true">
          <span class="bar"></span>
          {#each { length: 6 } as _, index (index)}
            <span class="tile"></span>
          {/each}
        </span>
        <span class="tick">{@render check()}</span>
        <strong>{t("welcome.fast")}</strong>
        <span class="hint">{t("welcome.fastHint")}</span>
        {#if gpu?.weak}
          <span class="tag">{t("welcome.gpuAdvice", { gpu: gpu.name })}</span>
        {/if}
      </button>
    </div>
    <div class="foot">
      <div class="language">
        <label class="sr-only" for="{titleId}-language">{t("welcome.language")}</label>
        <Select
          id="{titleId}-language"
          bind:value={() => language, chooseLanguage}
          options={LANGUAGES}
        />
      </div>
      <span class="grow"></span>
      <!-- svelte-ignore a11y_autofocus -->
      <button
        type="button"
        class="pill-button primary go"
        autofocus
        onclick={() => (step = "account")}
      >
        {t("welcome.next")}
        {@render arrow()}
      </button>
    </div>
  {:else if step === "account"}
    {#if account}
      <div class="hello">
        <PlayerHead skin={account.skin} name={account.name} size={72} />
        <div>
          <h2 id={titleId}>{t("welcome.hello", { name: account.name })}</h2>
          <p class="lead">{t("welcome.helloLead")}</p>
        </div>
      </div>
      <div class="foot">
        <span class="grow"></span>
        <button type="button" class="pill-button primary go" onclick={() => (step = "pack")}>
          {t("welcome.next")}
          {@render arrow()}
        </button>
      </div>
    {:else if session.signingIn}
      <h2 id={titleId}>{t("welcome.waitingTitle")}</h2>
      <p class="lead">{t("welcome.waitingLead")}</p>
      <div class="waiting">
        <span class="moon"><MoonPhase percent={50} /></span>
        {t("welcome.waitingTitle")}
      </div>
      <div class="foot">
        <span class="grow"></span>
        <button type="button" class="link" onclick={() => cancelSignIn()}>
          {t("welcome.cancel")}
        </button>
      </div>
    {:else}
      <h2 id={titleId}>{t("welcome.signInTitle")}</h2>
      <p class="lead">{t("welcome.signInLead")}</p>
      <div class="sign-in">
        <button type="button" class="microsoft" onclick={() => signIn()}>
          <svg viewBox="0 0 21 21" aria-hidden="true">
            <rect x="0" y="0" width="10" height="10" fill="#f25022" />
            <rect x="11" y="0" width="10" height="10" fill="#7fba00" />
            <rect x="0" y="11" width="10" height="10" fill="#00a4ef" />
            <rect x="11" y="11" width="10" height="10" fill="#ffb900" />
          </svg>
          {session.error ? t("welcome.retry") : t("welcome.signIn")}
        </button>
      </div>
      {#if session.error}
        <p class="error" role="alert">{t(`errors.${session.error.kind}`)}</p>
      {/if}
      <div class="foot">
        <button type="button" class="link" onclick={() => (step = "pack")}>
          {t("welcome.skip")}
        </button>
        <span class="note">{t("welcome.skipHint")}</span>
      </div>
    {/if}
  {:else}
    <h2 id={titleId}>{t("welcome.packTitle")}</h2>
    <p class="lead">{t("welcome.packLead")}</p>
    <div class="picks">
      {#each PICKS as option (option.value)}
        <button
          type="button"
          class="pick"
          aria-pressed={pick === option.value}
          onclick={() => (pick = option.value)}
        >
          {#if option.cover}
            <span class="cover" style:background={COVERS[option.cover]}></span>
          {:else}
            <span class="cover file" aria-hidden="true">
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 4.5v10M8 10.5l4 4 4-4M5 19.5h14" />
              </svg>
            </span>
          {/if}
          <span class="tick">{@render check()}</span>
          <strong>{t(PICK_TEXT[option.value].title)}</strong>
          <span class="hint">{t(PICK_TEXT[option.value].hint)}</span>
          {#if option.value === "fps"}
            <span class="tag">{t("welcome.fpsTag")}</span>
          {/if}
        </button>
      {/each}
    </div>

    {#if needsVersion}
      <div class="block">
        <div class="heading">
          <strong>{t("welcome.version")}</strong>
          <span>{versionHint}</span>
        </div>
        <div class="chips">
          {#each chips as id (id)}
            <button
              type="button"
              class="chip"
              aria-pressed={version === id}
              onclick={() => (version = id)}
            >
              {id}
              {#if id === latest?.id}
                <small>{t("welcome.versionNew")}</small>
              {/if}
            </button>
          {/each}
          {#if releases.length > 0}
            <div class="other" class:chosen={other}>
              <label class="sr-only" for="{titleId}-version">{t("welcome.versionOther")}</label>
              <Select
                id="{titleId}-version"
                bind:value={() => (other ? version : ""), (next) => (version = next)}
                placeholder={t("welcome.versionOther")}
                options={releases.map((entry) => ({ value: entry.id, label: entry.id }))}
              />
            </div>
          {/if}
        </div>
        {#if pick === "fps"}
          <div class="heading second">
            <strong>{t("welcome.loader")}</strong>
            <span>
              {#if !loaders}
                {t("welcome.loaderChecking")}
              {:else if !loader}
                {t("welcome.noLoaders")}
              {:else}
                {loaderHint}
              {/if}
            </span>
          </div>
          <div class="chips">
            {#each LOADERS as name (name)}
              <button
                type="button"
                class="chip"
                aria-pressed={loader === name}
                disabled={!available.includes(name)}
                onclick={() => (chosenLoader = name)}
              >
                {LOADER_NAMES[name]}
                {#if name === "fabric"}
                  <small>{t("welcome.loaderRecommended")}</small>
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <p class="block note-block">
        {pick === "pack" ? t("welcome.packNote") : t("welcome.fileNote")}
      </p>
    {/if}

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <div class="foot">
      <button type="button" class="link" onclick={finish}>{t("welcome.skip")}</button>
      <span class="grow"></span>
      <button type="button" class="pill-button primary go" disabled={!ready} onclick={start}>
        {pick === "pack"
          ? t("welcome.openCatalog")
          : pick === "file"
            ? t("welcome.chooseFile")
            : t("welcome.create")}
        {@render arrow()}
      </button>
    </div>
  {/if}
</dialog>

<style>
  /* Anchored near the top so that steps of different height do not make it jump. */
  dialog {
    --top: max(24px, calc(50vh - 340px));
    width: min(860px, calc(100vw - 48px));
    max-height: calc(100vh - var(--top) - 24px);
    margin: var(--top) auto auto;
    padding: 30px 36px 30px;
    overflow-y: auto;
    border: none;
    border-radius: 30px;
    background: var(--surface-strong);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      0 0 0 1px var(--border),
      0 40px 80px -30px rgb(0 0 0 / 0.6);
    color: var(--text);
    backdrop-filter: var(--glass);
  }

  :global(:root[data-style="solid"]) dialog {
    background: var(--surface-solid);
  }

  dialog::backdrop {
    background: color-mix(in srgb, var(--background) 45%, transparent);
    backdrop-filter: blur(18px);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .steps {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 0 22px;
    padding: 0;
    list-style: none;
    color: var(--text-muted);
    font-size: 13.5px;
    font-weight: 750;
  }

  .steps li {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .steps li + li::before {
    width: 30px;
    height: 1.5px;
    margin-right: 4px;
    background: var(--border);
    content: "";
  }

  .steps .current {
    color: var(--text);
  }

  .dot {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--field);
    font-size: 12px;
  }

  .current .dot {
    background: var(--accent);
    color: var(--accent-text);
  }

  .done .dot {
    background: var(--accent-soft);
    color: var(--accent-fg);
  }

  .dot svg {
    width: 14px;
    height: 14px;
    stroke-width: 2.4;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 34px;
    font-weight: var(--display-weight);
    line-height: 1.15;
    letter-spacing: -0.015em;
  }

  .lead {
    max-width: 640px;
    margin: 10px 0 0;
    color: var(--text-2);
    font-size: 15px;
    font-weight: 650;
    line-height: 1.5;
  }

  .looks,
  .picks {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-top: 24px;
  }

  .picks {
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
    margin-top: 20px;
  }

  .look,
  .pick {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 10px 18px;
    border: none;
    border-radius: 24px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
    color: var(--text);
    text-align: left;
    transition:
      box-shadow 200ms var(--ease),
      transform 260ms var(--spring);
  }

  .look:hover,
  .pick:hover {
    transform: translateY(-2px);
  }

  .look[aria-pressed="true"],
  .pick[aria-pressed="true"] {
    box-shadow:
      inset 0 0 0 2px var(--accent),
      0 14px 34px -14px color-mix(in srgb, var(--accent) 70%, transparent);
  }

  .look strong,
  .pick strong {
    margin: 6px 10px 0;
    font-size: 19px;
    font-weight: 800;
  }

  .pick strong {
    font-size: 17px;
  }

  .hint {
    margin: 0 10px;
    color: var(--text-2);
    font-size: 13.5px;
    font-weight: 600;
    line-height: 1.45;
  }

  .pick .hint {
    font-size: 13px;
  }

  .tag {
    margin: 4px 10px 0;
    padding: 4px 11px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-fg);
    font-size: 12px;
    font-weight: 800;
  }

  .tick {
    position: absolute;
    top: 20px;
    right: 20px;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-text);
    opacity: 0;
    transform: scale(0.6);
    transition:
      opacity 150ms var(--ease),
      transform 260ms var(--spring);
  }

  [aria-pressed="true"] > .tick {
    opacity: 1;
    transform: none;
  }

  .tick svg {
    width: 16px;
    height: 16px;
    stroke-width: 2.6;
  }

  .preview {
    position: relative;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-auto-rows: 52px;
    align-content: start;
    gap: 8px;
    width: 100%;
    height: 150px;
    padding: 26px 26px 0;
    overflow: hidden;
    border-radius: 16px;
  }

  .pretty {
    background:
      radial-gradient(
        circle at 30% 0%,
        color-mix(in srgb, var(--accent) 70%, transparent),
        transparent 55%
      ),
      radial-gradient(circle at 80% 40%, rgb(52 200 190 / 0.45), transparent 50%),
      radial-gradient(1px 1px at 20% 60%, #ffffff, transparent),
      radial-gradient(1px 1px at 70% 20%, #ffffff, transparent),
      radial-gradient(1.5px 1.5px at 85% 75%, #ffffff, transparent),
      radial-gradient(1px 1px at 45% 85%, #ffffff, transparent), #15122c;
  }

  .plain {
    background: var(--surface-solid);
  }

  .bar {
    grid-column: 1 / -1;
    width: 45%;
    height: 10px;
    margin-bottom: 6px;
    border-radius: 999px;
    background: rgb(255 255 255 / 0.8);
  }

  .plain .bar {
    background: var(--text-muted);
    opacity: 0.6;
  }

  .preview .tile {
    border-radius: 10px;
    background: rgb(255 255 255 / 0.06);
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.12);
  }

  .plain .tile {
    background: var(--field);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .cover {
    display: grid;
    place-items: center;
    width: 100%;
    height: 84px;
    border-radius: 16px;
    color: var(--text-2);
    image-rendering: pixelated;
  }

  .cover.file {
    background: var(--field);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .cover svg {
    width: 24px;
    height: 24px;
    stroke-width: 1.8;
  }

  .block {
    margin-top: 16px;
    padding: 18px 20px 20px;
    border-radius: 22px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
  }

  .note-block {
    color: var(--text-2);
    font-size: 14px;
    font-weight: 650;
  }

  .heading {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 12px;
  }

  .heading.second {
    margin-top: 16px;
  }

  .heading strong {
    font-size: 15px;
    font-weight: 800;
  }

  .heading span {
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 650;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }

  .chip {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 38px;
    padding: 0 16px;
    border: none;
    border-radius: 999px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
    color: var(--text);
    font-size: 14.5px;
    font-weight: 800;
    transition:
      background 150ms var(--ease),
      transform 260ms var(--spring);
  }

  .chip:hover:enabled {
    background: var(--surface-hover);
  }

  .chip:active:enabled {
    transform: scale(0.96);
  }

  .chip[aria-pressed="true"] {
    background: var(--accent);
    box-shadow: 0 8px 18px -10px color-mix(in srgb, var(--accent) 80%, transparent);
    color: var(--accent-text);
  }

  .chip:disabled {
    background: transparent;
    color: var(--text-muted);
    opacity: 0.55;
  }

  .chip small {
    font-size: 11px;
    font-weight: 750;
    opacity: 0.8;
  }

  .language {
    width: 200px;
  }

  .language :global(.field) {
    height: 42px;
    border: none;
    border-radius: 999px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
    font-weight: 700;
  }

  .other {
    width: 150px;
  }

  .other :global(.field) {
    height: 38px;
    border: none;
    border-radius: 999px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
    font-weight: 800;
  }

  .other.chosen :global(.field) {
    background: var(--accent);
    color: var(--accent-text);
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-top: 24px;
  }

  .grow {
    flex: 1;
  }

  .go {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 44px;
    padding: 0 22px;
    font-size: 15px;
  }

  .arrow {
    width: 18px;
    height: 18px;
    stroke-width: 2;
  }

  .link {
    height: 40px;
    padding: 0;
    border: none;
    background: none;
    color: var(--text);
    font-size: 14.5px;
    font-weight: 800;
  }

  .link:hover {
    color: var(--accent-fg);
  }

  .note {
    color: var(--text-muted);
    font-size: 13.5px;
    font-weight: 650;
  }

  .sign-in {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-top: 28px;
  }

  .microsoft {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 52px;
    padding: 0 24px;
    border: none;
    border-radius: 999px;
    background: #ffffff;
    box-shadow: 0 10px 26px -12px rgb(0 0 0 / 0.5);
    color: #1b1a2c;
    font-size: 16px;
    font-weight: 800;
    transition: transform 260ms var(--spring);
  }

  .microsoft:active {
    transform: scale(0.97);
  }

  .microsoft svg {
    width: 21px;
    height: 21px;
  }

  .waiting {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 24px;
    color: var(--text-2);
    font-size: 14px;
    font-weight: 750;
  }

  .moon {
    display: flex;
    color: var(--gold);
    animation: pulse 1.6s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.45;
    }
  }

  .hello {
    display: flex;
    align-items: center;
    gap: 24px;
    margin-top: 8px;
  }

  .hello :global(.head) {
    border-radius: 18px;
  }

  .error {
    margin: 14px 0 0;
    color: var(--danger-text);
    font-size: 13.5px;
    font-weight: 700;
  }
</style>
