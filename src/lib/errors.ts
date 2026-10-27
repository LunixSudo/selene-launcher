import { isCommandError } from "$lib/api";
import { t } from "$lib/i18n/index.svelte";

export function describeError(error: unknown): string {
  return t(`errors.${isCommandError(error) ? error.kind : "core"}`);
}
