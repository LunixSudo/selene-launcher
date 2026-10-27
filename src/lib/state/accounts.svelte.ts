import { type Account, api, type CommandError, isCommandError } from "$lib/api";

export const session = $state({
  accounts: [] as Account[],
  active: null as string | null,
  signingIn: false,
  error: null as CommandError | null,
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
  } catch (error) {
    if (!isCommandError(error) || error.kind !== "signInCancelled") {
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
