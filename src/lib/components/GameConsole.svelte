<script lang="ts">
  import { tick } from "svelte";
  import { api } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";
  import { onGameLog } from "$lib/state/instances.svelte";

  let { instanceId }: { instanceId: string } = $props();

  const MAX_LINES = 5000;

  let lines = $state<string[]>([]);
  let filter = $state("");
  let copied = $state(false);
  let output = $state<HTMLDivElement>();
  let followTail = true;

  const visible = $derived(
    filter ? lines.filter((line) => line.toLowerCase().includes(filter.toLowerCase())) : lines,
  );

  $effect(() => {
    const id = instanceId;
    void api.gameLog(id).then(async (saved) => {
      lines = saved;
      await scrollToEnd();
    });
    return onGameLog(async (log) => {
      if (log.instanceId !== id) {
        return;
      }
      lines.push(...log.lines);
      if (lines.length > MAX_LINES) {
        lines.splice(0, lines.length - MAX_LINES);
      }
      if (followTail) {
        await scrollToEnd();
      }
    });
  });

  async function scrollToEnd() {
    await tick();
    output?.scrollTo({ top: output.scrollHeight });
  }

  function onscroll() {
    if (output) {
      followTail = output.scrollHeight - output.scrollTop - output.clientHeight < 24;
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(visible.join("\n"));
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  function level(line: string): string {
    if (/\b(ERROR|FATAL)\b|Exception|^\s+at /.test(line)) {
      return "error";
    }
    return /\bWARN\b/.test(line) ? "warn" : "";
  }
</script>

<div class="console">
  <div class="controls">
    <input type="search" data-search bind:value={filter} placeholder={t("logs.filter")}>
    <button type="button" onclick={copy} disabled={visible.length === 0}>
      {copied ? t("logs.copied") : t("logs.copy")}
    </button>
    <button type="button" onclick={() => (lines = [])} disabled={lines.length === 0}>
      {t("logs.clear")}
    </button>
  </div>

  <div class="output" bind:this={output} {onscroll}>
    {#if lines.length === 0}
      <p class="empty">{t("logs.empty")}</p>
    {:else}
      {#each visible as line, index (index)}
        <div class="line {level(line)}">{line}</div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .console {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
    min-height: 0;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  input {
    flex: 1;
    height: 34px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  button {
    height: 34px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    font-weight: 600;
  }

  button:disabled {
    opacity: 0.5;
  }

  .output {
    flex: 1;
    min-height: 0;
    padding: 10px 12px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: rgb(0 0 0 / 0.35);
    color: #e6e6ef;
    font-family: "Cascadia Mono", Consolas, ui-monospace, monospace;
    font-size: 12px;
    line-height: 1.5;
    user-select: text;
  }

  .line {
    white-space: pre-wrap;
    word-break: break-word;
  }

  .warn {
    color: #ffd27a;
  }

  .error {
    color: #ff8a8a;
  }

  .empty {
    margin: 0;
    padding: 24px 0;
    color: #a1a1ad;
    font-family: var(--font-sans);
    text-align: center;
  }
</style>
