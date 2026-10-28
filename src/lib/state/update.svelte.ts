import { api, type LauncherUpdate } from "$lib/api";
import { describeError } from "$lib/errors";

export const launcherUpdate = $state({
  available: null as LauncherUpdate | null,
  checking: false,
  installing: false,
  checked: false,
  error: null as string | null,
});

export async function checkForUpdate() {
  launcherUpdate.checking = true;
  try {
    launcherUpdate.available = await api.checkUpdate();
  } catch {
    // No connection or GitHub is down; try again next launch.
  } finally {
    launcherUpdate.checking = false;
    launcherUpdate.checked = true;
  }
}

export async function installUpdate() {
  launcherUpdate.installing = true;
  launcherUpdate.error = null;
  try {
    await api.installUpdate();
  } catch (err) {
    launcherUpdate.error = describeError(err);
  } finally {
    launcherUpdate.installing = false;
  }
}
