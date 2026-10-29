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

  /** How far the pointer may move for a press to still count as a click, not a turn. */
  const CLICK_SLOP = 4;
  const BOUNCE_MS = 420;

  let canvas: HTMLCanvasElement;
  let viewer = $state<SkinViewer>();
  let animations = $state<typeof import("skinview3d")>();
  let unsupported = $state(false);
  let shown = $state(false);
  let pressedAt: { x: number; y: number } | null = null;

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
      created.controls.enablePan = false;
      // Turning only around the player keeps the feet on the floor shadow.
      created.controls.minPolarAngle = Math.PI / 2;
      created.controls.maxPolarAngle = Math.PI / 2;
      // Turning glides to a stop instead of following every jitter of the mouse.
      created.controls.enableDamping = true;
      created.controls.dampingFactor = 0.1;
      created.playerWrapper.rotation.y = -0.45;
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
    return whenLoaded(skin, (image) => {
      viewer?.loadSkin(image, { model });
      // Fades in once dressed, instead of flashing the default skin first.
      shown = true;
    });
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

  function press(event: PointerEvent) {
    pressedAt = { x: event.clientX, y: event.clientY };
  }

  /** A click that did not turn the player makes it hop, like a poke. */
  function release(event: PointerEvent) {
    const start = pressedAt;
    pressedAt = null;
    const player = viewer?.playerObject;
    if (
      !start ||
      !player ||
      Math.hypot(event.clientX - start.x, event.clientY - start.y) > CLICK_SLOP ||
      matchMedia("(prefers-reduced-motion: reduce)").matches
    ) {
      return;
    }
    const began = performance.now();
    const step = (now: number) => {
      const progress = Math.min(1, (now - began) / BOUNCE_MS);
      const wave = Math.sin(progress * Math.PI) * (1 - progress);
      player.scale.set(1 + wave * 0.12, 1 - wave * 0.1, 1 + wave * 0.12);
      player.position.y = Math.sin(progress * Math.PI) * 3;
      if (progress < 1) {
        requestAnimationFrame(step);
      }
    };
    requestAnimationFrame(step);
  }
</script>

{#if unsupported}
  <p class="unsupported">{t("skins.noPreview")}</p>
{/if}
<div class="viewer" class:hidden={!skin || unsupported} class:shown>
  <span class="floor" aria-hidden="true"></span>
  <canvas bind:this={canvas} onpointerdown={press} onpointerup={release}></canvas>
</div>
{#if skin && !unsupported}
  <p class="hint">{t("skins.dragHint")}</p>
{/if}

<style>
  .viewer {
    position: relative;
    display: grid;
    place-items: center;
    opacity: 0;
    transition: opacity 400ms var(--ease);
  }

  .viewer.shown {
    opacity: 1;
  }

  canvas {
    position: relative;
    cursor: grab;
    touch-action: none;
  }

  .floor {
    position: absolute;
    bottom: 6%;
    left: 50%;
    width: 46%;
    height: 7%;
    border-radius: 50%;
    background: radial-gradient(closest-side, rgb(0 0 0 / 0.35), transparent);
    transform: translateX(-50%);
  }

  .hint {
    margin: -6px 0 0;
    color: var(--text-muted);
    font-size: 12px;
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
