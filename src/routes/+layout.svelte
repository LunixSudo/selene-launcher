<script lang="ts">
  import "@fontsource-variable/nunito";
  import "@fontsource-variable/onest";
  import "@fontsource-variable/rubik";
  import "@fontsource-variable/unbounded";
  import "$lib/styles/global.css";
  import type { Snippet } from "svelte";
  import DropOverlay from "$lib/components/DropOverlay.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import { locale } from "$lib/i18n/index.svelte";
  import { listenToGames } from "$lib/state/instances.svelte";
  import { applySettings, settings } from "$lib/state/settings.svelte";
  import { checkForUpdate } from "$lib/state/update.svelte";

  let { children }: { children: Snippet } = $props();

  $effect(() => {
    document.documentElement.lang = locale();
  });

  $effect(() => {
    applySettings();
  });

  $effect(() => {
    if (import.meta.env.PROD && settings.checkUpdates) {
      void checkForUpdate();
    }
  });

  $effect(() => {
    // The webview's own menu (open link in new window, save link as) makes no sense here.
    const block = (event: MouseEvent) => {
      const target = event.target as HTMLElement;
      if (!target.closest("input, textarea")) {
        event.preventDefault();
      }
    };
    window.addEventListener("contextmenu", block);
    return () => window.removeEventListener("contextmenu", block);
  });

  $effect(() => {
    const onkeydown = (event: KeyboardEvent) => {
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
    };
    window.addEventListener("keydown", onkeydown);
    return () => window.removeEventListener("keydown", onkeydown);
  });

  $effect(() => {
    const unlisten = listenToGames();
    return () => {
      void unlisten.then((stop) => stop());
    };
  });
</script>

<div class="app">
  <TitleBar />
  <main>
    {@render children()}
  </main>
</div>
<DropOverlay />

<style>
  .app {
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
