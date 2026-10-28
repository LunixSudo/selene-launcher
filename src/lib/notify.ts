import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { settings } from "$lib/state/settings.svelte";

/** Shows a Windows notification, but only while the launcher is in the background. */
export async function notify(title: string, body: string) {
  if (!settings.notifications || document.hasFocus()) {
    return;
  }
  try {
    const allowed = (await isPermissionGranted()) || (await requestPermission()) === "granted";
    if (allowed) {
      sendNotification({ title, body });
    }
  } catch {
    // Notifications are a nicety; the launcher works the same without them.
  }
}
