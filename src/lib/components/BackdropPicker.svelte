<script lang="ts" generics="T extends string">
  import { t } from "$lib/i18n/index.svelte";
  import { settings } from "$lib/state/settings.svelte";

  let {
    value = $bindable(),
    options,
    name,
    label,
    cover = null,
  }: { value: T; options: T[]; name: string; label: string; cover?: string | null } = $props();

  /** "app" previews whatever the launcher itself uses. */
  function art(option: string): string {
    return option === "app" ? settings.backdrop : option;
  }
</script>

<fieldset class="backdrops">
  <legend class="sr-only">{label}</legend>
  {#each options as option (option)}
    <label class="backdrop">
      <input type="radio" {name} value={option} bind:group={value}>
      <span
        class="preview {art(option)}"
        style:background-image={option === "cover" ? cover : undefined}
        aria-hidden="true"
      ></span>
      <span class="name">{t(`backdrops.${option as "none"}`)}</span>
    </label>
  {/each}
</fieldset>

<style>
  .backdrops {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    gap: 12px;
    margin: 0;
    padding: 0;
    border: none;
  }

  .backdrop {
    display: grid;
    gap: 8px;
    cursor: pointer;
  }

  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .preview {
    display: block;
    height: 64px;
    border: 2px solid var(--border);
    border-radius: var(--radius-md);
    background-color: #1b1b21;
    background-size: cover;
    transition: border-color 150ms var(--ease);
  }

  .preview.none {
    background-color: var(--background);
  }

  .preview.halo {
    background-image:
      radial-gradient(circle at 48% 52%, transparent 26%, #34e0d2 31%, transparent 38%),
      radial-gradient(circle at 55% 48%, transparent 30%, #ff4fd8 35%, transparent 42%),
      radial-gradient(circle at 52% 50%, transparent 33%, var(--accent) 38%, transparent 45%);
  }

  .preview.rays {
    background-image: repeating-conic-gradient(
      from 190deg at 20% 0%,
      transparent 0deg,
      color-mix(in srgb, var(--accent) 55%, transparent) 4deg,
      transparent 9deg,
      transparent 16deg
    );
  }

  .preview.stars {
    background-image:
      radial-gradient(circle at 20% 30%, #fff 1px, transparent 1.5px),
      radial-gradient(circle at 70% 20%, #fff 1px, transparent 1.5px),
      radial-gradient(circle at 45% 70%, #fff 1.2px, transparent 1.8px),
      radial-gradient(circle at 85% 65%, #fff 0.8px, transparent 1.3px),
      radial-gradient(circle at 30% 85%, #fff 0.8px, transparent 1.3px),
      radial-gradient(circle at 60% 40%, #fff 0.7px, transparent 1.2px),
      radial-gradient(
        circle at 85% 10%,
        color-mix(in srgb, var(--accent) 45%, transparent),
        transparent 55%
      );
  }

  .preview.aurora {
    background-image:
      radial-gradient(ellipse at 20% 15%, rgb(47 224 162 / 0.7), transparent 55%),
      radial-gradient(
        ellipse at 85% 30%,
        color-mix(in srgb, var(--accent) 75%, transparent),
        transparent 55%
      ),
      radial-gradient(ellipse at 50% 100%, rgb(63 140 255 / 0.7), transparent 60%);
  }

  .backdrop:has(input:checked) .preview {
    border-color: var(--accent);
  }

  .backdrop:has(input:focus-visible) .preview {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .name {
    font-size: 13px;
    text-align: center;
  }

  .backdrop:has(input:checked) .name {
    font-weight: 600;
  }
</style>
