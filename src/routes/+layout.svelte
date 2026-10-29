<script lang="ts">
  import "@fontsource-variable/nunito";
  import "@fontsource-variable/onest";
  import "@fontsource-variable/rubik";
  import "@fontsource-variable/unbounded";
  import "$lib/styles/global.css";
  import "$lib/styles/motion.css";
  import type { Snippet } from "svelte";
  import { page } from "$app/state";
  import { api } from "$lib/api";
  import { coverOf } from "$lib/appearance";
  import Backdrop from "$lib/components/Backdrop.svelte";
  import DropOverlay from "$lib/components/DropOverlay.svelte";
  import PrivacyDialog from "$lib/components/PrivacyDialog.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import { locale } from "$lib/i18n/index.svelte";
  import { playClick } from "$lib/sound";
  import { discordPresence, library, listenToGames } from "$lib/state/instances.svelte";
  import { applySettings, settings } from "$lib/state/settings.svelte";
  import { checkForUpdate } from "$lib/state/update.svelte";

  let { children }: { children: Snippet } = $props();

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
    if (ctrl && event.code === "KeyF") {
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

  $effect(() => {
    const unlisten = listenToGames();
    return () => {
      void unlisten.then((stop) => stop());
    };
  });
</script>

<svelte:window
  {oncontextmenu}
  {onkeydown}
  {onpointerdown}
  onclick={blockNewWindow}
  onauxclick={blockNewWindow}
/>

<Backdrop kind={backdrop} cover={instance ? coverOf(instance.id, instance.cover) : null} />
<div class="app">
  <TitleBar />
  <main>
    {@render children()}
  </main>
</div>
<DropOverlay />
<PrivacyDialog />

<style>
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
    padding: 0 24px 24px;
  }
</style>
