import { type Account, api, type CommandError, isCommandError } from "#lib/api.ts";
import { prepareAll } from "./instances.svelte";

export const session = $state({
  accounts: [] as Account[],
  active: null as string | null,
  signingIn: false,
  error: null as CommandError | null,
  /** Asks for a player name: for a new account that has none, or to change the active one. */
  naming: null as "create" | "change" | null,
  /** Accounts whose saved sign-in Microsoft no longer takes; they go through the browser again. */
  expired: [] as string[],
});

/** Set while the user stops a sign-in from the launcher, which needs no message. */
let cancelling = false;

export function activeAccount(): Account | undefined {
  return session.accounts.find((account) => account.id === session.active);
}

export async function loadAccounts() {
  const list = await api.accounts();
  session.accounts = list.accounts;
  session.active = list.active;
}

/** Signs in through the browser. Resolves to whether an account is ready to play. */
export async function signIn(): Promise<boolean> {
  session.signingIn = true;
  session.error = null;
  cancelling = false;
  try {
    await api.signIn();
    await loadAccounts();
    session.expired = session.expired.filter((id) => id !== session.active);
    // Instances added before signing in could not download their game yet.
    void prepareAll();
    return true;
  } catch (error) {
    if (isCommandError(error) && error.kind === "needsPlayerName") {
      session.naming = "create";
    } else if (!(cancelling && isCommandError(error) && error.kind === "signInCancelled")) {
      session.error = isCommandError(error) ? error : { kind: "core", message: String(error) };
    }
    return false;
  } finally {
    session.signingIn = false;
  }
}

/** Opens the sign-in when a start needs it, remembering an account whose sign-in ran out. */
export function signInToPlay(reason: CommandError): Promise<boolean> {
  if (
    reason.kind === "sessionExpired" &&
    session.active &&
    !session.expired.includes(session.active)
  ) {
    session.expired.push(session.active);
  }
  return signIn();
}

export async function cancelSignIn() {
  cancelling = true;
  await api.cancelSignIn();
}

export async function signOut(id: string) {
  await api.signOut(id);
  session.expired = session.expired.filter((entry) => entry !== id);
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
