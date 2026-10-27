<script lang="ts">
  import "$lib/styles/global.css";
  import type { Snippet } from "svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import { locale } from "$lib/i18n/index.svelte";
  import { listenToGames } from "$lib/state/instances.svelte";

  let { children }: { children: Snippet } = $props();

  $effect(() => {
    document.documentElement.lang = locale();
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
