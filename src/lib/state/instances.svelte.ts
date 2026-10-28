import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  api,
  type CommandError,
  type CrashReport,
  type GameExited,
  type GameLog,
  type Instance,
  isCommandError,
  type LaunchProgress,
  type LaunchTarget,
  type LoaderVersion,
  type ManualDownload,
} from "$lib/api";
import { t } from "$lib/i18n/index.svelte";
import { notify } from "$lib/notify";
import { settings } from "./settings.svelte";

export type LaunchState =
  | { status: "preparing"; stage: LaunchProgress["stage"] | null; percent: number }
  | { status: "running" }
  | { status: "failed"; error: CommandError };

const UNKNOWN_ERROR: CommandError = { kind: "core", message: "unknown error" };

export const library = $state({
  instances: [] as Instance[],
  loaded: false,
  importing: 0,
  importError: null as CommandError | null,
  importManual: [] as ManualDownload[],
  launches: {} as Record<string, LaunchState>,
  crashes: {} as Record<string, CrashReport>,
});

export async function loadInstances() {
  library.instances = await api.instances();
  library.loaded = true;
}

export async function createInstance(name: string, version: string, loader: LoaderVersion | null) {
  const instance = await api.createInstance(name, version, loader);
  library.instances = [instance, ...library.instances];
}

export async function importPack(path: string) {
  library.importing++;
  library.importError = null;
  library.importManual = [];
  try {
    const result = await api.importPack(path);
    library.instances = [result.instance, ...library.instances];
    library.importManual = result.manual;
    void notify(t("notify.imported"), t("notify.importedBody", { name: result.instance.name }));
  } catch (error) {
    library.importError = isCommandError(error) ? error : UNKNOWN_ERROR;
  } finally {
    library.importing--;
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
}

export async function play(id: string, target: LaunchTarget | null = null) {
  library.launches[id] = { status: "preparing", stage: null, percent: 0 };
  delete library.crashes[id];
  try {
    await api.launchInstance(id, target);
  } catch (error) {
    library.launches[id] = {
      status: "failed",
      error: isCommandError(error) ? error : UNKNOWN_ERROR,
    };
  }
}

const logListeners = new Set<(log: GameLog) => void>();

/** Calls `listener` for every line a running game prints. Returns a function that unsubscribes. */
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
    listen<string>("game-started", ({ payload }) => {
      library.launches[payload] = { status: "running" };
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
  return () => {
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
}
