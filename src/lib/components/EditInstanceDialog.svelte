<script lang="ts">
  import { api, type Instance } from "$lib/api";
  import { COVERS, coverOf, DEFAULT_ICON, ICONS, iconOf } from "$lib/appearance";
  import { describeError } from "$lib/errors";
  import { t } from "$lib/i18n/index.svelte";
  import { library, replaceInstance } from "$lib/state/instances.svelte";
  import Dialog from "./Dialog.svelte";

  let { instance, open = $bindable() }: { instance: Instance; open: boolean } = $props();

  let name = $state("");
  let icon = $state<string | null>(null);
  let cover = $state<string | null>(null);
  let group = $state("");
  let saving = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    if (open) {
      name = instance.name;
      icon = instance.icon ?? DEFAULT_ICON;
      cover = instance.cover;
      group = instance.group ?? "";
      error = null;
    }
  });

  const groups = $derived(
    [
      ...new Set(library.instances.map((entry) => entry.group).filter((name) => name !== null)),
    ].sort(),
  );

  async function save() {
    saving = true;
    error = null;
    try {
      replaceInstance(await api.updateInstance(instance.id, name, icon, cover, group || null));
      open = false;
    } catch (err) {
      error = describeError(err);
    } finally {
      saving = false;
    }
  }
</script>

<Dialog bind:open title={t("edit.title")} width={460} onsubmit={save}>
  <div class="preview" style:background={coverOf(instance.id, cover)}>
    <span class="tile">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconOf(icon)} /></svg>
    </span>
    <span class="name">{name || instance.name}</span>
  </div>

  <label class="form-field">
    <span>{t("edit.name")}</span>
    <input bind:value={name} maxlength="64" required>
  </label>

  <label class="form-field">
    <span>{t("edit.group")}</span>
    <input
      bind:value={group}
      maxlength="32"
      list="instance-groups"
      placeholder={t("edit.groupPlaceholder")}
    >
    <datalist id="instance-groups">
      {#each groups as name (name)}
        <option value={name}></option>
      {/each}
    </datalist>
  </label>

  <fieldset>
    <legend>{t("edit.icon")}</legend>
    <div class="icons">
      {#each Object.entries(ICONS) as [key, path] (key)}
        <label class="choice">
          <input type="radio" name="icon" value={key} bind:group={icon} aria-label={key}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={path} /></svg>
        </label>
      {/each}
    </div>
  </fieldset>

  <fieldset>
    <legend>{t("edit.cover")}</legend>
    <div class="covers">
      {#each Object.entries(COVERS) as [key, gradient] (key)}
        <label class="choice swatch" style:background={gradient}>
          <input type="radio" name="cover" value={key} bind:group={cover} aria-label={key}>
        </label>
      {/each}
    </div>
  </fieldset>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#snippet actions()}
    <button type="button" onclick={() => (open = false)}>{t("create.cancel")}</button>
    <button type="submit" class="primary" disabled={saving}>{t("edit.save")}</button>
  {/snippet}
</Dialog>

<style>
  svg {
    width: 20px;
    height: 20px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .preview {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    height: 104px;
    padding: 14px;
    border-radius: var(--radius-md);
  }

  .tile {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border: 1px solid rgb(255 255 255 / 0.35);
    border-radius: 14px;
    background: rgb(255 255 255 / 0.2);
    backdrop-filter: blur(10px);
  }

  .tile svg {
    width: 26px;
    height: 26px;
    stroke: #ffffff;
  }

  .name {
    overflow: hidden;
    color: #ffffff;
    font-size: 16px;
    font-weight: 600;
    text-overflow: ellipsis;
    text-shadow: 0 1px 6px rgb(0 0 0 / 0.3);
    white-space: nowrap;
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: none;
  }

  legend {
    margin-bottom: 8px;
    padding: 0;
    color: var(--text-muted);
    font-size: 12px;
  }

  .icons,
  .covers {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 6px;
  }

  .icons {
    grid-template-columns: repeat(6, 1fr);
  }

  .choice {
    display: grid;
    place-items: center;
    height: 40px;
    border: 2px solid transparent;
    border-radius: var(--radius-sm);
    background: var(--field);
    cursor: pointer;
    transition: border-color 150ms var(--ease);
  }

  .swatch {
    height: 32px;
  }

  .choice:hover {
    border-color: var(--border);
  }

  .choice:has(input:checked) {
    border-color: var(--accent);
  }

  .choice:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .choice input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 13px;
  }
</style>
