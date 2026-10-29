<script lang="ts">
  import { t } from "$lib/i18n/index.svelte";
  import {
    BEAUTIFUL,
    currentLook,
    FAST,
    type Look,
    matchesLook,
    settings,
  } from "$lib/state/settings.svelte";
  import { tooltip } from "$lib/tooltip";

  const NAME_LENGTH = 24;

  let naming = $state(false);
  let name = $state("");
  let input = $state<HTMLInputElement>();

  $effect(() => {
    if (naming) {
      input?.focus();
    }
  });

  function apply(look: Partial<Look>) {
    Object.assign(settings, look);
  }

  function save(event: SubmitEvent) {
    event.preventDefault();
    const trimmed = name.trim();
    if (!trimmed) {
      return;
    }
    settings.presets = [
      ...settings.presets.filter((preset) => preset.name !== trimmed),
      { name: trimmed, look: currentLook() },
    ];
    naming = false;
    name = "";
  }

  function cancelOnEscape(event: KeyboardEvent) {
    if (event.key === "Escape") {
      naming = false;
    }
  }

  function remove(index: number) {
    settings.presets = settings.presets.filter((_, at) => at !== index);
  }
</script>

<div class="presets">
  <button type="button" class="preset" class:active={matchesLook(FAST)} onclick={() => apply(FAST)}>
    {t("welcome.fast")}
  </button>
  <button
    type="button"
    class="preset beautiful"
    class:active={matchesLook(BEAUTIFUL)}
    onclick={() => apply(BEAUTIFUL)}
  >
    {t("welcome.beautiful")}
  </button>
  {#each settings.presets as preset, index (preset.name)}
    <span class="preset own" class:active={matchesLook(preset.look)}>
      <button type="button" class="apply" onclick={() => apply(preset.look)}>{preset.name}</button>
      <button
        type="button"
        class="remove"
        aria-label={t("settings.deletePreset", { name: preset.name })}
        {@attach tooltip(t("settings.deletePreset", { name: preset.name }))}
        onclick={() => remove(index)}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
      </button>
    </span>
  {/each}
  {#if naming}
    <form class="naming" onsubmit={save}>
      <input
        bind:this={input}
        bind:value={name}
        maxlength={NAME_LENGTH}
        placeholder={t("settings.presetName")}
        aria-label={t("settings.presetName")}
        spellcheck="false"
        onkeydown={cancelOnEscape}
      >
      <button type="submit" class="preset active" disabled={!name.trim()}>
        {t("settings.savePresetConfirm")}
      </button>
    </form>
  {:else}
    <button type="button" class="preset add" onclick={() => (naming = true)}>
      + {t("settings.savePreset")}
    </button>
  {/if}
</div>

<style>
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .preset {
    display: inline-flex;
    align-items: center;
    height: 32px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--field);
    font-size: 13px;
    font-weight: 600;
    transition:
      background 150ms var(--ease),
      border-color 150ms var(--ease);
  }

  button.preset:hover:enabled,
  .own:hover {
    border-color: var(--accent);
  }

  .preset.active {
    border-color: transparent;
    background: var(--accent);
    color: var(--accent-text);
  }

  .beautiful:not(.active) {
    background:
      linear-gradient(var(--surface-solid), var(--surface-solid)) padding-box,
      linear-gradient(120deg, #34e0d2, var(--accent), #ff4fd8) border-box;
    border-color: transparent;
  }

  .add {
    border-style: dashed;
    background: transparent;
    color: var(--text-muted);
  }

  .own {
    gap: 2px;
    padding: 0 4px 0 14px;
  }

  .own button {
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    color: inherit;
  }

  .remove {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    opacity: 0.6;
  }

  .remove:hover {
    background: rgb(0 0 0 / 0.15);
    opacity: 1;
  }

  .remove svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }

  .naming {
    display: flex;
    gap: 6px;
  }

  .naming input {
    width: 170px;
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--field);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }
</style>
