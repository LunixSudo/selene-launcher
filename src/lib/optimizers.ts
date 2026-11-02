import type { CatalogItem, Loader } from "#lib/api.ts";

/** Modrinth projects that speed the game up without changing how it plays. */
const SODIUM = "AANobbMI";
const EMBEDDIUM = "sk9rgfiA";
const LITHIUM = "gvQqBUqZ";
const FERRITE_CORE = "uXXizFIs";
const ENTITY_CULLING = "NNAgCjsB";
const IMMEDIATELY_FAST = "5ZwdcRci";
const FOAM_FIX = "jupr7Bf5";
/** Fixes and speed-ups for 1.8.9 and 1.12.2, which have no Sodium of their own. */
const POLY_PATCHER = "YknNc5nN";

function parse(gameVersion: string): [number, number] {
  // Versions since 2026 are numbered by year, like 26.1.
  const [major = 0, minor = 0] = gameVersion.split(".").map(Number);
  return [major, minor];
}

/** Sodium came to NeoForge with 1.21; Embeddium covers it before that and covers Forge. */
function renderer(loader: Loader, gameVersion: string): string {
  if (loader === "fabric" || loader === "quilt") {
    return SODIUM;
  }
  const [major, minor] = parse(gameVersion);
  return loader === "neoforge" && (major > 1 || minor >= 21) ? SODIUM : EMBEDDIUM;
}

/**
 * Mods known to work together for this loader and version. Projects without a build for the
 * version are skipped when installing, so the list may name more than ends up in the folder.
 * A new renderer would clash with OptiFine, so it is left out next to it.
 */
export function optimizers(
  loader: Loader,
  gameVersion: string,
  hasOptiFine: boolean,
): CatalogItem[] {
  const [major, minor] = parse(gameVersion);
  const legacy = major === 1 && minor <= 12;
  const ids = legacy
    ? [POLY_PATCHER, ENTITY_CULLING, FOAM_FIX]
    : [
        ...(hasOptiFine ? [] : [renderer(loader, gameVersion)]),
        FERRITE_CORE,
        ENTITY_CULLING,
        IMMEDIATELY_FAST,
        ...(loader === "forge" ? [] : [LITHIUM]),
      ];
  return ids.map((id) => ({ source: "modrinth", id }));
}
