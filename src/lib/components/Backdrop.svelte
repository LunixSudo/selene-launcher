<script lang="ts">
  import { fade } from "svelte/transition";
  import { SHADER_BACKDROPS, type ShaderBackdrop, startShader } from "$lib/backdrops";
  import type { Backdrop } from "$lib/state/settings.svelte";

  const STAR_COUNT = 170;
  /** Shooting stars come every few seconds, sometimes two or three together. */
  const SHOOTING_PAUSE_MS: [number, number] = [2500, 9000];
  const SHOOTING_SPEED: [number, number] = [220, 1300];
  const SHOOTING_GROUP_CHANCE = 0.25;
  /** Down and to the left, about 30 degrees below the horizon. */
  const SHOOTING_ANGLE = Math.PI * 0.83;

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

  interface ShootingStar {
    x: number;
    y: number;
    speed: number;
    angle: number;
    length: number;
  }

  function between([min, max]: [number, number]): number {
    return min + Math.random() * (max - min);
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
    let shooting: ShootingStar[] = [];
    let nextShooting = performance.now() + between(SHOOTING_PAUSE_MS);
    let lastFrame = performance.now();
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

      if (!still && now >= nextShooting) {
        const count = Math.random() < SHOOTING_GROUP_CHANCE ? 2 + Math.round(Math.random()) : 1;
        for (let i = 0; i < count; i++) {
          shooting.push(launchShootingStar(width, height));
        }
        nextShooting = now + between(SHOOTING_PAUSE_MS);
      }
      // A long pause, like a hidden window, should not make them jump ahead.
      const seconds = Math.min(now - lastFrame, 100) / 1000;
      lastFrame = now;
      context.lineWidth = 1.6;
      context.lineCap = "round";
      for (const star of shooting) {
        star.x += Math.cos(star.angle) * star.speed * seconds;
        star.y += Math.sin(star.angle) * star.speed * seconds;
        const tailX = star.x - Math.cos(star.angle) * star.length;
        const tailY = star.y - Math.sin(star.angle) * star.length;
        const trail = context.createLinearGradient(star.x, star.y, tailX, tailY);
        trail.addColorStop(0, color);
        trail.addColorStop(1, "transparent");
        context.globalAlpha = 1;
        context.strokeStyle = trail;
        context.beginPath();
        context.moveTo(star.x, star.y);
        context.lineTo(tailX, tailY);
        context.stroke();
      }
      shooting = shooting.filter((star) => star.x > -star.length && star.y < height + star.length);
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

  /** Starts just past the top or right edge, so it flies in instead of appearing. */
  function launchShootingStar(width: number, height: number): ShootingStar {
    const speed = between(SHOOTING_SPEED);
    const length = 60 + speed * 0.12;
    const angle = SHOOTING_ANGLE + (Math.random() - 0.5) * 0.2;
    const fromTop = Math.random() < width / (width + height);
    return {
      x: fromTop ? between([width * 0.25, width + length]) : width + length,
      y: fromTop ? -length : between([-length, height * 0.5]),
      speed,
      angle,
      length,
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
