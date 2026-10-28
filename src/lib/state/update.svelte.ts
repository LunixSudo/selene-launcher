import { api, type LauncherUpdate } from "$lib/api";

export const launcherUpdate = $state({
  available: null as LauncherUpdate | null,
  checking: false,
  installing: false,
  checked: false,
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
  try {
    await api.installUpdate();
  } finally {
    launcherUpdate.installing = false;
  }
}
