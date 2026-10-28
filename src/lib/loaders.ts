import type { Loader, LoaderRelease } from "$lib/api";

export const LOADERS: Loader[] = ["fabric", "neoforge", "forge", "quilt"];

export const LOADER_NAMES: Record<Loader, string> = {
  fabric: "Fabric",
  quilt: "Quilt",
  forge: "Forge",
  neoforge: "NeoForge",
};

/** The build to pick by default: the one Forge recommends, else the newest stable one. */
export function preferredRelease(releases: LoaderRelease[]): LoaderRelease | undefined {
  return (
    releases.find((release) => release.recommended) ??
    releases.find((release) => release.stable) ??
    releases[0]
  );
}
