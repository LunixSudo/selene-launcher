import type { Loader } from "$lib/api";

export const LOADERS: Loader[] = ["fabric", "quilt"];

export const LOADER_NAMES: Record<Loader, string> = {
  fabric: "Fabric",
  quilt: "Quilt",
};
