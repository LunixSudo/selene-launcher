<script lang="ts">
  import { fade } from "svelte/transition";
  import type { Backdrop } from "$lib/state/settings.svelte";

  const STAR_COUNT = 170;
  const SHOOTING_STAR_EVERY_MS = 7000;

  let { kind, cover = null }: { kind: Backdrop | "cover"; cover?: string | null } = $props();

  let canvas = $state<HTMLCanvasElement>();

  $effect(() => {
    if (kind === "stars" && canvas) {
      return drawStars(canvas);
    }
  });

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
        const age = (now - shooting.born) / 900;
        if (age >= 1) {
          shooting = null;
        } else {
          const headX = (shooting.x - age * 0.35) * width;
          const headY = (shooting.y + age * 0.2) * height;
          const trail = context.createLinearGradient(headX, headY, headX + 120, headY - 70);
          trail.addColorStop(0, color);
          trail.addColorStop(1, "transparent");
          context.globalAlpha = 1 - age;
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
    {#if kind === "halo"}
      <span class="ring one"></span>
      <span class="ring two"></span>
      <span class="ring three"></span>
    {:else if kind === "rays"}
      <span class="beams"></span>
      <span class="glare"></span>
    {:else if kind === "aurora"}
      <span class="blob one"></span>
      <span class="blob two"></span>
      <span class="blob three"></span>
    {:else if kind === "stars"}
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

  .ring {
    position: absolute;
    top: 50%;
    left: 55%;
    width: 58vmin;
    height: 58vmin;
    border: 5vmin solid;
    border-radius: 47% 53% 44% 56% / 52% 45% 55% 48%;
    filter: blur(16px);
    opacity: 0.6;
    mix-blend-mode: screen;
    translate: -50% -50%;
    animation: spin 26s linear infinite;
  }

  .ring.one {
    color: var(--accent);
  }

  .ring.two {
    margin: -3vmin 0 0 5vmin;
    border-radius: 55% 45% 58% 42% / 44% 56% 46% 54%;
    color: #ff4fd8;
    animation-duration: 33s;
    animation-direction: reverse;
  }

  .ring.three {
    margin: 4vmin 0 0 -4vmin;
    width: 50vmin;
    height: 50vmin;
    border-radius: 42% 58% 51% 49% / 57% 43% 53% 47%;
    color: #34e0d2;
    animation-duration: 41s;
  }

  .beams {
    position: absolute;
    inset: -20%;
    background: repeating-conic-gradient(
      from 190deg at 20% 0%,
      transparent 0deg,
      color-mix(in srgb, var(--accent) 38%, transparent) 4deg,
      transparent 9deg,
      rgb(255 255 255 / 0.06) 12deg,
      transparent 16deg
    );
    filter: blur(10px);
    mask-image: radial-gradient(ellipse at 20% 0%, #000 10%, transparent 70%);
    transform-origin: 20% 0%;
    animation: sway 14s ease-in-out infinite alternate;
  }

  .glare {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      115deg,
      transparent 40%,
      rgb(255 255 255 / 0.1) 47%,
      rgb(255 255 255 / 0.22) 49%,
      rgb(255 255 255 / 0.05) 53%,
      transparent 60%
    );
    background-size: 300% 100%;
    animation: sweep 9s ease-in-out infinite;
  }

  .blob {
    position: absolute;
    width: 70vmax;
    height: 45vmax;
    border-radius: 50%;
    filter: blur(70px);
    opacity: 0.45;
    animation: drift 22s ease-in-out infinite alternate;
  }

  .blob.one {
    top: -20%;
    left: -10%;
    background: #2fe0a2;
  }

  .blob.two {
    top: -10%;
    right: -20%;
    background: var(--accent);
    animation-duration: 28s;
    animation-direction: alternate-reverse;
  }

  .blob.three {
    bottom: -30%;
    left: 20%;
    background: #3f8cff;
    animation-duration: 34s;
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

  @keyframes spin {
    from {
      rotate: 0deg;
      scale: 1;
    }
    50% {
      scale: 1.08;
    }
    to {
      rotate: 360deg;
      scale: 1;
    }
  }

  @keyframes sway {
    from {
      rotate: -5deg;
    }
    to {
      rotate: 6deg;
    }
  }

  @keyframes sweep {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: 0% 0;
    }
  }

  @keyframes drift {
    from {
      translate: 0 0;
    }
    to {
      translate: 8vmax 6vmax;
    }
  }

  @keyframes flow {
    from {
      background-position: 0% 0%;
    }
    to {
      background-position: 100% 100%;
    }
  }

  @media (prefers-color-scheme: light) {
    :global(:root:not([data-theme="dark"])) .ring {
      mix-blend-mode: normal;
      opacity: 0.4;
    }
  }

  :global(:root[data-theme="light"]) .ring {
    mix-blend-mode: normal;
    opacity: 0.4;
  }
</style>
