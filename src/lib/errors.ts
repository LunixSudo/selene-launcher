import { isCommandError } from "#lib/api.ts";
import { t } from "#lib/i18n/index.svelte.ts";

export function describeError(error: unknown): string {
  return t(`errors.${isCommandError(error) ? error.kind : "core"}`);
}
