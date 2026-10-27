<script lang="ts">
  import "@fontsource-variable/onest";
  import "@fontsource-variable/unbounded";
  import "$lib/styles/global.css";
  import type { Snippet } from "svelte";
  import DropOverlay from "$lib/components/DropOverlay.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import { locale } from "$lib/i18n/index.svelte";
  import { listenToGames } from "$lib/state/instances.svelte";
  import { applySettings } from "$lib/state/settings.svelte";

  let { children }: { children: Snippet } = $props();

  $effect(() => {
    document.documentElement.lang = locale();
  });

  $effect(() => {
    applySettings();
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
