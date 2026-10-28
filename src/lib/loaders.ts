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

/** Whether some builds are betas that `releaseOptions` hides until asked. */
export function hasHiddenBetas(releases: LoaderRelease[]): boolean {
  const preferred = preferredRelease(releases);
  return releases.some((release) => !release.stable && release !== preferred);
}

/**
 * Select options for loader builds, marking the preferred one and betas. Betas are left out
 * unless `showBetas`, except the preferred build and `keep`, the one already chosen.
 */
export function releaseOptions(
  releases: LoaderRelease[],
  showBetas: boolean,
  keep?: string,
): { value: string; label: string }[] {
  const preferred = preferredRelease(releases);
  return releases
    .filter(
      (release) => showBetas || release.stable || release === preferred || release.version === keep,
    )
    .map((release) => ({
      value: release.version,
      label:
        release === preferred
          ? `${release.version} (${t("create.recommended")})`
          : release.stable
            ? release.version
            : `${release.version} (${t("create.unstable")})`,
    }));
}
