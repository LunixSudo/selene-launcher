<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    open = $bindable(),
    title,
    width = 420,
    onsubmit,
    children,
    actions,
  }: {
    open: boolean;
    title: string;
    width?: number;
    onsubmit: () => void;
    children: Snippet;
    actions: Snippet;
  } = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    if (open) {
      dialog.showModal();
    } else {
      dialog.close();
    }
  });

  function submit(event: SubmitEvent) {
    event.preventDefault();
    onsubmit();
  }
</script>

<dialog bind:this={dialog} style:--width="{width}px" onclose={() => (open = false)}>
  <form onsubmit={submit}>
    <h2>{title}</h2>
    {@render children()}
    <div class="actions">
      {@render actions()}
    </div>
  </form>
</dialog>

<style>
  dialog {
    width: min(var(--width), calc(100vw - 32px));
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-solid);
    color: var(--text);
    box-shadow: 0 24px 64px rgb(0 0 0 / 0.24);
  }

  dialog[open] {
    animation: pop 220ms var(--ease);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.25);
    backdrop-filter: blur(6px);
  }

  form {
    display: grid;
    gap: 14px;
    padding: 24px;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 600;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }

  .actions :global(button) {
    height: 34px;
    padding: 0 16px;
    border: none;
    border-radius: var(--radius-sm);
    background: var(--field);
    font-weight: 600;
  }

  .actions :global(.primary) {
    background: var(--accent);
    color: var(--accent-text);
  }

  .actions :global(.danger) {
    background: var(--danger);
    color: #ffffff;
  }

  .actions :global(button:disabled) {
    opacity: 0.6;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
  }
</style>
