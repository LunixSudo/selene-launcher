<script lang="ts">
  import "$lib/styles/global.css";
  import type { Snippet } from "svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";

  let { children }: { children: Snippet } = $props();
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
