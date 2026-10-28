import { t } from "$lib/i18n/index.svelte";

export function playTime(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return t("instances.minutes", { minutes: Math.max(1, minutes) });
  }
  return t("instances.hours", { hours: Math.floor(minutes / 60), minutes: minutes % 60 });
}
