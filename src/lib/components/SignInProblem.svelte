<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { CommandError } from "#lib/api.ts";
  import { t } from "#lib/i18n/index.svelte.ts";
  import { signIn } from "#lib/state/accounts.svelte.ts";

  let {
    error,
    actions = true,
    ondismiss,
  }: {
    error: CommandError;
    /** Off where a sign-in button is already next to the message. */
    actions?: boolean;
    ondismiss?: () => void;
  } = $props();

  type Known =
    | "network"
    | "signInCancelled"
    | "gameNotOwned"
    | "noXboxAccount"
    | "childAccount"
    | "sessionExpired";

  interface Look {
    tone: "red" | "grey" | "accent" | "green" | "orange" | "pink";
    icon: string;
    action?: { label: "retry" | "signInAgain" | "otherAccount"; primary?: boolean };
    link?: { label: string; url: string };
  }

  const LOOKS: Record<Known, Look> = {
    network: {
      tone: "red",
      icon: "M2.5 8.5a14 14 0 0 1 19 0M5.8 12a9.5 9.5 0 0 1 12.4 0M9 15.5a5 5 0 0 1 6 0M12 19h.01M3.5 3.5l17 17",
      action: { label: "retry", primary: true },
    },
    signInCancelled: {
      tone: "grey",
      icon: "M7 7l10 10M17 7 7 17",
      action: { label: "signInAgain" },
    },
    gameNotOwned: {
      tone: "accent",
      icon: "M12 3l8 4.5v9L12 21l-8-4.5v-9zM4 7.5l8 4.5 8-4.5M12 12v9",
      action: { label: "otherAccount" },
      link: { label: "minecraft.net", url: "https://www.minecraft.net/" },
    },
    noXboxAccount: {
      tone: "green",
      icon: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM8.5 8.5l7 7M15.5 8.5l-7 7",
    },
    childAccount: {
      tone: "orange",
      icon: "M10 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zM3.5 20a6.5 6.5 0 0 1 13 0M19 8v6M16 11h6",
    },
    sessionExpired: {
      tone: "pink",
      icon: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 7.5V12l3 2",
      action: { label: "signInAgain" },
    },
  };
  const OTHER: Look = {
    tone: "red",
    icon: "M12 3.5l9 16H3zM12 10v4M12 17h.01",
    action: { label: "retry", primary: true },
  };

  const known = $derived(error.kind in LOOKS ? (error.kind as Known) : null);
  const look = $derived(known ? LOOKS[known] : OTHER);
  const title = $derived(known ? t(`account.problems.${known}.title`) : t("account.problemTitle"));
  const body = $derived(known ? t(`account.problems.${known}.body`) : t(`errors.${error.kind}`));
  const link = $derived(
    known === "noXboxAccount"
      ? { label: t("account.openXbox"), url: "https://www.xbox.com/" }
      : known === "childAccount"
        ? { label: t("account.howTo"), url: "https://account.microsoft.com/family/" }
        : look.link,
  );
</script>

<div class="problem" role="alert">
  <span class="tile {look.tone}" aria-hidden="true">
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d={look.icon} /></svg>
  </span>
  <div class="text">
    <strong>{title}</strong>
    <p>{body}</p>
    {#if (actions && look.action) || link}
      <div class="actions">
        {#if actions && look.action}
          <button
            type="button"
            class="pill-button"
            class:primary={look.action.primary}
            onclick={() => signIn()}
          >
            {t(`account.${look.action.label}`)}
          </button>
        {/if}
        {#if link}
          <button type="button" class="link" onclick={() => openUrl(link.url)}>
            {link.label}
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M14 5h5v5M19 5l-8 8M17 13.5V19H5V7h5.5" />
            </svg>
          </button>
        {/if}
      </div>
    {/if}
  </div>
  {#if ondismiss}
    <button
      type="button"
      class="icon-button close"
      aria-label={t("account.dismiss")}
      onclick={ondismiss}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7l10 10M17 7 7 17" /></svg>
    </button>
  {/if}
</div>

<style>
  .problem {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 14px;
    text-align: left;
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .tile {
    display: grid;
    flex: none;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 12px;
    color: #ffffff;
  }

  .tile svg {
    width: 19px;
    height: 19px;
    stroke-width: 2;
  }

  .red {
    background: var(--danger);
  }

  .grey {
    background: var(--field);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-2);
  }

  .accent {
    background: var(--accent);
    color: var(--accent-text);
  }

  .green {
    background: #107c10;
  }

  .orange {
    background: #c26d14;
  }

  .pink {
    background: #d94c79;
  }

  .text {
    display: grid;
    flex: 1;
    gap: 4px;
    min-width: 0;
  }

  strong {
    font-size: 15px;
    font-weight: 800;
  }

  p {
    margin: 0;
    color: var(--text-2);
    font-size: 13.5px;
    font-weight: 600;
    line-height: 1.45;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    margin-top: 8px;
  }

  .link {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 34px;
    padding: 0;
    border: none;
    background: none;
    color: var(--text);
    font-size: 13.5px;
    font-weight: 800;
  }

  .link:hover {
    color: var(--accent-fg);
  }

  .link svg {
    width: 14px;
    height: 14px;
    stroke-width: 2;
  }

  .close {
    position: absolute;
    top: -6px;
    right: -6px;
  }

  .problem:has(.close) strong {
    padding-right: 30px;
  }
</style>
