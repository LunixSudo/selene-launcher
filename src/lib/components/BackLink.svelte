<script lang="ts">
  import { goBack } from "#lib/history.ts";
  import { t } from "#lib/i18n/index.svelte.ts";

  function back(event: MouseEvent) {
    if (goBack()) {
      event.preventDefault();
    }
  }
</script>

<a class="back-link" href="/" onclick={back}>
  <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
  {t("window.back")}
</a>
