<script lang="ts">
  import { t } from "$lib/i18n/index.svelte";
  import { scrollFade } from "$lib/scroll-fade";

  let text = $state<string | null>(null);
  let failed = $state(false);

  $effect(() => {
    fetch("/third-party-licenses.txt")
      .then((response) => (response.ok ? response.text() : Promise.reject()))
      .then((body) => {
        text = body;
      })
      .catch(() => {
        failed = true;
      });
  });
</script>

<section class="page">
  <header>
    <a class="back-link" href="/settings">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
      {t("settings.title")}
    </a>
    <h1 class="page-title">{t("licenses.title")}</h1>
  </header>

  <div class="content panel" {@attach scrollFade}>
    {#if text}
      <pre>{text}</pre>
    {:else if failed}
      <p>{t("licenses.missing")}</p>
    {/if}
  </div>
</section>

<style>
  header {
    display: grid;
    gap: 4px;
    padding-top: 8px;
  }

  .content {
    min-height: 0;
    margin-bottom: 16px;
    padding: 14px 18px;
    overflow: auto;
  }

  pre {
    margin: 0;
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 11px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }

  p {
    margin: 0;
    color: var(--text-muted);
  }
</style>
