import type { Instance } from "#lib/api.ts";
import { coverOf, iconOf, sceneOf } from "#lib/appearance.ts";

/** Sizes Explorer shows, so it never has to scale an image down itself. */
const SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 256];

/** Draws the instance's cover and icon the way its card shows them, as PNGs of every size. */
export async function shortcutIcon(instance: Instance): Promise<number[][]> {
  const scene = await loadScene(instance);
  const images = await Promise.all(SIZES.map((size) => draw(instance, scene, size)));
  return images.filter((image) => image.length > 0);
}

interface Scene {
  image: HTMLImageElement;
  /** Where the square is cut from, 0 to 1 across and down. */
  focus: [number, number];
}

async function loadScene(instance: Instance): Promise<Scene | null> {
  const scene = sceneOf(instance.id, instance.cover);
  if (!scene) {
    return null;
  }
  const image = new Image();
  image.src = scene.src;
  try {
    await image.decode();
  } catch {
    return null;
  }
  const [x, y] = scene.focus.split(" ").map((part) => Number.parseFloat(part) / 100);
  return { image, focus: [x, y] };
}

async function draw(instance: Instance, scene: Scene | null, size: number): Promise<number[]> {
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const context = canvas.getContext("2d");
  if (!context) {
    return [];
  }
  const scale = size / 256;

  context.beginPath();
  context.roundRect(0, 0, size, size, 56 * scale);
  if (scene) {
    const { image, focus } = scene;
    const side = Math.min(image.naturalWidth, image.naturalHeight);
    const left = (image.naturalWidth - side) * focus[0];
    const top = (image.naturalHeight - side) * focus[1];
    context.save();
    context.clip();
    context.imageSmoothingQuality = "high";
    context.drawImage(image, left, top, side, side, 0, 0, size, size);
    context.restore();
  } else {
    context.fillStyle = gradient(context, coverOf(instance.id, instance.cover), size);
    context.fill();
  }

  const iconScale = 6.4 * scale;
  const offset = (size - 24 * iconScale) / 2;
  context.translate(offset, offset);
  context.scale(iconScale, iconScale);
  context.strokeStyle = "#ffffff";
  // Thin strokes vanish at small sizes, so they get relatively thicker there.
  context.lineWidth = Math.max(1.7, 1.2 / iconScale);
  context.lineCap = "round";
  context.lineJoin = "round";
  context.shadowColor = "rgb(0 0 0 / 0.25)";
  context.shadowBlur = 8 * scale;
  context.stroke(new Path2D(iconOf(instance.icon)));

  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  return blob ? Array.from(new Uint8Array(await blob.arrayBuffer())) : [];
}

/** Covers are written as `linear-gradient(135deg, #a, #b 55%, #c)`. */
function gradient(context: CanvasRenderingContext2D, css: string, size: number): CanvasGradient {
  const fill = context.createLinearGradient(0, 0, size, size);
  const stops = css.match(/#[0-9a-f]{6}(?:\s+\d+%)?/gi) ?? ["#5b7cff"];
  stops.forEach((stop, index) => {
    const [color, percent] = stop.split(/\s+/);
    const at = percent ? Number.parseFloat(percent) / 100 : index / Math.max(stops.length - 1, 1);
    fill.addColorStop(at, color);
  });
  return fill;
}
