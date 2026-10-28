<script lang="ts">
  import type { SkinViewer } from "skinview3d";
  import type { SkinVariant } from "$lib/api";

  let { skin, variant, cape }: { skin: string | null; variant: SkinVariant; cape: string | null } =
    $props();

  let canvas: HTMLCanvasElement;
  let viewer = $state<SkinViewer>();

  $effect(() => {
    let disposed = false;
    let created: SkinViewer | undefined;
    // three.js is large, so it only loads when someone opens the skins page.
    void import("skinview3d").then(({ SkinViewer, WalkingAnimation }) => {
      if (disposed) {
        return;
      }
      created = new SkinViewer({ canvas, width: 260, height: 360, zoom: 0.85 });
      const walking = new WalkingAnimation();
      walking.speed = 0.6;
      created.animation = walking;
      created.controls.enableZoom = false;
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

<canvas bind:this={canvas} class:hidden={!skin}></canvas>

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
</style>
