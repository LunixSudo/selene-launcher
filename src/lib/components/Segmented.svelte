<script lang="ts" generics="T extends string">
  let {
    value = $bindable(),
    options,
    name,
    label,
  }: { value: T; options: { value: T; label: string }[]; name: string; label: string } = $props();
</script>

<div class="segmented" role="radiogroup" aria-label={label}>
  {#each options as option (option.value)}
    <label>
      <input type="radio" {name} value={option.value} bind:group={value}>
      <span>{option.label}</span>
    </label>
  {/each}
</div>

<style>
  .segmented {
    display: flex;
    flex: none;
    gap: 2px;
    padding: 4px;
    border-radius: 999px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
    backdrop-filter: var(--glass);
  }

  label {
    display: grid;
    place-items: center;
    min-height: 32px;
    padding: 0 14px;
    border-radius: 999px;
    color: var(--text-2);
    font-size: 13.5px;
    font-weight: 700;
    cursor: pointer;
    transition:
      background 150ms var(--ease),
      color 150ms var(--ease);
  }

  label:hover {
    color: var(--text);
  }

  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  label:has(input:checked) {
    background: var(--accent);
    box-shadow: 0 6px 14px -8px color-mix(in srgb, var(--accent) 80%, transparent);
    color: var(--accent-text);
  }

  label:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
