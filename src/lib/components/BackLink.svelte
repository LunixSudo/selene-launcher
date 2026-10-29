<script lang="ts">
  import { afterNavigate } from "$app/navigation";
  import { t } from "$lib/i18n/index.svelte";

  /** Goes back to the page the user came from, or home when the page was opened directly. */
  let previous = $state("/");

  afterNavigate(({ from }) => {
    if (from) {
      previous = from.url.pathname + from.url.search;
    }
  });
</script>

<a class="back-link" href={previous}>
  <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
  {t("window.back")}
</a>
