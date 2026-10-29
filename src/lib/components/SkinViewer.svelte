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
  }: {
    skin: string | null;
    variant: SkinVariant;
    cape: string | null;
    animation: SkinAnimation;
  } = $props();

  let canvas: HTMLCanvasElement;
  let viewer = $state<SkinViewer>();
  let animations = $state<typeof import("skinview3d")>();
  let unsupported = $state(false);

  /**
   * Loads a picture and hands it over unless a newer one was asked for meanwhile, so quick
   * hovering over capes never ends on the wrong one.
   */
  function whenLoaded(url: string, apply: (image: HTMLImageElement) => void): () => void {
    let stale = false;
    const image = new Image();
    image.crossOrigin = "anonymous";
    image.src = url;
    image
      .decode()
      .then(() => {
        if (!stale) {
          apply(image);
        }
      })
      .catch(() => {});
    return () => {
      stale = true;
    };
  }

  /** Small laptop screens get a smaller model, so the capes under it stay in view. */
  function fittedSize(): { width: number; height: number } {
    const height = Math.round(Math.min(360, Math.max(240, innerHeight * 0.4)));
    return { width: Math.round((height * 260) / 360), height };
  }

  $effect(() => {
    const resize = () => {
      const { width, height } = fittedSize();
      viewer?.setSize(width, height);
    };
    addEventListener("resize", resize);
    return () => removeEventListener("resize", resize);
  });

  $effect(() => {
    let disposed = false;
    let created: SkinViewer | undefined;
    // three.js is large, so it only loads when someone opens the skins page.
    void import("skinview3d").then((module) => {
      if (disposed) {
        return;
      }
      try {
        created = new module.SkinViewer({ canvas, ...fittedSize(), zoom: 0.8 });
      } catch {
        // Old or blocklisted graphics drivers give no WebGL context.
        unsupported = true;
        return;
      }
      created.controls.enableZoom = false;
      // Turning glides to a stop instead of following every jitter of the mouse.
      created.controls.enableDamping = true;
      created.controls.dampingFactor = 0.12;
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
    if (!skin) {
      viewer.resetSkin();
      return;
    }
    const model = variant === "slim" ? "slim" : "default";
    return whenLoaded(skin, (image) => viewer?.loadSkin(image, { model }));
  });

  $effect(() => {
    if (!viewer) {
      return;
    }
    // Flying looks right with wings, the way the game shows a cape on an elytra.
    const backEquipment = animation === "fly" ? "elytra" : "cape";
    if (!cape) {
      viewer.resetCape();
      return;
    }
    return whenLoaded(cape, (image) => viewer?.loadCape(image, { backEquipment }));
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
