<script lang="ts">
  import { t } from "$lib/i18n/index.svelte";
  import {
    activeAccount,
    cancelSignIn,
    loadAccounts,
    session,
    signIn,
    signOut,
    switchAccount,
  } from "$lib/state/accounts.svelte";

  let open = $state(false);
  let root: HTMLDivElement;

  const active = $derived(activeAccount());

  $effect(() => {
    void loadAccounts();
  });

  $effect(() => {
    if (!open) {
      return;
    }
    const close = (event: PointerEvent) => {
      if (!root.contains(event.target as Node)) {
        open = false;
      }
    };
    window.addEventListener("pointerdown", close);
    return () => window.removeEventListener("pointerdown", close);
  });

  function toggle() {
    if (session.accounts.length === 0 && !session.signingIn) {
      void signIn();
    } else {
      open = !open;
    }
  }
</script>

<div class="account" bind:this={root}>
  <button type="button" class="pill" class:signing-in={session.signingIn} onclick={toggle}>
    {#if active}
      <span class="avatar" aria-hidden="true">{active.name.slice(0, 1)}</span>
      <span>{active.name}</span>
    {:else if session.signingIn}
      <span class="spinner" aria-hidden="true"></span>
      <span>{t("account.waiting")}</span>
    {:else}
      <span>{t("account.signIn")}</span>
    {/if}
  </button>

  {#if open}
    <div class="menu" role="menu">
      {#each session.accounts as account (account.id)}
        <div class="row">
          <button
            type="button"
            role="menuitemradio"
            aria-checked={account.id === session.active}
            class="item"
            onclick={() => switchAccount(account.id)}
          >
            <span class="avatar" aria-hidden="true">{account.name.slice(0, 1)}</span>
            <span class="name">{account.name}</span>
            {#if account.id === session.active}
              <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.5l3 3 6-7" /></svg>
            {/if}
          </button>
          <button type="button" class="sign-out" onclick={() => signOut(account.id)}>
            {t("account.signOut")}
          </button>
        </div>
      {/each}

      {#if session.signingIn}
        <div class="waiting">
          <span class="spinner" aria-hidden="true"></span>
          <span>{t("account.waiting")}</span>
          <button type="button" class="link" onclick={cancelSignIn}>{t("account.cancel")}</button>
        </div>
      {:else}
        <button type="button" role="menuitem" class="item add" onclick={signIn}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 3v10M3 8h10" /></svg>
          {t("account.add")}
        </button>
      {/if}
    </div>
  {/if}

  {#if session.error && !open}
    <div class="error" role="alert">
      <p>{t(`errors.${session.error.kind}`)}</p>
      <button
        type="button"
        aria-label={t("account.dismiss")}
        onclick={() => (session.error = null)}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" /></svg>
      </button>
    </div>
  {/if}
</div>

<style>
  .account {
    position: relative;
    display: flex;
    align-items: center;
    margin-right: 8px;
  }

  .pill {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding: 0 12px 0 4px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    font-size: 12px;
    font-weight: 600;
    transition: background 150ms var(--ease);
  }

  .pill:hover {
    background: var(--surface-hover);
  }

  .pill:not(:has(.avatar)) {
    padding-left: 12px;
  }

  .pill.signing-in {
    padding-left: 8px;
  }

  .avatar {
    display: grid;
    flex: none;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 6px;
    background: var(--accent);
    color: var(--accent-text);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 10;
    display: grid;
    gap: 2px;
    min-width: 240px;
    padding: 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-solid);
    box-shadow: 0 16px 40px rgb(0 0 0 / 0.18);
    animation: drop 160ms var(--ease);
  }

  .row {
    display: flex;
    align-items: center;
  }

  .item {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 8px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
  }

  .item:hover {
    background: var(--field);
  }

  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .item svg:only-of-type {
    color: var(--accent);
  }

  .add {
    color: var(--text-muted);
  }

  .sign-out,
  .link {
    padding: 4px 8px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
  }

  .sign-out:hover {
    color: var(--danger);
  }

  .link {
    margin-left: auto;
    color: var(--accent);
  }

  .waiting {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 8px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 700ms linear infinite;
  }

  .error {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 10;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    width: 300px;
    padding: 10px 8px 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-solid);
    box-shadow: 0 16px 40px rgb(0 0 0 / 0.18);
    animation: drop 160ms var(--ease);
  }

  .error p {
    flex: 1;
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
  }

  .error p::before {
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-right: 6px;
    border-radius: 50%;
    background: var(--danger);
    content: "";
  }

  .error button {
    display: grid;
    flex: none;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
  }

  .error button:hover {
    background: var(--field);
  }

  .error svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
</style>
