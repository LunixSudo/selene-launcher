<script lang="ts">
  import type { SkinViewer } from "skinview3d";
  import type { SkinVariant } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";

  let { skin, variant, cape }: { skin: string | null; variant: SkinVariant; cape: string | null } =
    $props();

  let canvas: HTMLCanvasElement;
  let viewer = $state<SkinViewer>();
  let unsupported = $state(false);

  $effect(() => {
    let disposed = false;
    let created: SkinViewer | undefined;
    // three.js is large, so it only loads when someone opens the skins page.
    void import("skinview3d").then(({ SkinViewer, WalkingAnimation }) => {
      if (disposed) {
        return;
      }
      try {
        created = new SkinViewer({ canvas, width: 260, height: 360, zoom: 0.85 });
      } catch {
        // Old or blocklisted graphics drivers give no WebGL context.
        unsupported = true;
        return;
      }
      created.controls.enableZoom = false;
      if (!matchMedia("(prefers-reduced-motion: reduce)").matches) {
        const walking = new WalkingAnimation();
        walking.speed = 0.6;
        created.animation = walking;
      }
      viewer = created;
    });
    return () => {
      disposed = true;
      created?.dispose();
    };
  });

  $effect(() => {
    if (!viewer) {
      return;
    }
    if (skin) {
      void viewer.loadSkin(skin, { model: variant === "slim" ? "slim" : "default" });
    } else {
      viewer.resetSkin();
    }
  });

  $effect(() => {
    if (!viewer) {
      return;
    }
    if (cape) {
      void viewer.loadCape(cape);
    } else {
      viewer.resetCape();
    }
  });
</script>

{#if unsupported}
  <p class="unsupported">{t("skins.noPreview")}</p>
{/if}
<canvas bind:this={canvas} class:hidden={!skin || unsupported}></canvas>

<style>
  canvas {
    cursor: grab;
  }

  canvas:active {
    cursor: grabbing;
  }

  .hidden {
    visibility: hidden;
  }

  .unsupported {
    max-width: 260px;
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
  }
</style>
