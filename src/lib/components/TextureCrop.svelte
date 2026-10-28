<script lang="ts">
  /** Draws one rectangle of a texture, plus an optional overlay from elsewhere in it. */
  let {
    src,
    x,
    y,
    width,
    height,
    overlay,
    onerror,
  }: {
    src: string;
    x: number;
    y: number;
    width: number;
    height: number;
    overlay?: [number, number];
    onerror?: () => void;
  } = $props();

  let canvas: HTMLCanvasElement;

  $effect(() => {
    const image = new Image();
    image.crossOrigin = "anonymous";
    image.onload = () => {
      const context = canvas.getContext("2d");
      if (!context) {
        return;
      }
      context.clearRect(0, 0, width, height);
      context.drawImage(image, x, y, width, height, 0, 0, width, height);
      if (overlay) {
        context.drawImage(image, overlay[0], overlay[1], width, height, 0, 0, width, height);
      }
    };
    image.onerror = () => onerror?.();
    image.src = src;
  });
</script>

<canvas bind:this={canvas} {width} {height} aria-hidden="true"></canvas>

<style>
  canvas {
    display: block;
    width: 100%;
    height: 100%;
    image-rendering: pixelated;
  }
</style>
