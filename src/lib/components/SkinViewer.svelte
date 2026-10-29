<script lang="ts">
  import type { PlayerAnimation, SkinViewer } from "skinview3d";
  import type { SkinVariant } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";
  import type { SkinAnimation } from "$lib/state/settings.svelte";

  let {
    skin,
    variant,
    cape,
    animation,
    name = null,
  }: {
    skin: string | null;
    variant: SkinVariant;
    cape: string | null;
    animation: SkinAnimation;
    name?: string | null;
  } = $props();

  let canvas: HTMLCanvasElement;
  let viewer = $state<SkinViewer>();
  let animations = $state<typeof import("skinview3d")>();
  let unsupported = $state(false);

  $effect(() => {
    let disposed = false;
    let created: SkinViewer | undefined;
    // three.js is large, so it only loads when someone opens the skins page.
    void import("skinview3d").then((module) => {
      if (disposed) {
        return;
      }
      try {
        created = new module.SkinViewer({ canvas, width: 260, height: 360, zoom: 0.8 });
      } catch {
        // Old or blocklisted graphics drivers give no WebGL context.
        unsupported = true;
        return;
      }
      created.controls.enableZoom = false;
      animations = module;
      viewer = created;
    });
    return () => {
      disposed = true;
      created?.dispose();
    };
  });

  $effect(() => {
    const module = animations;
    if (!viewer || !module) {
      return;
    }
    const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const created: Record<SkinAnimation, () => PlayerAnimation> = {
      idle: () => new module.IdleAnimation(),
      walk: () => new module.WalkingAnimation(),
      run: () => new module.RunningAnimation(),
      fly: () => new module.FlyingAnimation(),
      wave: () => new module.WaveAnimation(),
    };
    const playing = still ? null : created[animation]();
    if (playing && animation === "walk") {
      playing.speed = 0.6;
    }
    viewer.animation = playing;
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
    // Flying looks right with wings, the way the game shows a cape on an elytra.
    const backEquipment = animation === "fly" ? "elytra" : "cape";
    if (cape) {
      void viewer.loadCape(cape, { backEquipment });
    } else {
      viewer.resetCape();
    }
  });

  $effect(() => {
    if (viewer) {
      viewer.nameTag = name;
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
