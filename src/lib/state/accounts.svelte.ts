import { type Account, api, type CommandError, isCommandError } from "$lib/api";
import { prepareAll } from "./instances.svelte";

export const session = $state({
  accounts: [] as Account[],
  active: null as string | null,
  signingIn: false,
  error: null as CommandError | null,
  /** Asks for a player name: for a new account that has none, or to change the active one. */
  naming: null as "create" | "change" | null,
});

export function activeAccount(): Account | undefined {
  return session.accounts.find((account) => account.id === session.active);
}

export async function loadAccounts() {
  const list = await api.accounts();
  session.accounts = list.accounts;
  session.active = list.active;
}

export async function signIn() {
  session.signingIn = true;
  session.error = null;
  try {
    await api.signIn();
    await loadAccounts();
    // Instances added before signing in could not download their game yet.
    void prepareAll();
  } catch (error) {
    if (isCommandError(error) && error.kind === "needsPlayerName") {
      session.naming = "create";
    } else if (!isCommandError(error) || error.kind !== "signInCancelled") {
      session.error = isCommandError(error) ? error : { kind: "core", message: String(error) };
    }
  } finally {
    session.signingIn = false;
  }
}

export async function cancelSignIn() {
  await api.cancelSignIn();
}

export async function signOut(id: string) {
  await api.signOut(id);
  await loadAccounts();
}

export async function switchAccount(id: string) {
  await api.setActiveAccount(id);
  session.active = id;
}

/** Gives the player its name: finishes a sign-in that needed one, or renames the player. */
export async function savePlayerName(name: string) {
  if (session.naming === "create") {
    await api.createPlayer(name);
    await loadAccounts();
    void prepareAll();
  } else {
    await api.changeName(name);
    await loadAccounts();
  }
  session.naming = null;
}
