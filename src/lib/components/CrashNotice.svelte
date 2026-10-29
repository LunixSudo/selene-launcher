<script lang="ts">
  import type { CrashReport } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";

  let {
    crash,
    onlogs,
    ondismiss,
  }: { crash: CrashReport; onlogs: () => void; ondismiss: () => void } = $props();
</script>

<div class="crash" role="alert">
  <div class="text">
    <strong>{t("crash.title")}</strong>
    <p>{t(`crash.causes.${crash.cause}`)}</p>
    {#if crash.mods.length > 0}
      <p class="blamed">{t("crash.blamed", { mods: crash.mods.join(", ") })}</p>
    {/if}
    {#if crash.unfit.length > 0}
      <p>{t("crash.unfit", { mods: crash.unfit.join(", ") })}</p>
    {/if}
    {#if crash.details.length > 0}
      <pre>{crash.details.join("\n")}</pre>
    {/if}
  </div>
  <div class="actions">
    <button type="button" onclick={onlogs}>{t("crash.showLogs")}</button>
    <button type="button" class="ghost" onclick={ondismiss}>{t("account.dismiss")}</button>
  </div>
</div>

<style>
  .crash {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    padding: 14px 16px;
    border: 1px solid color-mix(in srgb, var(--danger) 45%, transparent);
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--danger) 14%, var(--surface));
  }

  .text {
    display: grid;
    flex: 1;
    gap: 4px;
    min-width: 0;
  }

  p {
    margin: 0;
    font-size: 13px;
  }

  .blamed {
    font-weight: 600;
  }

  pre {
    margin: 4px 0 0;
    padding: 8px 10px;
    overflow-x: auto;
    border-radius: var(--radius-sm);
    background: rgb(0 0 0 / 0.25);
    font-family: "Cascadia Mono", Consolas, monospace;
    font-size: 11px;
    white-space: pre-wrap;
    user-select: text;
  }

  .actions {
    display: flex;
    flex: none;
    gap: 6px;
  }

  button {
    height: 30px;
    padding: 0 12px;
    border: none;
    border-radius: 999px;
    background: var(--danger);
    color: #ffffff;
    font-size: 12px;
    font-weight: 600;
  }

  .ghost {
    background: var(--field);
    color: var(--text);
  }
</style>
