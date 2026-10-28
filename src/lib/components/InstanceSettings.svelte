<script lang="ts">
  import { api, type Instance, type MemoryInfo } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { locale, t } from "$lib/i18n/index.svelte";
  import { library } from "$lib/state/instances.svelte";

  let { instance }: { instance: Instance } = $props();

  const STEP_MB = 512;
  const MIN_MB = 1024;

  let info = $state<MemoryInfo | null>(null);
  let error = $state<string | null>(null);
  let manual = $derived(instance.maxMemoryMb);

  $effect(() => {
    api
      .memoryInfo(instance.id)
      .then((value) => {
        info = value;
      })
      .catch((err) => {
        error = describeError(err);
      });
  });

  function gigabytes(megabytes: number): string {
    const count = new Intl.NumberFormat(locale(), { maximumFractionDigits: 1 }).format(
      megabytes / 1024,
    );
    return t("instance.gigabytes", { count });
  }

  async function save(maxMemoryMb: number | null) {
    try {
      const updated = await api.setInstanceMemory(instance.id, maxMemoryMb);
      library.instances = library.instances.map((entry) =>
        entry.id === updated.id ? updated : entry,
      );
    } catch (err) {
      error = describeError(err);
    }
  }
</script>

<section class="group">
  <h2>{t("instance.memory")}</h2>

  {#if info}
    {@const recommended = info.recommended}
    <label class="row">
      <span>{t("instance.memoryAuto")}</span>
      <span class="switch">
        <input
          type="checkbox"
          role="switch"
          checked={manual === null}
          aria-checked={manual === null}
          onchange={(event) => save(event.currentTarget.checked ? null : recommended)}
        >
        <span aria-hidden="true"></span>
      </span>
    </label>

    {#if manual === null}
      <p class="hint">
        {t("instance.memoryAutoHint", {
          amount: gigabytes(info.recommended),
          total: gigabytes(info.total),
        })}
      </p>
    {:else}
      <div class="slider">
        <input
          type="range"
          min={MIN_MB}
          max={info.max}
          step={STEP_MB}
          value={manual}
          aria-label={t("instance.memory")}
          oninput={(event) => (manual = Number(event.currentTarget.value))}
          onchange={(event) => save(Number(event.currentTarget.value))}
        >
        <output>{gigabytes(manual)}</output>
      </div>
      <p class="hint">{t("instance.memoryManualHint")}</p>
    {/if}
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</section>

<style>
  .group {
    display: grid;
    gap: 10px;
    max-width: 560px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: var(--display-weight);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  .slider {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .slider input {
    flex: 1;
    accent-color: var(--accent);
  }

  output {
    min-width: 56px;
    font-weight: 600;
    text-align: right;
  }

  .switch {
    position: relative;
    flex: none;
    width: 38px;
    height: 22px;
  }

  .switch input {
    position: absolute;
    inset: 0;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }

  .switch span {
    position: absolute;
    inset: 0;
    border-radius: 999px;
    background: var(--field);
    box-shadow: inset 0 0 0 1px var(--border);
    transition: background 150ms var(--ease);
    pointer-events: none;
  }

  .switch span::after {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #ffffff;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.25);
    transition: transform 150ms var(--ease);
    content: "";
  }

  .switch input:checked + span {
    background: var(--accent);
  }

  .switch input:checked + span::after {
    transform: translateX(16px);
  }

  .switch input:focus-visible + span {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .error {
    margin: 0;
    color: var(--danger);
  }
</style>
