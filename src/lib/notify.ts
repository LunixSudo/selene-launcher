import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { settings } from "#lib/state/settings.svelte.ts";
import { type Toast, toast } from "#lib/state/toasts.svelte.ts";

/**
 * Tells the player something finished: in the corner of the launcher while it is in front,
 * otherwise as a Windows notification.
 */
export async function notify(title: string, body: string, action?: Toast["action"]) {
  if (document.hasFocus()) {
    toast({ title, body, action });
    return;
  }
  if (!settings.notifications) {
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
