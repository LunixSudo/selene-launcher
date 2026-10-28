import type { Instance } from "$lib/api";
import { coverOf, iconOf } from "$lib/appearance";

const SIZE = 256;
const RADIUS = 56;
const ICON_SCALE = 6.4;

/** Draws the instance's cover and icon the way its card shows them, as PNG bytes. */
export async function shortcutIcon(instance: Instance): Promise<number[]> {
  const canvas = document.createElement("canvas");
  canvas.width = SIZE;
  canvas.height = SIZE;
  const context = canvas.getContext("2d");
  if (!context) {
    return [];
  }

  context.beginPath();
  context.roundRect(0, 0, SIZE, SIZE, RADIUS);
  context.clip();
  context.fillStyle = gradient(context, coverOf(instance.id, instance.cover));
  context.fillRect(0, 0, SIZE, SIZE);

  const offset = (SIZE - 24 * ICON_SCALE) / 2;
  context.translate(offset, offset);
  context.scale(ICON_SCALE, ICON_SCALE);
  context.strokeStyle = "#ffffff";
  context.lineWidth = 1.7;
  context.lineCap = "round";
  context.lineJoin = "round";
  context.shadowColor = "rgb(0 0 0 / 0.25)";
  context.shadowBlur = 8;
  context.stroke(new Path2D(iconOf(instance.icon)));

  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  return blob ? Array.from(new Uint8Array(await blob.arrayBuffer())) : [];
}

/** Covers are written as `linear-gradient(135deg, #a, #b 55%, #c)`. */
function gradient(context: CanvasRenderingContext2D, css: string): CanvasGradient {
  const fill = context.createLinearGradient(0, 0, SIZE, SIZE);
  const stops = css.match(/#[0-9a-f]{6}(?:\s+\d+%)?/gi) ?? ["#5b7cff"];
  stops.forEach((stop, index) => {
    const [color, percent] = stop.split(/\s+/);
    const at = percent ? Number.parseFloat(percent) / 100 : index / Math.max(stops.length - 1, 1);
    fill.addColorStop(at, color);
  });
  return fill;
}
