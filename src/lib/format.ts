import { locale, t } from "$lib/i18n/index.svelte";

export function playTime(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return t("instances.minutes", { minutes: Math.max(1, minutes) });
  }
  return t("instances.hours", { hours: Math.floor(minutes / 60), minutes: minutes % 60 });
}

/** "5 minutes ago", "yesterday" and so on, in the current language. */
export function relativeTime(date: string): string {
  const minutes = (new Date(date).getTime() - Date.now()) / 60_000;
  const format = new Intl.RelativeTimeFormat(locale(), { numeric: "auto" });
  if (Math.abs(minutes) < 60) {
    return format.format(Math.round(minutes), "minute");
  }
  if (Math.abs(minutes) < 60 * 24) {
    return format.format(Math.round(minutes / 60), "hour");
  }
  return format.format(Math.round(minutes / 60 / 24), "day");
}
