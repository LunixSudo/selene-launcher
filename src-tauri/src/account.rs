use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use keyring::Entry;
use reqwest::Client;
use selene_core::auth::{self, LoginRequest, MicrosoftTokens, MinecraftSession};
use selene_core::launch::LaunchAccount;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::error::{CommandError, CommandResult};

/// Name the refresh tokens are filed under in the OS credential store.
const KEYRING_SERVICE: &str = "io.github.lunixsudo.selene";
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// Sessions this close to expiring are renewed before launch so the game never gets a stale one.
const EXPIRY_MARGIN: Duration = Duration::from_secs(5 * 60);

/// What the launcher remembers about an account on disk. Tokens never go here: the
/// Microsoft refresh token lives in the credential store and game sessions only in memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountList {
    pub accounts: Vec<Account>,
    pub active: Option<String>,
}

pub struct Accounts {
    path: PathBuf,
    list: Mutex<AccountList>,
    sessions: tokio::sync::Mutex<HashMap<String, MinecraftSession>>,
    cancel_sign_in: Mutex<Option<oneshot::Sender<()>>>,
}

impl Accounts {
    pub fn load(path: PathBuf) -> Self {
        let list = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            path,
            list: Mutex::new(list),
            sessions: tokio::sync::Mutex::new(HashMap::new()),
            cancel_sign_in: Mutex::new(None),
        }
    }

    pub fn list(&self) -> AccountList {
        lock(&self.list).clone()
    }

    /// Runs the browser sign-in. `open` shows the Microsoft page to the user.
    pub async fn sign_in(
        &self,
        client: &Client,
        open: impl FnOnce(&str) -> CommandResult<()>,
    ) -> CommandResult<Account> {
        let request = LoginRequest::start().await?;
        let (cancel, cancelled) = oneshot::channel();
        if let Some(previous) = lock(&self.cancel_sign_in).replace(cancel) {
            let _ = previous.send(());
        }
        open(request.url.as_str())?;

        let tokens = tokio::select! {
            tokens = tokio::time::timeout(SIGN_IN_TIMEOUT, request.finish(client)) => {
                tokens.map_err(|_| CommandError::SignInCancelled)??
            }
            _ = cancelled => return Err(CommandError::SignInCancelled),
        };
        let session = auth::minecraft_login(client, &tokens.access_token).await?;
        let account = Account {
            id: session.profile.id.clone(),
            name: session.profile.name.clone(),
        };

        store_refresh_token(&account.id, &tokens)?;
        self.sessions
            .lock()
            .await
            .insert(account.id.clone(), session);
        self.update(|list| {
            list.accounts.retain(|known| known.id != account.id);
            list.accounts.push(account.clone());
            list.active = Some(account.id.clone());
        })?;
        Ok(account)
    }

    pub fn cancel_sign_in(&self) {
        if let Some(cancel) = lock(&self.cancel_sign_in).take() {
            let _ = cancel.send(());
        }
    }

    pub async fn sign_out(&self, id: &str) -> CommandResult<()> {
        match credential(id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(err) => return Err(err.into()),
        }
        self.sessions.lock().await.remove(id);
        self.update(|list| {
            list.accounts.retain(|account| account.id != id);
            if list.active.as_deref() == Some(id) {
                list.active = list.accounts.first().map(|account| account.id.clone());
            }
        })
    }

    pub fn set_active(&self, id: &str) -> CommandResult<()> {
        self.update(|list| {
            if list.accounts.iter().any(|account| account.id == id) {
                list.active = Some(id.to_owned());
            }
        })
    }

    /// The active account with a fresh game session, renewed through Microsoft when needed.
    pub async fn launch_account(&self, client: &Client) -> CommandResult<LaunchAccount> {
        let Some(id) = self.list().active else {
            return dev_account().ok_or(CommandError::NotSignedIn);
        };

        let mut sessions = self.sessions.lock().await;
        let fresh = sessions
            .get(&id)
            .is_some_and(|session| session.expires_at > SystemTime::now() + EXPIRY_MARGIN);
        if !fresh {
            let refresh_token = String::from_utf8(credential(&id)?.get_secret()?)
                .map_err(|_| CommandError::NotSignedIn)?;
            let tokens = auth::refresh(client, &refresh_token).await?;
            // Microsoft rotates refresh tokens, so the new one replaces the old right away.
            store_refresh_token(&id, &tokens)?;
            let session = auth::minecraft_login(client, &tokens.access_token).await?;
            sessions.insert(id.clone(), session);
        }

        let session = &sessions[&id];
        Ok(LaunchAccount {
            name: session.profile.name.clone(),
            uuid: session.profile.id.clone(),
            access_token: session.access_token.clone(),
            xuid: None,
        })
    }

    fn update(&self, change: impl FnOnce(&mut AccountList)) -> CommandResult<()> {
        let mut list = lock(&self.list);
        change(&mut list);
        write_list(&self.path, &list)
    }
}

fn write_list(path: &Path, list: &AccountList) -> CommandResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(list).map_err(selene_core::Error::from)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

fn credential(id: &str) -> CommandResult<Entry> {
    Ok(Entry::new(KEYRING_SERVICE, id)?)
}

fn store_refresh_token(id: &str, tokens: &MicrosoftTokens) -> CommandResult<()> {
    // Stored as UTF-8 bytes: Windows caps a credential at 2560 bytes, and a UTF-16 password
    // would only leave room for half as many characters.
    credential(id)?.set_secret(tokens.refresh_token.as_bytes())?;
    Ok(())
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Debug builds can start singleplayer without signing in while the app waits for Mojang's
/// approval: set `SELENE_DEV_PLAYER` to a player name. Release builds never include this.
#[cfg(debug_assertions)]
fn dev_account() -> Option<LaunchAccount> {
    let name = std::env::var("SELENE_DEV_PLAYER").ok()?;
    Some(LaunchAccount {
        name,
        uuid: "00000000000000000000000000000000".into(),
        access_token: "0".into(),
        xuid: None,
    })
}

#[cfg(not(debug_assertions))]
fn dev_account() -> Option<LaunchAccount> {
    None
}
