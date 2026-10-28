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
  icon: string | null;
  cover: string | null;
  playSeconds: number;
  window: WindowSize | null;
  fullscreen: boolean;
  javaArgs: string;
  group: string | null;
}

export interface WindowSize {
  width: number;
  height: number;
}

export type LaunchTarget = { kind: "world"; value: string } | { kind: "server"; value: string };

export type GameMode = "survival" | "creative" | "adventure" | "spectator" | "hardcore";

export interface World {
  folder: string;
  name: string;
  lastPlayed: string | null;
  gameMode: GameMode | null;
  icon: string | null;
}

export interface WorldList {
  worlds: World[];
  quickPlay: boolean;
}

export interface Server {
  name: string;
  address: string;
  icon: string | null;
}

export interface Screenshot {
  path: string;
  name: string;
  takenAt: string | null;
}

export interface Account {
  id: string;
  name: string;
}

export interface AccountList {
  accounts: Account[];
  active: string | null;
}

export type Source = "modrinth" | "curseforge";
export type PackFormat = "mrpack" | "curseforge";

export interface CatalogHit {
  source: Source;
  id: string;
  title: string;
  description: string;
  author: string;
  iconUrl: string | null;
  downloads: number;
  pageUrl: string;
}

export interface CatalogPage {
  hits: CatalogHit[];
  hasMore: boolean;
}

export interface ManualDownload {
  title: string;
  url: string;
}

export interface ImportResult {
  instance: Instance;
  manual: ManualDownload[];
}

export interface CatalogItem {
  source: Source;
  id: string;
}

export interface PlannedMod {
  title: string;
  requested: boolean;
}

export interface InstallPreview {
  mods: PlannedMod[];
  manual: ManualDownload[];
}

export interface InstallResult {
  installed: number;
  manual: ManualDownload[];
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
  | "appNotApproved"
  | "keyring"
  | "io"
  | "unknownVersion"
  | "unknownInstance"
  | "invalidPack"
  | "unsupportedLoader"
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

export type CrashCause =
  | "outOfMemory"
  | "wrongJava"
  | "missingDependency"
  | "incompatibleMods"
  | "modError"
  | "graphicsDriver"
  | "unknown";

export interface CrashReport {
  cause: CrashCause;
  details: string[];
}

export interface GameExited {
  instanceId: string;
  code: number | null;
  crash: CrashReport | null;
}

export interface GameLog {
  instanceId: string;
  line: string;
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
  updateInstance: (
    id: string,
    name: string,
    icon: string | null,
    cover: string | null,
    group: string | null,
  ) => invoke<Instance>("update_instance", { id, name, icon, cover, group }),
  duplicateInstance: (id: string, name: string) =>
    invoke<Instance>("duplicate_instance", { id, name }),
  openDataFolder: () => invoke<void>("open_data_folder"),
  setLaunchOptions: (
    instanceId: string,
    window: WindowSize | null,
    fullscreen: boolean,
    javaArgs: string,
  ) => invoke<Instance>("set_launch_options", { instanceId, window, fullscreen, javaArgs }),
  gameLog: (instanceId: string) => invoke<string[]>("game_log", { instanceId }),
  openInstanceFolder: (id: string) => invoke<void>("open_instance_folder", { id }),
  deleteInstance: (id: string) => invoke<void>("delete_instance", { id }),
  memoryInfo: (instanceId: string) => invoke<MemoryInfo>("memory_info", { instanceId }),
  setInstanceMemory: (instanceId: string, maxMemoryMb: number | null) =>
    invoke<Instance>("set_instance_memory", { instanceId, maxMemoryMb }),
  launchInstance: (id: string, target: LaunchTarget | null = null) =>
    invoke<void>("launch_instance", { id, target }),
  stopInstance: (id: string) => invoke<boolean>("stop_instance", { id }),
  worlds: (instanceId: string) => invoke<WorldList>("worlds", { instanceId }),
  openWorldFolder: (instanceId: string, folder: string) =>
    invoke<void>("open_world_folder", { instanceId, folder }),
  backUpWorld: (instanceId: string, folder: string) =>
    invoke<string>("back_up_world", { instanceId, folder }),
  deleteWorld: (instanceId: string, folder: string) =>
    invoke<void>("delete_world", { instanceId, folder }),
  servers: (instanceId: string) => invoke<Server[]>("servers", { instanceId }),
  saveServers: (instanceId: string, servers: Server[]) =>
    invoke<void>("save_servers", { instanceId, servers }),
  screenshots: (instanceId: string) => invoke<Screenshot[]>("screenshots", { instanceId }),
  openScreenshot: (instanceId: string, name: string) =>
    invoke<void>("open_screenshot", { instanceId, name }),
  deleteScreenshot: (instanceId: string, name: string) =>
    invoke<void>("delete_screenshot", { instanceId, name }),
  searchMods: (instanceId: string, source: Source, query: string, page: number) =>
    invoke<CatalogPage>("search_mods", { instanceId, source, query, page }),
  installedProjects: (instanceId: string) => invoke<string[]>("installed_projects", { instanceId }),
  installedMods: (instanceId: string) => invoke<InstalledMod[]>("installed_mods", { instanceId }),
  previewInstall: (instanceId: string, items: CatalogItem[]) =>
    invoke<InstallPreview>("preview_install", { instanceId, items }),
  installMods: (instanceId: string, items: CatalogItem[]) =>
    invoke<InstallResult>("install_mods", { instanceId, items }),
  setModEnabled: (instanceId: string, fileName: string, enabled: boolean) =>
    invoke<void>("set_mod_enabled", { instanceId, fileName, enabled }),
  removeMod: (instanceId: string, fileName: string) =>
    invoke<void>("remove_mod", { instanceId, fileName }),
  checkModUpdates: (instanceId: string) => invoke<ModUpdate[]>("check_mod_updates", { instanceId }),
  updateMods: (instanceId: string) => invoke<number>("update_mods", { instanceId }),
  importPack: (path: string) => invoke<ImportResult>("import_pack", { path }),
  exportPack: (instanceId: string, path: string, folders: string[], format: PackFormat) =>
    invoke<void>("export_pack", { instanceId, path, folders, format }),
};
