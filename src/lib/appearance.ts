// Built-in icons and covers for instance cards. Icons are 24×24 stroke paths.

export const ICONS: Record<string, string> = {
  cube: "M12 3l8 4.5v9L12 21l-8-4.5v-9L12 3zM4 7.5l8 4.5 8-4.5M12 12v9",
  sword: "M19.5 4.5L9 15M19.5 4.5h-3.5M19.5 4.5V8M7 13l4 4M8.5 15.5L4.5 19.5",
  pickaxe: "M5 19L14 10M6.5 6.5c4-2.5 8.5-2 11.5 1M17.5 6.5l-2 2M18 7.5c1.5 2 2 4.5 1.5 6.5",
  gem: "M7 4h10l4 5-9 11L3 9l4-5zM3 9h18M10 4l2 16M14 4l-2 16",
  tree: "M12 3l6 8h-3l4 5H5l4-5H6l6-8zM12 16v5",
  flame: "M12 3c1 4 6 6 6 11a6 6 0 0 1-12 0c0-3 2-4 3-6 1 2 2 2 3 2-1-3-1-5 0-7z",
  moon: "M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z",
  mountain: "M3 19l6-10 4 6 3-4 5 8H3z",
  star: "M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1-4.4-4.3 6.1-.9L12 3z",
  heart: "M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10z",
  shield: "M12 3l7 3v5c0 5-3 8.5-7 10-4-1.5-7-5-7-10V6l7-3z",
  rocket:
    "M12 3c3 2 5 6 4 11l-1.5 3h-5L8 14c-1-5 1-9 4-11zM12 8.5v2M8 13l-3 3.5 3.5.5M16 13l3 3.5-3.5.5M10.5 20h3",
};

export const COVERS: Record<string, string> = {
  aurora: "linear-gradient(135deg, #3ad6c4, #5b7cff 55%, #a35bff)",
  sunset: "linear-gradient(135deg, #ff9a5b, #ff5b8a 55%, #8a5bff)",
  ocean: "linear-gradient(135deg, #22c1ee, #2a6bd8 60%, #1d3f9e)",
  forest: "linear-gradient(135deg, #a8e063, #3c9d5d 55%, #1f5f4a)",
  lava: "linear-gradient(135deg, #ffd25b, #ff7a3d 50%, #c4292b)",
  night: "linear-gradient(135deg, #3b3f8f, #1f2253 55%, #0d0f2b)",
  candy: "linear-gradient(135deg, #ffb3e6, #c39bff 55%, #8fd3ff)",
  stone: "linear-gradient(135deg, #b8bcc8, #7d8394 55%, #4b505e)",
};

export const DEFAULT_ICON = "cube";

// Instances without a chosen cover still get a stable one, picked from the id.
export function coverOf(id: string, cover: string | null): string {
  if (cover && cover in COVERS) {
    return COVERS[cover];
  }
  const names = Object.keys(COVERS);
  return COVERS[names[hash(id) % names.length]];
}

export function iconOf(icon: string | null): string {
  return ICONS[icon && icon in ICONS ? icon : DEFAULT_ICON];
}

function hash(text: string): number {
  let value = 0;
  for (const char of text) {
    value = (value * 31 + char.charCodeAt(0)) >>> 0;
  }
  return value;
}
