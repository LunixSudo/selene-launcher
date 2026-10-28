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
  javaPath: string | null;
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

export interface LauncherUpdate {
  version: string;
  notes: string;
  url: string;
}

export interface DiscordPresence {
  details: string;
  state: string | null;
  /** Unix seconds; Discord counts the elapsed time from it. */
  startedAt: number | null;
}

export interface LauncherConfig {
  autoUpdateMods: boolean;
  maxDownloads: number | null;
  speedLimitKib: number | null;
}

export interface Account {
  id: string;
  name: string;
  skin: string | null;
}

export type SkinVariant = "classic" | "slim";

export interface SkinProfile {
  name: string;
  skin: { url: string; variant: SkinVariant } | null;
  capes: Cape[];
}

export interface Cape {
  id: string;
  name: string;
  url: string;
  active: boolean;
}

export interface SavedSkin {
  id: string;
  name: string;
  variant: SkinVariant;
  /** Data URL of the PNG. */
  image: string;
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

export interface PlannedFile {
  title: string;
  requested: boolean;
}

export interface InstallPreview {
  mods: PlannedFile[];
  manual: ManualDownload[];
}

export interface InstallResult {
  installed: number;
  manual: ManualDownload[];
}

export type ContentKind = "mod" | "resourcePack" | "shaderPack";
export type CatalogSort = "relevance" | "downloads" | "newest" | "updated";

export interface CategoryTag {
  id: string;
  name: string;
}

export interface ProjectRef {
  source: Source;
  id: string;
}

export type Issue = { kind: "requires"; title: string } | { kind: "conflictsWith"; title: string };

export interface InstalledFile {
  fileName: string;
  enabled: boolean;
  sha1: string;
  size: number;
  title: string | null;
  iconUrl: string | null;
  versionNumber: string | null;
  project: ProjectRef | null;
  issues: Issue[];
}

export interface ContentUpdate {
  fileName: string;
  title: string;
  from: string | null;
  to: string;
}

export interface VersionChange {
  updated: ContentUpdate[];
  incompatible: string[];
  unknown: string[];
}

export interface ProjectVersion {
  id: string;
  name: string;
  versionNumber: string;
  published: string | null;
  channel: "release" | "beta" | "alpha";
  changelog: string | null;
}

export interface ProjectPage {
  title: string;
  description: string;
  format: "markdown" | "html";
  body: string;
  iconUrl: string | null;
  gallery: string[];
  pageUrl: string;
  downloads: number;
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
  | "invalidSkin"
  | "noLoader"
  | "noCompatibleVersion"
  | "alreadyRunning"
  | "javaNotFound"
  | "spawn"
  | "network"
  | "core";

export interface CommandError {
  kind: ErrorKind;
  message: string;
}

export interface LaunchProgress {
  instanceId: string;
  stage: "mods" | "files" | "java";
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
    javaPath: string | null,
  ) =>
    invoke<Instance>("set_launch_options", { instanceId, window, fullscreen, javaArgs, javaPath }),
  config: () => invoke<LauncherConfig>("config"),
  createShortcut: (id: string) => invoke<void>("create_shortcut", { id }),
  takeStartupLaunch: () => invoke<string | null>("take_startup_launch"),
  checkUpdate: () => invoke<LauncherUpdate | null>("check_update"),
  installUpdate: () => invoke<void>("install_update"),
  setConfig: (config: LauncherConfig) => invoke<void>("set_config", { config }),
  skinProfile: () => invoke<SkinProfile>("skin_profile"),
  applySkin: (id: string) => invoke<SkinProfile>("apply_skin", { id }),
  resetSkin: () => invoke<SkinProfile>("reset_skin"),
  setCape: (id: string | null) => invoke<SkinProfile>("set_cape", { id }),
  savedSkins: () => invoke<SavedSkin[]>("saved_skins"),
  addSkin: (path: string, variant: SkinVariant) => invoke<SavedSkin>("add_skin", { path, variant }),
  setSkinVariant: (id: string, variant: SkinVariant) =>
    invoke<void>("set_skin_variant", { id, variant }),
  removeSkin: (id: string) => invoke<void>("remove_skin", { id }),
  setDiscordPresence: (presence: DiscordPresence | null) =>
    invoke<void>("set_discord_presence", { presence }),
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
  searchCatalog: (
    instanceId: string,
    kind: ContentKind,
    source: Source,
    query: string,
    category: string | null,
    sort: CatalogSort,
    page: number,
  ) =>
    invoke<CatalogPage>("search_catalog", {
      instanceId,
      kind,
      source,
      query,
      category,
      sort,
      page,
    }),
  catalogCategories: (kind: ContentKind, source: Source) =>
    invoke<CategoryTag[]>("catalog_categories", { kind, source }),
  installedProjects: (instanceId: string, kind: ContentKind) =>
    invoke<string[]>("installed_projects", { instanceId, kind }),
  installedContent: (instanceId: string, kind: ContentKind) =>
    invoke<InstalledFile[]>("installed_content", { instanceId, kind }),
  previewInstall: (instanceId: string, kind: ContentKind, items: CatalogItem[]) =>
    invoke<InstallPreview>("preview_install", { instanceId, kind, items }),
  installContent: (instanceId: string, kind: ContentKind, items: CatalogItem[]) =>
    invoke<InstallResult>("install_content", { instanceId, kind, items }),
  setContentEnabled: (instanceId: string, kind: ContentKind, fileName: string, enabled: boolean) =>
    invoke<void>("set_content_enabled", { instanceId, kind, fileName, enabled }),
  removeContent: (instanceId: string, kind: ContentKind, fileName: string) =>
    invoke<void>("remove_content", { instanceId, kind, fileName }),
  checkUpdates: (instanceId: string, kind: ContentKind) =>
    invoke<ContentUpdate[]>("check_updates", { instanceId, kind }),
  updateContent: (instanceId: string, kind: ContentKind) =>
    invoke<number>("update_content", { instanceId, kind }),
  projectVersions: (instanceId: string, kind: ContentKind, project: ProjectRef) =>
    invoke<ProjectVersion[]>("project_versions", { instanceId, kind, project }),
  curseforgeChangelog: (projectId: string, fileId: string) =>
    invoke<string>("curseforge_changelog", { projectId, fileId }),
  installVersion: (
    instanceId: string,
    kind: ContentKind,
    project: ProjectRef,
    versionId: string,
    fileName: string,
  ) => invoke<void>("install_version", { instanceId, kind, project, versionId, fileName }),
  previewVersionChange: (instanceId: string, version: string, loader: LoaderVersion | null) =>
    invoke<VersionChange>("preview_version_change", { instanceId, version, loader }),
  changeVersion: (instanceId: string, version: string, loader: LoaderVersion | null) =>
    invoke<Instance>("change_version", { instanceId, version, loader }),
  projectPage: (kind: ContentKind, project: ProjectRef) =>
    invoke<ProjectPage>("project_page", { kind, project }),
  importPack: (path: string) => invoke<ImportResult>("import_pack", { path }),
  exportPack: (instanceId: string, path: string, folders: string[], format: PackFormat) =>
    invoke<void>("export_pack", { instanceId, path, folders, format }),
};
