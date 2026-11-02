import type { CatalogItem, Loader } from "#lib/api.ts";

/** Modrinth projects that speed the game up without changing how it plays. */
const SODIUM = "AANobbMI";
const EMBEDDIUM = "sk9rgfiA";
const LITHIUM = "gvQqBUqZ";
const FERRITE_CORE = "uXXizFIs";
const ENTITY_CULLING = "NNAgCjsB";
const IMMEDIATELY_FAST = "5ZwdcRci";

/** Sodium came to NeoForge with 1.21; Embeddium covers it before that and covers Forge. */
function renderer(loader: Loader, gameVersion: string): string {
  if (loader === "fabric" || loader === "quilt") {
    return SODIUM;
  }
  // Versions since 2026 are numbered by year, like 26.1.
  const [major = 0, minor = 0] = gameVersion.split(".").map(Number);
  const since121 = major > 1 || minor >= 21;
  return loader === "neoforge" && since121 ? SODIUM : EMBEDDIUM;
}

/**
 * The set for a new instance. Projects without a build for this version are skipped when
 * installing, so the list may name more than ends up in the folder.
 */
export function optimizers(loader: Loader, gameVersion: string): CatalogItem[] {
  const ids = [renderer(loader, gameVersion), FERRITE_CORE, ENTITY_CULLING, IMMEDIATELY_FAST];
  if (loader !== "forge") {
    ids.push(LITHIUM);
  }
  return ids.map((id) => ({ source: "modrinth", id }));
}
