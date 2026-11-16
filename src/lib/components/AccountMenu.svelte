<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import MoonPhase from "#lib/components/MoonPhase.svelte";
  import PlayerHead from "#lib/components/PlayerHead.svelte";
  import PlayerNameDialog from "#lib/components/PlayerNameDialog.svelte";
  import SignInProblem from "#lib/components/SignInProblem.svelte";
  import { t } from "#lib/i18n/index.svelte.ts";
  import {
    activeAccount,
    cancelSignIn,
    loadAccounts,
    session,
    signIn,
    signOut,
    switchAccount,
  } from "#lib/state/accounts.svelte.ts";

  let open = $state(false);
  let root: HTMLDivElement;

  const active = $derived(activeAccount());
  const expired = $derived(Boolean(active && session.expired.includes(active.id)));

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
    // With nothing else to pick, the button does what it says.
    const lone = session.accounts.length === 0 || (expired && session.accounts.length === 1);
    if (lone && !session.signingIn && !open) {
      void signIn();
    } else {
      open = !open;
    }
  }

  function onkeydown(event: KeyboardEvent) {
    if (open && event.key === "Escape") {
      open = false;
    }
  }

  async function choose(id: string) {
    await switchAccount(id);
    open = false;
  }

  function act(action: () => unknown) {
    open = false;
    void action();
  }
</script>

<svelte:window {onkeydown} />

{#snippet icon(path: string)}
  <svg viewBox="0 0 24 24" aria-hidden="true"><path d={path} /></svg>
{/snippet}

<div class="account" bind:this={root}>
  <button
    type="button"
    class="pill"
    class:expired={expired && !session.signingIn}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={toggle}
  >
    {#if session.signingIn}
      <span class="moon"><MoonPhase percent={50} /></span>
      <span>{t("account.waitingShort")}</span>
    {:else if active}
      <PlayerHead skin={active.skin} name={active.name} size={24} />
      <span>{expired ? t("account.signInAgain") : active.name}</span>
    {:else}
      <span class="plus">{@render icon("M12 6v12M6 12h12")}</span>
      <span>{t("account.signIn")}</span>
    {/if}
  </button>

  {#if open}
    <div class="menu" role="menu" aria-label={t("account.menu")}>
      {#each session.accounts as account (account.id)}
        {@const current = account.id === session.active}
        {@const lapsed = session.expired.includes(account.id)}
        <div class="who">
          <button
            type="button"
            role="menuitemradio"
            aria-checked={current}
            class="pick"
            onclick={() => choose(account.id)}
          >
            <PlayerHead skin={account.skin} name={account.name} size={36} />
            <span class="names">
              <span class="name">{account.name}</span>
              <span class="sub" class:bad={lapsed}>
                {lapsed
                  ? t("account.expired")
                  : current
                    ? t("account.current")
                    : t("account.switchTo")}
              </span>
            </span>
            {#if current && !lapsed}
              <span class="check">{@render icon("M6 12.5l4 4 8-9")}</span>
            {/if}
          </button>
          {#if lapsed && !session.signingIn}
            <button type="button" class="pill-button again" onclick={() => act(signIn)}>
              {t("account.signInAgain")}
            </button>
          {/if}
        </div>
      {/each}

      {#if session.signingIn}
        <div class="waiting">
          <span class="moon"><MoonPhase percent={50} /></span>
          <span>{t("account.waiting")}</span>
          <button type="button" class="cancel" onclick={cancelSignIn}>{t("account.cancel")}</button>
        </div>
      {/if}

      <hr>
      <button
        type="button"
        role="menuitem"
        class="item"
        disabled={session.signingIn}
        onclick={() => act(signIn)}
      >
        {@render icon("M12 5v14M5 12h14")}
        {t("account.add")}
      </button>
      {#if active}
        <button
          type="button"
          role="menuitem"
          class="item"
          onclick={() => act(() => (session.naming = "change"))}
        >
          {@render icon("M15.5 4.5l4 4L9 19H5v-4z")}
          {t("playerName.changeTitle")}
        </button>
        <a class="item" role="menuitem" href="/skins" onclick={() => (open = false)}>
          {@render icon("M9 3.5 4 6l2 4 2-1v11h8V9l2 1 2-4-5-2.5a3 3 0 0 1-6 0z")}
          {t("account.skin")}
        </a>
      {/if}
      <button
        type="button"
        role="menuitem"
        class="item"
        onclick={() => act(() => openUrl("https://account.microsoft.com/"))}
      >
        {@render icon("M14 5h5v5M19 5l-8 8M17 13.5V19H5V7h5.5")}
        {t("account.microsoft")}
      </button>
      {#if active}
        <hr>
        <button
          type="button"
          role="menuitem"
          class="item danger"
          onclick={() => act(() => signOut(active.id))}
        >
          {@render icon("M14 4.5h4.5v15H14M10 8l-4 4 4 4M6 12h9")}
          {t("account.signOutOf", { name: active.name })}
        </button>
      {/if}
    </div>
  {/if}

  <PlayerNameDialog />

  {#if session.error && !open}
    <div class="problem">
      <SignInProblem error={session.error} ondismiss={() => (session.error = null)} />
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

  svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .pill {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 14px 0 4px;
    border: none;
    border-radius: 999px;
    background: var(--surface);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      inset 0 0 0 1px var(--border);
    color: var(--text);
    font-size: 13px;
    font-weight: 800;
    transition:
      background 150ms var(--ease),
      box-shadow 150ms var(--ease);
  }

  .pill:hover,
  .pill[aria-expanded="true"] {
    background: var(--surface-hover);
  }

  .pill :global(.head) {
    border-radius: 8px;
  }

  .pill.expired {
    box-shadow:
      inset 0 0 0 1.5px var(--danger-text),
      0 0 14px -4px color-mix(in srgb, var(--danger-text) 60%, transparent);
  }

  .plus {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-text);
  }

  .plus svg {
    width: 14px;
    height: 14px;
    stroke-width: 2.4;
  }

  .moon {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    color: var(--gold);
    animation: pulse 1.6s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.45;
    }
  }

  .menu,
  .problem {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 31;
    width: 340px;
    border-radius: var(--radius-lg);
    background: var(--surface-strong);
    box-shadow:
      inset 0 1px 0 var(--edge-top),
      0 0 0 1px var(--border),
      0 30px 60px -20px rgb(0 0 0 / 0.6);
    backdrop-filter: var(--glass);
    animation: drop 320ms var(--spring);
  }

  .menu {
    display: grid;
    gap: 2px;
    padding: 8px;
  }

  .problem {
    width: 380px;
    padding: 16px 14px 16px 16px;
  }

  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.98);
    }
  }

  .who {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .pick {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 8px;
    border: none;
    border-radius: 14px;
    background: transparent;
    color: var(--text);
    text-align: left;
  }

  .pick:hover {
    background: var(--field);
  }

  .pick :global(.head) {
    border-radius: 10px;
  }

  .names {
    display: grid;
    flex: 1;
    gap: 1px;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    font-size: 15px;
    font-weight: 800;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub {
    color: var(--text-muted);
    font-size: 12.5px;
    font-weight: 650;
  }

  .sub.bad {
    color: var(--danger-text);
  }

  .check {
    display: flex;
    color: var(--accent-fg);
  }

  .check svg {
    stroke-width: 2.2;
  }

  .again {
    margin-right: 6px;
  }

  .waiting {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    color: var(--text-2);
    font-size: 13px;
    font-weight: 700;
  }

  .cancel {
    margin-left: auto;
    padding: 4px 8px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--accent-fg);
    font-size: 13px;
    font-weight: 800;
  }

  hr {
    width: calc(100% - 16px);
    height: 1px;
    margin: 6px auto;
    border: none;
    background: var(--border);
  }

  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 40px;
    padding: 0 12px;
    border: none;
    border-radius: 12px;
    background: transparent;
    color: var(--text);
    font-size: 14px;
    font-weight: 750;
    text-align: left;
    text-decoration: none;
  }

  .item svg {
    color: var(--text-2);
  }

  .item:hover:enabled,
  a.item:hover {
    background: var(--field);
  }

  .item:disabled {
    opacity: 0.5;
  }

  .item.danger,
  .item.danger svg {
    color: var(--danger-text);
  }
</style>
