import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  api,
  type CommandError,
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
  | { status: "preparing"; stage: LaunchProgress["stage"] | "importing" | null; percent: number }
  | { status: "running"; since: number }
  | { status: "failed"; error: CommandError };

const UNKNOWN_ERROR: CommandError = { kind: "core", message: "unknown error" };

export const library = $state({
  instances: [] as Instance[],
  loaded: false,
  loadError: null as CommandError | null,
  importing: 0,
  importError: null as CommandError | null,
  importManual: [] as ManualDownload[],
  launches: {} as Record<string, LaunchState>,
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
  const task = api
    .prepareInstance(id)
    // A failed download is retried, and reported, when the instance starts.
    .catch(() => {})
    .finally(() => {
      preparing.delete(id);
      if (library.launches[id]?.status === "preparing") {
        delete library.launches[id];
      }
    });
  preparing.set(id, task);
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
    library.importError = isCommandError(error) ? error : UNKNOWN_ERROR;
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
  delete settings.instanceBackdrops[id];
}

/** Starts the game, joining `server` right away when given. */
export async function play(id: string, server: string | null = null) {
  await preparing.get(id);
  const current = library.launches[id]?.status;
  // A shortcut clicked while the game runs or starts must not hide its Stop button.
  if (current === "running" || current === "preparing") {
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
      library.launches[payload.id] = { status: "preparing", stage: "importing", percent: 0 };
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
        library.crashes[payload.instanceId] = payload.crash;
      }
      void loadInstances();
      if (settings.afterLaunch === "minimize" && settings.restoreAfterGame) {
        const appWindow = getCurrentWindow();
        void appWindow.unminimize().then(() => appWindow.setFocus());
      }
    }),
  ]);
  Object.assign(library.crashes, await api.takeMissedCrashes());
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
