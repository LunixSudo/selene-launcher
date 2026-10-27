import { invoke } from "@tauri-apps/api/core";

export type VersionKind = "release" | "snapshot" | "old_beta" | "old_alpha";

export interface VersionSummary {
  id: string;
  kind: VersionKind;
  releaseTime: string;
}

export type Loader = "fabric" | "quilt";

export interface LoaderVersion {
  loader: Loader;
  version: string;
}

export interface LoaderRelease {
  version: string;
  stable: boolean;
}

export interface Instance {
  id: string;
  name: string;
  version: string;
  loader: LoaderVersion | null;
  createdAt: string;
  lastPlayed: string | null;
  maxMemoryMb: number | null;
}

export interface Account {
  id: string;
  name: string;
}

export interface AccountList {
  accounts: Account[];
  active: string | null;
}

export interface SearchHit {
  project_id: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  icon_url: string | null;
  downloads: number;
  display_categories: string[];
}

export interface SearchResults {
  hits: SearchHit[];
  total_hits: number;
}

export interface Project {
  id: string;
  slug: string;
  title: string;
  description: string;
  icon_url: string | null;
}

export interface InstalledMod {
  fileName: string;
  enabled: boolean;
  sha1: string;
  size: number;
  project: Project | null;
  versionNumber: string | null;
}

export interface ModUpdate {
  fileName: string;
  title: string;
  from: string | null;
  to: string;
}

export interface MemoryInfo {
  total: number;
  max: number;
  recommended: number;
}

export type ErrorKind =
  | "notSignedIn"
  | "signInCancelled"
  | "noXboxAccount"
  | "regionBlocked"
  | "childAccount"
  | "gameNotOwned"
  | "authService"
  | "keyring"
  | "io"
  | "unknownVersion"
  | "unknownInstance"
  | "noLoader"
  | "noCompatibleVersion"
  | "alreadyRunning"
  | "spawn"
  | "network"
  | "core";

export interface CommandError {
  kind: ErrorKind;
  message: string;
}

export interface LaunchProgress {
  instanceId: string;
  stage: "files" | "java";
  bytesDone: number;
  bytesTotal: number;
}

export interface GameExited {
  instanceId: string;
  code: number | null;
}

export function isCommandError(value: unknown): value is CommandError {
  return typeof value === "object" && value !== null && "kind" in value && "message" in value;
}

export const api = {
  accounts: () => invoke<AccountList>("accounts"),
  signIn: () => invoke<Account>("sign_in"),
  cancelSignIn: () => invoke<void>("cancel_sign_in"),
  signOut: (id: string) => invoke<void>("sign_out", { id }),
  setActiveAccount: (id: string) => invoke<void>("set_active_account", { id }),
  versions: () => invoke<VersionSummary[]>("versions"),
  instances: () => invoke<Instance[]>("instances"),
  loaderVersions: (loader: Loader, gameVersion: string) =>
    invoke<LoaderRelease[]>("loader_versions", { loader, gameVersion }),
  createInstance: (name: string, version: string, loader: LoaderVersion | null) =>
    invoke<Instance>("create_instance", { name, version, loader }),
  deleteInstance: (id: string) => invoke<void>("delete_instance", { id }),
  memoryInfo: (instanceId: string) => invoke<MemoryInfo>("memory_info", { instanceId }),
  setInstanceMemory: (instanceId: string, maxMemoryMb: number | null) =>
    invoke<Instance>("set_instance_memory", { instanceId, maxMemoryMb }),
  launchInstance: (id: string) => invoke<void>("launch_instance", { id }),
  searchMods: (instanceId: string, query: string, offset: number) =>
    invoke<SearchResults>("search_mods", { instanceId, query, offset }),
  installedMods: (instanceId: string) => invoke<InstalledMod[]>("installed_mods", { instanceId }),
  installMod: (instanceId: string, projectId: string) =>
    invoke<string[]>("install_mod", { instanceId, projectId }),
  setModEnabled: (instanceId: string, fileName: string, enabled: boolean) =>
    invoke<void>("set_mod_enabled", { instanceId, fileName, enabled }),
  removeMod: (instanceId: string, fileName: string) =>
    invoke<void>("remove_mod", { instanceId, fileName }),
  checkModUpdates: (instanceId: string) => invoke<ModUpdate[]>("check_mod_updates", { instanceId }),
  updateMods: (instanceId: string) => invoke<number>("update_mods", { instanceId }),
};
