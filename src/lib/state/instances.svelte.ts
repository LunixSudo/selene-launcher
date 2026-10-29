import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { goto } from "$app/navigation";
import {
  api,
  type CommandError,
  type ContentProgress,
  type ContentStage,
  type CrashReport,
  type DiscordPresence,
  type GameExited,
  type GameLog,
  type Instance,
  isCommandError,
  type LaunchProgress,
  type LoaderVersion,
  type ManualDownload,
} from "$lib/api";
import { t } from "$lib/i18n/index.svelte";
import { LOADER_NAMES } from "$lib/loaders";
import { notify } from "$lib/notify";
import { settings } from "./settings.svelte";

export type LaunchState =
  | { status: "preparing"; stage: LaunchProgress["stage"] | null; percent: number }
  | { status: "running"; since: number }
  | { status: "failed"; error: CommandError };

/** An add-on download of an instance: an import, an install or an update. */
export interface ContentTask {
  stage: ContentStage;
  filesDone: number;
  filesTotal: number;
  percent: number;
}

const UNKNOWN_ERROR: CommandError = { kind: "core", message: "unknown error" };

export const library = $state({
  instances: [] as Instance[],
  loaded: false,
  loadError: null as CommandError | null,
  importing: 0,
  importError: null as CommandError | null,
  importManual: [] as ManualDownload[],
  launches: {} as Record<string, LaunchState>,
  tasks: {} as Record<string, ContentTask>,
  crashes: {} as Record<string, CrashReport>,
});

export async function loadInstances() {
  try {
    library.instances = await api.instances();
    library.loadError = null;
  } catch (err) {
    library.loadError = isCommandError(err) ? err : UNKNOWN_ERROR;
  }
  library.loaded = true;
}

export async function createInstance(name: string, version: string, loader: LoaderVersion | null) {
  const instance = await api.createInstance(name, version, loader);
  library.instances = [instance, ...library.instances];
  prepare(instance.id);
}

/** Background downloads of new instances, which a start has to wait for. */
const preparing = new Map<string, Promise<void>>();

/** Downloads the instance's game right away, so the first start is quick. */
export function prepare(id: string) {
  const task: Promise<void> = api
    .prepareInstance(id)
    // A failed download is retried, and reported, when the instance starts.
    .catch(() => {})
    .finally(() => {
      // A newer download may have taken over after this one was cancelled.
      if (preparing.get(id) !== task) {
        return;
      }
      preparing.delete(id);
      if (library.launches[id]?.status === "preparing") {
        delete library.launches[id];
      }
    });
  preparing.set(id, task);
}

export function isPreparing(id: string): boolean {
  return preparing.has(id);
}

/** What the instance's Play button says while something downloads, and how far it got. */
export function activity(id: string): { label: string; percent: number } | null {
  const task = library.tasks[id];
  if (task) {
    const label =
      task.filesTotal > 0
        ? t(`launch.${task.stage}Count`, { done: task.filesDone, total: task.filesTotal })
        : t(`launch.${task.stage}`);
    return { label, percent: task.percent };
  }
  const launch = library.launches[id];
  if (launch?.status === "preparing") {
    const label = launch.stage
      ? t(`launch.${launch.stage}`, { percent: launch.percent })
      : t("launch.starting");
    return { label, percent: launch.percent };
  }
  return null;
}

export async function importPack(path: string) {
  library.importing++;
  library.importError = null;
  library.importManual = [];
  try {
    const result = await api.importPack(path);
    delete library.launches[result.instance.id];
    prepare(result.instance.id);
    library.importManual = result.manual;
    void notify(t("notify.imported"), t("notify.importedBody", { name: result.instance.name }));
  } catch (error) {
    // Deleting the instance while it imports cancels the import; that is no error.
    if (!isCommandError(error) || error.kind !== "cancelled") {
      library.importError = isCommandError(error) ? error : UNKNOWN_ERROR;
    }
  } finally {
    library.importing--;
    // Reread from disk: the list on screen may have been loaded while the import was running.
    await loadInstances();
  }
}

export function replaceInstance(updated: Instance) {
  library.instances = library.instances.map((instance) =>
    instance.id === updated.id ? updated : instance,
  );
}

export async function deleteInstance(id: string) {
  await api.deleteInstance(id);
  library.instances = library.instances.filter((instance) => instance.id !== id);
  delete library.launches[id];
  delete library.tasks[id];
  delete settings.instanceBackdrops[id];
}

/** Starts the game, joining `server` right away when given. */
export async function play(id: string, server: string | null = null) {
  await preparing.get(id);
  const current = library.launches[id]?.status;
  // A shortcut clicked while the game runs or starts must not hide its Stop button, and the
  // game must not start on half-swapped add-ons.
  if (current === "running" || current === "preparing" || library.tasks[id]) {
    return;
  }
  library.launches[id] = { status: "preparing", stage: null, percent: 0 };
  delete library.crashes[id];
  try {
    await api.launchInstance(id, server);
  } catch (error) {
    if (isCommandError(error) && error.kind === "cancelled") {
      delete library.launches[id];
      return;
    }
    library.launches[id] = {
      status: "failed",
      error: isCommandError(error) ? error : UNKNOWN_ERROR,
    };
  }
}

/** What Discord shows under the launcher's name: the running instance, or nothing much. */
export function discordPresence(): DiscordPresence {
  for (const [id, launch] of Object.entries(library.launches)) {
    const instance = library.instances.find((instance) => instance.id === id);
    if (launch.status === "running" && instance) {
      const loader = instance.loader ? ` · ${LOADER_NAMES[instance.loader.loader]}` : "";
      return {
        details: instance.name,
        state: `Minecraft ${instance.version}${loader}`,
        startedAt: Math.floor(launch.since / 1000),
      };
    }
  }
  return { details: t("discord.idle"), state: null, startedAt: null };
}

const logListeners = new Set<(log: GameLog) => void>();

/** Calls `listener` with each batch of lines a running game prints. Returns a function that unsubscribes. */
export function onGameLog(listener: (log: GameLog) => void): () => void {
  logListeners.add(listener);
  return () => logListeners.delete(listener);
}

/** Opens the instance page, where the crash notice says what went wrong. */
function showCrash(id: string, crash: CrashReport) {
  library.crashes[id] = crash;
  void goto(`/instance/${id}`);
}

/** Subscribes to game events from the backend. Returns a function that unsubscribes. */
export async function listenToGames(): Promise<() => void> {
  const unlisteners = await Promise.all([
    listen<LaunchProgress>("launch-progress", ({ payload }) => {
      library.launches[payload.instanceId] = {
        status: "preparing",
        stage: payload.stage,
        percent: Math.floor((payload.bytesDone / Math.max(payload.bytesTotal, 1)) * 100),
      };
    }),
    listen<Instance>("instance-importing", ({ payload }) => {
      if (!library.instances.some((instance) => instance.id === payload.id)) {
        library.instances = [payload, ...library.instances];
      }
      library.tasks[payload.id] ??= { stage: "importing", filesDone: 0, filesTotal: 0, percent: 0 };
    }),
    listen<ContentProgress>("content-progress", ({ payload }) => {
      if (payload.stage === null) {
        delete library.tasks[payload.instanceId];
        return;
      }
      library.tasks[payload.instanceId] = {
        stage: payload.stage,
        filesDone: payload.filesDone,
        filesTotal: payload.filesTotal,
        percent: Math.floor((payload.bytesDone / Math.max(payload.bytesTotal, 1)) * 100),
      };
    }),
    listen<string>("game-started", ({ payload }) => {
      library.launches[payload] = { status: "running", since: Date.now() };
      const name = library.instances.find((instance) => instance.id === payload)?.name ?? payload;
      void notify(t("notify.gameReady", { name }), t("notify.gameReadyBody"));
      void loadInstances();
      const appWindow = getCurrentWindow();
      if (settings.afterLaunch === "minimize") {
        void appWindow.minimize();
      } else if (settings.afterLaunch === "close") {
        void appWindow.close();
      }
    }),
    listen<string>("launch-request", ({ payload }) => {
      void play(payload);
    }),
    listen<GameLog>("game-log", ({ payload }) => {
      for (const listener of logListeners) {
        listener(payload);
      }
    }),
    listen<GameExited>("game-exited", ({ payload }) => {
      delete library.launches[payload.instanceId];
      if (payload.crash) {
        showCrash(payload.instanceId, payload.crash);
      }
      void loadInstances();
      if (settings.afterLaunch === "minimize" && settings.restoreAfterGame) {
        const appWindow = getCurrentWindow();
        void appWindow.unminimize().then(() => appWindow.setFocus());
      }
    }),
  ]);
  // The launcher opens again after a crash it closed for; it goes straight to the instance.
  for (const [id, crash] of Object.entries(await api.takeMissedCrashes())) {
    showCrash(id, crash);
  }
  for (const id of await api.runningGames()) {
    library.launches[id] ??= { status: "running", since: Date.now() };
  }
  const requested = await api.takeStartupLaunch();
  if (requested) {
    void play(requested);
  }
  return () => {
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
}
