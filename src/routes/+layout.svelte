<script lang="ts">
  import "@fontsource-variable/nunito";
  import "@fontsource-variable/pixelify-sans";
  import "#lib/styles/global.css";
  import "#lib/styles/motion.css";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { Snippet } from "svelte";
  import { fade } from "svelte/transition";
  import { api } from "#lib/api.ts";
  import { coverOf, sceneOf } from "#lib/appearance.ts";
  import Backdrop from "#lib/components/Backdrop.svelte";
  import CommandPalette from "#lib/components/CommandPalette.svelte";
  import DropOverlay from "#lib/components/DropOverlay.svelte";
  import NavDrawer from "#lib/components/NavDrawer.svelte";
  import PrivacyDialog from "#lib/components/PrivacyDialog.svelte";
  import TitleBar from "#lib/components/TitleBar.svelte";
  import Toaster from "#lib/components/Toaster.svelte";
  import WelcomeDialog from "#lib/components/WelcomeDialog.svelte";
  import { trackNavigation } from "#lib/history.ts";
  import { locale } from "#lib/i18n/index.svelte.ts";
  import { playClick } from "#lib/sound.ts";
  import { discordPresence, library, listenToGames } from "#lib/state/instances.svelte.ts";
  import { applySettings, settings } from "#lib/state/settings.svelte.ts";
  import { ui } from "#lib/state/ui.svelte.ts";
  import { checkForUpdate } from "#lib/state/update.svelte.ts";
  import { afterNavigate } from "$app/navigation";
  import { page } from "$app/state";

  let { children }: { children: Snippet } = $props();

  let privacySettled = $state(false);

  const instance = $derived(
    page.route.id === "/instance/[id]"
      ? library.instances.find((entry) => entry.id === page.params.id)
      : undefined,
  );
  const own = $derived((instance && settings.instanceBackdrops[instance.id]) ?? "app");
  const backdrop = $derived(own === "app" ? settings.backdrop : own);

  $effect(() => {
    document.documentElement.lang = locale();
  });

  $effect(() => {
    applySettings();
  });

  // The window starts hidden and shows once the first frame with the settings applied is
  // drawn, instead of opening empty and filling in.
  $effect(() => {
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        void getCurrentWindow().show();
      }),
    );
  });

  // Endless animations stop while the window is in the background, so they cost the game
  // nothing while it is played.
  $effect(() => {
    const root = document.documentElement;
    const update = () => {
      root.dataset.window = document.hasFocus() ? "active" : "background";
    };
    update();
    addEventListener("focus", update);
    addEventListener("blur", update);
    return () => {
      removeEventListener("focus", update);
      removeEventListener("blur", update);
    };
  });

  let sentPresence = "";
  $effect(() => {
    const presence = settings.discordStatus ? discordPresence() : null;
    // Launch progress reruns this many times a second; Discord only needs real changes.
    const key = JSON.stringify(presence);
    if (key !== sentPresence) {
      sentPresence = key;
      api.setDiscordPresence(presence).catch(() => {});
    }
  });

  $effect(() => {
    if (import.meta.env.PROD && settings.checkUpdates) {
      void checkForUpdate();
    }
  });

  // The webview's own menu (open link in new window, save link as) makes no sense here.
  function oncontextmenu(event: MouseEvent) {
    if (!(event.target as HTMLElement).closest("input, textarea")) {
      event.preventDefault();
    }
  }

  const CLICKABLE =
    "button:enabled, a[href], [role='tab'], input[type='checkbox'], label:has(input[type='radio'])";

  function onpointerdown(event: PointerEvent) {
    if (
      settings.clickSound &&
      event.button === 0 &&
      (event.target as HTMLElement).closest(CLICKABLE)
    ) {
      playClick();
    }
  }

  // Ctrl, Shift or middle click on a link would open a bare webview window.
  function blockNewWindow(event: MouseEvent) {
    const opensWindow = event.button === 1 || event.ctrlKey || event.shiftKey || event.metaKey;
    if (opensWindow && (event.target as HTMLElement).closest("a")) {
      event.preventDefault();
    }
  }

  function onkeydown(event: KeyboardEvent) {
    const ctrl = event.ctrlKey || event.metaKey;
    if (ctrl && event.code === "KeyK") {
      event.preventDefault();
      ui.paletteOpen = !ui.paletteOpen;
    } else if (ctrl && event.code === "KeyF") {
      // Every page with a list marks its search field; Ctrl+F jumps there.
      const search = document.querySelector<HTMLInputElement>("[data-search]");
      event.preventDefault();
      search?.focus();
      search?.select();
    } else if (
      import.meta.env.PROD &&
      (event.key === "F5" || (ctrl && ["KeyR", "KeyP", "KeyU", "KeyG"].includes(event.code)))
    ) {
      // Reloading or printing the webview would only break the launcher.
      event.preventDefault();
    }
  }

  afterNavigate(trackNavigation);

  $effect(() => {
    const unlisten = listenToGames();
    return () => {
      void unlisten.then((stop) => stop());
    };
  });

  function onerror(event: Event) {
    if (event instanceof ErrorEvent) {
      void api.logUiError(`${event.message} at ${event.filename}:${event.lineno}`).catch(() => {});
    }
  }

  function onunhandledrejection(event: PromiseRejectionEvent) {
    const reason = event.reason;
    const text =
      reason instanceof Error ? (reason.stack ?? reason.message) : JSON.stringify(reason);
    void api.logUiError(`unhandled: ${text}`).catch(() => {});
  }
</script>

<svelte:window
  {oncontextmenu}
  {onkeydown}
  {onpointerdown}
  onclick={blockNewWindow}
  onauxclick={blockNewWindow}
  {onerror}
  {onunhandledrejection}
/>

<Backdrop
  kind={backdrop}
  cover={instance ? coverOf(instance.id, instance.cover) : null}
  scene={instance ? sceneOf(instance.id, instance.cover) : null}
/>
{#if instance && own !== "cover"}
  {#key instance.id}
    <!-- The instance's cover across the top of its page, fading out under the tabs. -->
    <div
      class="wall"
      style:background={coverOf(instance.id, instance.cover)}
      aria-hidden="true"
      transition:fade={{ duration: 300 }}
    ></div>
  {/key}
{/if}
<div class="app" class:pinned={settings.menuPinned}>
  <TitleBar />
  <main>
    {@render children()}
  </main>
</div>
<NavDrawer />
<CommandPalette />
<Toaster />
<DropOverlay />
<PrivacyDialog bind:settled={privacySettled} />
<WelcomeDialog open={privacySettled && !settings.setupDone} />

<style>
  .wall {
    position: fixed;
    inset: 0 0 auto;
    z-index: 0;
    height: 340px;
    image-rendering: pixelated;
    opacity: 0.62;
    mask-image: linear-gradient(to bottom, #000 30%, transparent);
    pointer-events: none;
  }

  .wall::after {
    position: absolute;
    inset: 0;
    background: color-mix(in srgb, var(--background) 30%, transparent);
    content: "";
  }

  .app {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  main {
    flex: 1;
    min-height: 0;
    padding: 4px 32px 24px 40px;
    transition: padding-left 500ms var(--spring);
  }

  .pinned main {
    padding-left: 298px;
  }
</style>
