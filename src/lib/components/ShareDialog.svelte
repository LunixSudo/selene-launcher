<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { api, type Instance, type PackFormat } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { type MessageKey, t } from "$lib/i18n/index.svelte";
  import Segmented from "./Segmented.svelte";

  let { instance, open = $bindable() }: { instance: Instance; open: boolean } = $props();

  const FOLDERS: { name: string; label: MessageKey }[] = [
    { name: "config", label: "instance.shareConfig" },
    { name: "resourcepacks", label: "instance.shareResourcepacks" },
    { name: "shaderpacks", label: "instance.shareShaderpacks" },
  ];

  const FORMATS: Record<PackFormat, { name: string; extension: string }> = {
    mrpack: { name: "Modrinth", extension: "mrpack" },
    curseforge: { name: "CurseForge", extension: "zip" },
  };

  let dialog: HTMLDialogElement;
  let format = $state<PackFormat>("mrpack");
  let selected = $state(["config"]);
  let status = $state<"idle" | "saving" | "done">("idle");
  let error = $state<string | null>(null);

  $effect(() => {
    if (open) {
      status = "idle";
      error = null;
      dialog.showModal();
    } else {
      dialog.close();
    }
  });

  async function share(event: SubmitEvent) {
    event.preventDefault();
    const { name, extension } = FORMATS[format];
    const path = await save({
      defaultPath: `${instance.name}.${extension}`,
      filters: [{ name, extensions: [extension] }],
    });
    if (!path) {
      return;
    }
    status = "saving";
    error = null;
    try {
      await api.exportPack(instance.id, path, selected, format);
      status = "done";
    } catch (err) {
      status = "idle";
      error = describeError(err);
    }
  }
</script>

<dialog bind:this={dialog} onclose={() => (open = false)}>
  <form onsubmit={share}>
    <h2>{t("instance.shareTitle")}</h2>
    <Segmented
      bind:value={format}
      options={[
        { value: "mrpack", label: ".mrpack" },
        { value: "curseforge", label: ".zip" },
      ]}
      name="share-format"
      label={t("instance.shareFormat")}
    />
    <p class="hint">
      {format === "mrpack" ? t("instance.shareHint") : t("instance.shareZipHint")}
    </p>

    {#each FOLDERS as folder (folder.name)}
      <label class="check">
        <input type="checkbox" value={folder.name} bind:group={selected}>
        <span>{t(folder.label)}</span>
      </label>
    {/each}

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <div class="actions">
      <button type="button" class="secondary" onclick={() => (open = false)}>
        {t("create.cancel")}
      </button>
      <button type="submit" class="primary" disabled={status === "saving"}>
        {status === "done" ? t("instance.shareDone") : t("instance.shareSave")}
      </button>
    </div>
  </form>
</dialog>

<style>
  dialog {
    width: min(420px, calc(100vw - 32px));
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-solid);
    color: var(--text);
    box-shadow: 0 24px 64px rgb(0 0 0 / 0.24);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.25);
    backdrop-filter: blur(6px);
  }

  form {
    display: grid;
    gap: 12px;
    padding: 24px;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: var(--display-weight);
  }

  .hint {
    margin: 0 0 4px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .check input {
    accent-color: var(--accent);
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 13px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }

  .actions button {
    height: 34px;
    padding: 0 16px;
    border: none;
    border-radius: var(--radius-sm);
    font-weight: 600;
  }

  .primary {
    background: var(--accent);
    color: var(--accent-text);
  }

  .primary:disabled {
    opacity: 0.6;
  }

  .secondary {
    background: var(--field);
  }
</style>
