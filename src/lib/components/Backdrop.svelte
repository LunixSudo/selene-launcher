<script lang="ts">
  import { fade } from "svelte/transition";
  import { SHADER_BACKDROPS, type ShaderBackdrop, startShader } from "$lib/backdrops";
  import type { Backdrop } from "$lib/state/settings.svelte";

  const STAR_COUNT = 170;
  const SHOOTING_STAR_EVERY_MS = 7000;
  const SHOOTING_STAR_SPEED = 700;

  let { kind, cover = null }: { kind: Backdrop | "cover"; cover?: string | null } = $props();

  let canvas = $state<HTMLCanvasElement>();

  $effect(() => {
    if (!canvas) {
      return;
    }
    if (kind === "stars") {
      return drawStars(canvas);
    }
    if (isShader(kind)) {
      return startShader(canvas, kind) ?? undefined;
    }
  });

  function isShader(kind: string): kind is ShaderBackdrop {
    return SHADER_BACKDROPS.includes(kind as ShaderBackdrop);
  }

  interface Star {
    x: number;
    y: number;
    size: number;
    phase: number;
    speed: number;
  }

  /** Twinkling stars with now and then a shooting one. Returns a function that stops it. */
  function drawStars(canvas: HTMLCanvasElement): () => void {
    const context = canvas.getContext("2d");
    if (!context) {
      return () => {};
    }
    const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const stars: Star[] = Array.from({ length: STAR_COUNT }, () => ({
      x: Math.random(),
      y: Math.random(),
      size: Math.random() ** 3 * 1.6 + 0.4,
      phase: Math.random() * Math.PI * 2,
      speed: Math.random() * 0.0012 + 0.0004,
    }));
    let shooting: { x: number; y: number; born: number } | null = null;
    let lastShooting = performance.now();
    let frame = 0;
    let color = "";
    let drawn = 0;

    const resize = () => {
      const ratio = devicePixelRatio || 1;
      canvas.width = canvas.clientWidth * ratio;
      canvas.height = canvas.clientHeight * ratio;
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      // Resizing clears the canvas, and a still sky is not redrawn on its own.
      if (still) {
        frame = requestAnimationFrame(draw);
      }
    };
    const draw = (now: number) => {
      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      // Reading styles every frame is costly; once a second still follows a theme switch.
      if (still || drawn++ % 60 === 0) {
        color = getComputedStyle(canvas).color;
      }
      context.clearRect(0, 0, width, height);
      context.fillStyle = color;
      for (const star of stars) {
        context.globalAlpha = still
          ? 0.7
          : 0.35 + 0.65 * Math.abs(Math.sin(star.phase + now * star.speed));
        context.beginPath();
        context.arc(star.x * width, star.y * height, star.size, 0, Math.PI * 2);
        context.fill();
      }

      if (!still && !shooting && now - lastShooting > SHOOTING_STAR_EVERY_MS) {
        shooting = { x: Math.random() * 0.6 + 0.3, y: Math.random() * 0.3, born: now };
        lastShooting = now;
      }
      if (shooting) {
        // It flies at a steady speed until its tail has left the window.
        const seconds = (now - shooting.born) / 1000;
        const headX = shooting.x * width - seconds * SHOOTING_STAR_SPEED;
        const headY = shooting.y * height + seconds * SHOOTING_STAR_SPEED * 0.58;
        if (headX < -140 || headY > height + 80) {
          shooting = null;
        } else {
          const trail = context.createLinearGradient(headX, headY, headX + 120, headY - 70);
          trail.addColorStop(0, color);
          trail.addColorStop(1, "transparent");
          context.globalAlpha = Math.min(1, seconds * 6);
          context.strokeStyle = trail;
          context.lineWidth = 1.6;
          context.beginPath();
          context.moveTo(headX, headY);
          context.lineTo(headX + 120, headY - 70);
          context.stroke();
        }
      }
      context.globalAlpha = 1;
      if (!still) {
        frame = requestAnimationFrame(draw);
      }
    };
    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    if (!still) {
      frame = requestAnimationFrame(draw);
    }

    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }
</script>

{#key kind}
  <div class="backdrop {kind}" aria-hidden="true" transition:fade={{ duration: 400 }}>
    {#if kind === "stars" || isShader(kind)}
      <canvas bind:this={canvas}></canvas>
    {:else if kind === "cover" && cover}
      <span class="cover" style:background-image={cover}></span>
    {/if}
  </div>
{/key}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .stars {
    background:
      radial-gradient(
        circle at 85% 10%,
        color-mix(in srgb, var(--accent) 22%, transparent),
        transparent 45%
      ),
      radial-gradient(circle at 10% 90%, rgb(52 224 210 / 0.08), transparent 40%);
  }

  canvas {
    width: 100%;
    height: 100%;
    color: var(--text);
  }

  .cover {
    position: absolute;
    inset: 0;
    background-size: 220% 220%;
    opacity: 0.55;
    animation: flow 24s ease-in-out infinite alternate;
  }

  @keyframes flow {
    from {
      background-position: 0% 0%;
    }
    to {
      background-position: 100% 100%;
    }
  }
</style>
