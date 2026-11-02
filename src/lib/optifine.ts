import { api } from "#lib/api.ts";

let listed: Promise<string[]> | null = null;

/** Whether OptiFine has a build for this game version. Its site is asked once per run. */
export async function hasOptifineBuild(gameVersion: string): Promise<boolean> {
  listed ??= api.optifineVersions().catch(() => {
    listed = null;
    return [];
  });
  return (await listed).includes(gameVersion);
}
