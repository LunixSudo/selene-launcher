<script lang="ts">
  import { api, type NameChange, type NameStatus } from "$lib/api";
  import { describeError } from "$lib/errors";
  import { locale, t } from "$lib/i18n/index.svelte";
  import { activeAccount, savePlayerName, session } from "$lib/state/accounts.svelte";
  import Dialog from "./Dialog.svelte";

  const CHECK_DELAY_MS = 400;
  const NAME_PATTERN = /^[A-Za-z0-9_]{3,16}$/;

  let name = $state("");
  let status = $state<NameStatus | null>(null);
  let change = $state<NameChange | null>(null);
  let saving = $state(false);
  let error = $state<string | null>(null);

  const open = $derived(session.naming !== null);
  const creating = $derived(session.naming === "create");
  const valid = $derived(NAME_PATTERN.test(name));
  const blocked = $derived(!creating && change !== null && !change.allowed);

  $effect(() => {
    if (!open) {
      return;
    }
    name = creating ? "" : (activeAccount()?.name ?? "");
    status = null;
    error = null;
    change = null;
    if (!creating) {
      api
        .nameChange()
        .then((value) => {
          change = value;
        })
        .catch((err) => {
          error = describeError(err);
        });
    }
  });

  $effect(() => {
    const wanted = name;
    status = null;
    if (!open || !NAME_PATTERN.test(wanted) || wanted === activeAccount()?.name) {
      return;
    }
    const timer = setTimeout(() => {
      api
        .playerNameStatus(wanted)
        .then((value) => {
          if (name === wanted) {
            status = value;
          }
        })
        .catch(() => {});
    }, CHECK_DELAY_MS);
    return () => clearTimeout(timer);
  });

  function close(value: boolean) {
    if (!value) {
      session.naming = null;
    }
  }

  async function save() {
    saving = true;
    error = null;
    try {
      await savePlayerName(name);
    } catch (err) {
      error = describeError(err);
    } finally {
      saving = false;
    }
  }

  function nextChange(date: string): string {
    return new Intl.DateTimeFormat(locale(), { dateStyle: "long" }).format(new Date(date));
  }
</script>

<Dialog
  bind:open={() => open, close}
  title={t(creating ? "playerName.createTitle" : "playerName.changeTitle")}
  onsubmit={save}
>
  <p class="hint">{t(creating ? "playerName.createHint" : "playerName.changeHint")}</p>
  <label class="form-field">
    <span>{t("playerName.label")}</span>
    <input
      bind:value={name}
      maxlength="16"
      spellcheck="false"
      autocomplete="off"
      disabled={blocked}
    >
  </label>
  {#if blocked && change?.nextChange}
    <p class="hint">{t("playerName.cooldown", { date: nextChange(change.nextChange) })}</p>
  {:else if name && !valid}
    <p class="hint">{t("playerName.rules")}</p>
  {:else if status}
    <p class="status" class:ok={status === "available"}>{t(`playerName.status.${status}`)}</p>
  {/if}
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#snippet actions()}
    <button type="button" onclick={() => close(false)}>{t("create.cancel")}</button>
    <button
      type="submit"
      class="primary"
      disabled={saving || blocked || !valid || status === "taken" || status === "notAllowed"}
    >
      {t(creating ? "playerName.create" : "playerName.change")}
    </button>
  {/snippet}
</Dialog>

<style>
  .hint,
  .status {
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.5;
  }

  .status {
    color: var(--danger);
  }

  .status.ok {
    color: var(--accent);
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 13px;
  }
</style>
