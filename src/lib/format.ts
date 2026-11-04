import { locale, t } from "#lib/i18n/index.svelte.ts";

export function playTime(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return t("instances.minutes", { minutes: Math.max(1, minutes) });
  }
  const hours = Math.floor(minutes / 60);
  return minutes % 60 === 0
    ? t("instances.wholeHours", { hours })
    : t("instances.hours", { hours, minutes: minutes % 60 });
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
