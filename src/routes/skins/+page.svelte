<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { inferModelType, loadImage, loadSkinToCanvas } from "skinview-utils";
  import {
    api,
    type CommandError,
    isCommandError,
    type SavedSkin,
    type SkinProfile,
    type SkinVariant,
  } from "$lib/api";
  import PlayerHead from "$lib/components/PlayerHead.svelte";
  import Segmented from "$lib/components/Segmented.svelte";
  import SkinViewer from "$lib/components/SkinViewer.svelte";
  import TextureCrop from "$lib/components/TextureCrop.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { scrollFade } from "$lib/scroll-fade";
  import { activeAccount, loadAccounts } from "$lib/state/accounts.svelte";
  import { SKIN_ANIMATIONS, settings } from "$lib/state/settings.svelte";
  import { tooltip } from "$lib/tooltip";

  let profile = $state<SkinProfile | null>(null);
  let profileError = $state<CommandError | null>(null);
  let library = $state<SavedSkin[]>([]);
  let selected = $state<string | null>(null);
  let busy = $state(false);
  let actionError = $state<CommandError | null>(null);
  let hoveredCape = $state<string | null>(null);
  let saving = $state(false);
  let savedUrl = $state<string | null>(null);

  const account = $derived(activeAccount());
  const preview = $derived(library.find((skin) => skin.id === selected));
  const activeCape = $derived(profile?.capes.find((cape) => cape.active) ?? null);
  const shownCape = $derived(
    hoveredCape === "none"
      ? null
      : (profile?.capes.find((cape) => cape.id === hoveredCape)?.url ?? activeCape?.url ?? null),
  );
  const savedCurrent = $derived(savedUrl !== null && savedUrl === profile?.skin?.url);
  const animationOptions = $derived(
    SKIN_ANIMATIONS.map((value) => ({ value, label: t(`skins.animations.${value}`) })),
  );
  const variants = $derived([
    { value: "classic" as const, label: t("skins.classic") },
    { value: "slim" as const, label: t("skins.slim") },
  ]);

  $effect(() => {
    api
      .savedSkins()
      .then((skins) => {
        library = skins;
      })
      .catch(() => {});
  });

  // Keyed by id: reloading the account list after each change hands out new objects, and
  // reacting to those would ask Mojang for the profile again and again.
  const accountId = $derived(account?.id);

  $effect(() => {
    if (!accountId) {
      profile = null;
      return;
    }
    profileError = null;
    api
      .skinProfile()
      .then((loaded) => {
        profile = loaded;
        void loadAccounts();
      })
      .catch((error) => {
        profileError = asError(error);
      });
  });

  function asError(error: unknown): CommandError {
    return isCommandError(error) ? error : { kind: "core", message: String(error) };
  }

  async function change(request: () => Promise<SkinProfile>) {
    busy = true;
    actionError = null;
    try {
      profile = await request();
      selected = null;
      await loadAccounts();
    } catch (error) {
      actionError = asError(error);
    } finally {
      busy = false;
    }
  }

  async function addSkin() {
    const path = await open({ filters: [{ name: t("skins.files"), extensions: ["png"] }] });
    if (!path) {
      return;
    }
    actionError = null;
    try {
      const added = await api.addSkin(path, "classic");
      library = [added, ...library.filter((skin) => skin.id !== added.id)];
      selected = added.id;
      if ((await guessVariant(added.image)) === "slim") {
        await setVariant(library[0], "slim");
      }
    } catch (error) {
      actionError = asError(error);
    }
  }

  /** Slim skins leave the outer pixels of the arms empty. */
  async function guessVariant(image: string): Promise<SkinVariant> {
    try {
      const canvas = document.createElement("canvas");
      loadSkinToCanvas(canvas, await loadImage(image));
      return inferModelType(canvas) === "slim" ? "slim" : "classic";
    } catch {
      return "classic";
    }
  }

  /** The capes sit in one row; the mouse wheel moves along it. */
  function scrollCapes(event: WheelEvent & { currentTarget: HTMLElement }) {
    if (event.deltaY !== 0 && event.currentTarget.scrollWidth > event.currentTarget.clientWidth) {
      event.preventDefault();
      event.currentTarget.scrollBy({ left: event.deltaY, behavior: "smooth" });
    }
  }

  async function saveCurrent() {
    saving = true;
    actionError = null;
    try {
      const saved = await api.saveCurrentSkin();
      library = [saved, ...library.filter((skin) => skin.id !== saved.id)];
      savedUrl = profile?.skin?.url ?? null;
    } catch (error) {
      actionError = asError(error);
    } finally {
      saving = false;
    }
  }

  async function setVariant(skin: SavedSkin, variant: SkinVariant) {
    skin.variant = variant;
    await api.setSkinVariant(skin.id, variant);
  }

  async function removeSkin(skin: SavedSkin) {
    await api.removeSkin(skin.id);
    library = library.filter((saved) => saved.id !== skin.id);
    if (selected === skin.id) {
      selected = null;
    }
  }
</script>

<section class="page">
  <header>
    <a class="back-link" href="/">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
      {t("instance.back")}
    </a>
    <h1 class="page-title">{t("skins.title")}</h1>
  </header>

  <div class="layout">
    <aside class="stage">
      <SkinViewer
        skin={preview?.image ?? profile?.skin?.url ?? null}
        variant={preview?.variant ?? profile?.skin?.variant ?? "classic"}
        cape={shownCape}
        animation={settings.skinAnimation}
        name={profile?.name ?? null}
      />
      <fieldset class="animations">
        <legend class="sr-only">{t("skins.animation")}</legend>
        {#each animationOptions as option (option.value)}
          <label>
            <input
              type="radio"
              name="skin-animation"
              value={option.value}
              bind:group={settings.skinAnimation}
            >
            <span>{option.label}</span>
          </label>
        {/each}
      </fieldset>
      {#if preview}
        <p class="caption">{t("skins.previewing", { name: preview.name })}</p>
        <Segmented
          bind:value={() => preview.variant, (variant) => setVariant(preview, variant)}
          options={variants}
          name="variant"
          label={t("skins.arms")}
        />
        <div class="actions">
          <button type="button" class="pill-button" onclick={() => (selected = null)}>
            {t("skins.back")}
          </button>
          <button
            type="button"
            class="pill-button primary"
            disabled={busy || !profile}
            onclick={() => change(() => api.applySkin(preview.id))}
          >
            {busy ? t("skins.applying") : t("skins.apply")}
          </button>
        </div>
      {:else if profile}
        <p class="caption">{t("skins.current", { name: profile.name })}</p>
        {#if profile.skin}
          <button
            type="button"
            class="pill-button"
            disabled={saving || savedCurrent}
            onclick={saveCurrent}
          >
            {savedCurrent ? t("skins.saved") : saving ? t("skins.saving") : t("skins.saveCurrent")}
          </button>
        {/if}
      {:else if !library.length}
        <p class="caption">{t("skins.empty")}</p>
      {/if}
      {#if profile && (profile.capes.length || activeCape)}
        <fieldset class="capes" onwheel={scrollCapes} onpointerleave={() => (hoveredCape = null)}>
          <legend class="sr-only">{t("skins.cape")}</legend>
          <button
            type="button"
            class="cape"
            class:selected={!activeCape}
            disabled={busy}
            aria-label={t("skins.noCape")}
            {@attach tooltip(t("skins.noCape"))}
            onpointerenter={() => (hoveredCape = "none")}
            onpointerleave={() => (hoveredCape = null)}
            onclick={() => change(() => api.setCape(null))}
          >
            <span class="cape-art none" aria-hidden="true"></span>
          </button>
          {#each profile.capes as cape (cape.id)}
            <button
              type="button"
              class="cape"
              class:selected={cape.active}
              disabled={busy}
              aria-label={cape.name}
              {@attach tooltip(cape.name)}
              onpointerenter={() => (hoveredCape = cape.id)}
              onpointerleave={() => (hoveredCape = null)}
              onclick={() => change(() => api.setCape(cape.id))}
            >
              <span class="cape-art">
                <TextureCrop src={cape.url} x={1} y={1} width={10} height={16} />
              </span>
            </button>
          {/each}
        </fieldset>
      {/if}
    </aside>

    <div class="content" {@attach scrollFade}>
      {#if !account}
        <p class="notice">{t("skins.signIn")}</p>
      {:else if profileError}
        <p class="notice">{t(`errors.${profileError.kind}`)}</p>
      {/if}
      {#if actionError}
        <p class="notice error" role="alert">{t(`errors.${actionError.kind}`)}</p>
      {/if}

      <section class="panel">
        <div class="panel-header">
          <h2 class="panel-title">{t("skins.library")}</h2>
          <button type="button" class="pill-button" onclick={addSkin}>{t("skins.add")}</button>
        </div>
        {#if library.length}
          <div class="skins">
            {#each library as skin (skin.id)}
              <div class="tile" class:selected={skin.id === selected}>
                <button type="button" class="pick" onclick={() => (selected = skin.id)}>
                  <PlayerHead skin={skin.image} name={skin.name} size={44} />
                  <span class="tile-name">{skin.name}</span>
                </button>
                <button
                  type="button"
                  class="icon-button danger remove"
                  aria-label={t("skins.remove")}
                  {@attach tooltip(t("skins.remove"))}
                  onclick={() => removeSkin(skin)}
                >
                  <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" /></svg>
                </button>
              </div>
            {/each}
          </div>
        {:else}
          <p class="hint">{t("skins.libraryHint")}</p>
        {/if}
      </section>

      {#if profile}
        <section class="panel">
          <div class="row">
            <div>
              <h3>{t("skins.reset")}</h3>
              <p>{t("skins.resetHint")}</p>
            </div>
            <button
              type="button"
              class="pill-button"
              disabled={busy}
              onclick={() => change(() => api.resetSkin())}
            >
              {t("skins.resetButton")}
            </button>
          </div>
        </section>
      {/if}
    </div>
  </div>
</section>

<style>
  header {
    display: grid;
    gap: 4px;
    padding-top: 8px;
  }

  .layout {
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
    gap: 20px;
    min-height: 0;
  }

  .stage {
    display: grid;
    align-content: start;
    justify-items: center;
    gap: 12px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background:
      radial-gradient(
        circle at 50% 35%,
        color-mix(in srgb, var(--accent) 22%, transparent),
        transparent 70%
      ),
      var(--surface);
    backdrop-filter: blur(24px) saturate(1.4);
  }

  .caption {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .content {
    display: grid;
    grid-auto-rows: max-content;
    align-content: start;
    gap: 16px;
    min-height: 0;
    overflow-y: auto;
  }

  .notice {
    margin: 0;
    padding: 12px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 13px;
  }

  .notice.error {
    color: var(--danger);
  }

  .panel {
    gap: 8px;
    padding-bottom: 14px;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-right: 14px;
  }

  .hint {
    margin: 0;
    padding: 0 18px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .skins {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 8px;
    padding: 0 14px;
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: none;
  }

  .animations {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 4px;
  }

  .animations input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .animations span {
    display: grid;
    place-items: center;
    height: 26px;
    padding: 0 10px;
    border: none;
    border-radius: 999px;
    background: var(--field);
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 600;
    transition:
      background 150ms var(--ease),
      color 150ms var(--ease);
  }

  .animations label {
    cursor: pointer;
  }

  .animations label:hover span {
    color: var(--text);
  }

  .animations input:checked + span {
    background: var(--accent);
    color: var(--accent-text);
  }

  .animations input:focus-visible + span {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .capes {
    display: flex;
    gap: 6px;
    width: 100%;
    padding: 14px 4px 6px;
    overflow-x: auto;
    border-top: 1px solid var(--border);
    scrollbar-width: thin;
  }

  .tile {
    position: relative;
  }

  .cape {
    flex: none;
    display: grid;
    place-items: center;
    width: 42px;
    height: 58px;
    padding: 0;
    border: 1.5px solid transparent;
    border-radius: var(--radius-sm);
    background: var(--field);
    transition:
      border-color 150ms var(--ease),
      transform 150ms var(--ease);
  }

  .cape:hover:enabled {
    border-color: var(--border);
    transform: translateY(-5px);
  }

  .cape.selected {
    border-color: var(--accent);
  }

  .pick {
    display: grid;
    justify-items: center;
    gap: 8px;
    width: 100%;
    padding: 14px 8px 10px;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    background: var(--field);
    transition: border-color 150ms var(--ease);
  }

  .pick:hover {
    border-color: var(--border);
  }

  .selected .pick {
    border-color: var(--accent);
  }

  .tile-name {
    max-width: 100%;
    overflow: hidden;
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .remove {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 24px;
    height: 24px;
    opacity: 0;
  }

  .tile:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  .cape-art {
    width: 30px;
    height: 48px;
    overflow: hidden;
    border-radius: 4px;
  }

  .cape-art.none {
    border: 1.5px dashed var(--border);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 18px 0;
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
</style>
