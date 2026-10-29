<script lang="ts">
  import { tick } from "svelte";
  import { api, type LogLine } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";
  import { onGameLog } from "$lib/state/instances.svelte";

  let { instanceId }: { instanceId: string } = $props();

  const MAX_LINES = 5000;

  type Line = { id: number; text: string; level: string };

  let nextId = 0;
  let lines = $state.raw<Line[]>([]);
  let filter = $state("");
  let copied = $state(false);
  let output = $state<HTMLDivElement>();
  let followTail = true;

  const visible = $derived.by(() => {
    const needle = filter.toLowerCase();
    return needle ? lines.filter((line) => line.text.toLowerCase().includes(needle)) : lines;
  });

  $effect(() => {
    const id = instanceId;
    let stale = false;
    // Batches that arrive before the saved log; the saved log may already hold some of them.
    let early: LogLine[] | null = [];
    let savedEnd = 0;

    function append(entries: LogLine[]) {
      const fresh = entries.filter((entry) => entry.number >= savedEnd);
      if (fresh.length > 0) {
        lines = lines.concat(toLines(fresh.map((entry) => entry.text))).slice(-MAX_LINES);
      }
    }

    void api.gameLog(id).then(async (saved) => {
      if (stale) {
        return;
      }
      lines = toLines(saved.lines).slice(-MAX_LINES);
      savedEnd = saved.end;
      append(early ?? []);
      early = null;
      await scrollToEnd();
    });
    const unsubscribe = onGameLog(async (log) => {
      if (log.instanceId !== id) {
        return;
      }
      if (early) {
        early.push(...log.lines);
        return;
      }
      append(log.lines);
      if (followTail) {
        await scrollToEnd();
      }
    });
    return () => {
      stale = true;
      unsubscribe();
    };
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
    await navigator.clipboard.writeText(visible.map((line) => line.text).join("\n"));
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  // Ids and levels are worked out once per line, so trimming the head of the log does not
  // re-render or re-match the thousands of lines that stay.
  function toLines(texts: string[]): Line[] {
    return texts.map((text) => ({ id: nextId++, text, level: level(text) }));
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
      {#each visible as line (line.id)}
        <div class="line {line.level}">{line.text}</div>
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
    content-visibility: auto;
    contain-intrinsic-size: auto 18px;
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
