import type { Loader, LoaderRelease } from "$lib/api";
import { t } from "$lib/i18n/index.svelte";

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

/** Select options for loader builds, marking the preferred one and betas. */
export function releaseOptions(releases: LoaderRelease[]): { value: string; label: string }[] {
  const preferred = preferredRelease(releases);
  return releases.map((release) => ({
    value: release.version,
    label:
      release === preferred
        ? `${release.version} (${t("create.recommended")})`
        : release.stable
          ? release.version
          : `${release.version} (${t("create.unstable")})`,
  }));
}
