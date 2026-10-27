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
    padding: 3px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
  }

  label {
    display: grid;
    place-items: center;
    min-height: 30px;
    padding: 0 14px;
    border-radius: 999px;
    font-size: 13px;
    cursor: pointer;
    transition: background 150ms var(--ease);
  }

  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  label:has(input:checked) {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }

  label:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
