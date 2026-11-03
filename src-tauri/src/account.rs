use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use keyring::Entry;
use reqwest::Client;
use selene_core::auth::{
    self, AuthError, LoginRequest, MicrosoftTokens, MinecraftSession, MinecraftToken, Profile,
};
use selene_core::launch::LaunchAccount;
use selene_core::names::{self, NameChange, NameStatus};
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
    /// Texture URL of the current skin, for the player head in the title bar.
    #[serde(default)]
    pub skin: Option<String>,
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
    /// A sign-in that stopped because the account owns the game but has no player yet. It
    /// finishes once a name is picked; the tokens stay in memory only.
    unnamed: tokio::sync::Mutex<Option<(MicrosoftTokens, MinecraftToken)>>,
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
            unnamed: tokio::sync::Mutex::new(None),
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
        let token = auth::minecraft_token(client, &tokens.access_token).await?;
        let profile = match auth::fetch_profile(client, &token.access_token).await {
            Err(selene_core::Error::Auth(AuthError::GameNotOwned))
                if names::owns_game(client, &token.access_token).await? =>
            {
                *self.unnamed.lock().await = Some((tokens, token));
                return Err(CommandError::NeedsPlayerName);
            }
            result => result?,
        };
        self.finish_sign_in(&tokens, token, profile).await
    }

    pub async fn player_name_status(
        &self,
        client: &Client,
        name: &str,
    ) -> CommandResult<NameStatus> {
        let token = match self.unnamed.lock().await.as_ref() {
            Some((_, token)) => token.access_token.clone(),
            None => self.session(client).await?.access_token,
        };
        Ok(names::name_status(client, &token, name).await?)
    }

    /// Gives the player of a sign-in that needed one its name and finishes signing in.
    pub async fn create_player(&self, client: &Client, name: &str) -> CommandResult<Account> {
        let mut unnamed = self.unnamed.lock().await;
        let (_, token) = unnamed.as_ref().ok_or(CommandError::NotSignedIn)?;
        let profile = names::create_profile(client, &token.access_token, name).await?;
        let (tokens, token) = unnamed.take().ok_or(CommandError::NotSignedIn)?;
        drop(unnamed);
        self.finish_sign_in(&tokens, token, profile).await
    }

    pub async fn name_change(&self, client: &Client) -> CommandResult<NameChange> {
        let token = self.session(client).await?.access_token;
        Ok(names::name_change(client, &token).await?)
    }

    pub async fn change_name(&self, client: &Client, name: &str) -> CommandResult<Account> {
        let token = self.session(client).await?.access_token;
        let profile = names::change_name(client, &token, name).await?;
        self.set_profile(&profile).await?;
        Ok(Account {
            id: profile.id.clone(),
            name: profile.name.clone(),
            skin: active_skin(&profile),
        })
    }

    async fn finish_sign_in(
        &self,
        tokens: &MicrosoftTokens,
        token: MinecraftToken,
        profile: Profile,
    ) -> CommandResult<Account> {
        let session = MinecraftSession {
            access_token: token.access_token,
            expires_at: token.expires_at,
            profile,
        };
        let account = Account {
            id: session.profile.id.clone(),
            name: session.profile.name.clone(),
            skin: active_skin(&session.profile),
        };

        store_refresh_token(&account.id, tokens)?;
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
        if self.list().active.is_none() {
            return dev_account().ok_or(CommandError::NotSignedIn);
        }
        let session = match self.session(client).await {
            Ok(session) => session,
            // Like the official launcher, an account that has signed in before can still play
            // single player when Microsoft cannot be reached at all.
            Err(CommandError::Core(selene_core::Error::Http(err))) if err.status().is_none() => {
                return self.offline_account();
            }
            Err(err) => return Err(err),
        };
        Ok(LaunchAccount {
            name: session.profile.name,
            uuid: session.profile.id,
            access_token: session.access_token,
            xuid: None,
        })
    }

    fn offline_account(&self) -> CommandResult<LaunchAccount> {
        let list = self.list();
        let account = list
            .accounts
            .iter()
            .find(|account| Some(&account.id) == list.active.as_ref())
            .ok_or(CommandError::NotSignedIn)?;
        Ok(LaunchAccount {
            name: account.name.clone(),
            uuid: account.id.clone(),
            access_token: "0".into(),
            xuid: None,
        })
    }

    /// A game session for the active account, for calls to Minecraft Services.
    pub async fn session(&self, client: &Client) -> CommandResult<MinecraftSession> {
        let id = self.list().active.ok_or(CommandError::NotSignedIn)?;
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
            self.remember(&session.profile)?;
            sessions.insert(id.clone(), session);
        }
        Ok(sessions[&id].clone())
    }

    /// Keeps the cached session and the saved account in step after a skin or cape change.
    pub async fn set_profile(&self, profile: &Profile) -> CommandResult<()> {
        if let Some(session) = self.sessions.lock().await.get_mut(&profile.id) {
            session.profile = profile.clone();
        }
        self.remember(profile)
    }

    /// The name and skin can change on minecraft.net too, so they are taken from every
    /// fresh profile.
    fn remember(&self, profile: &Profile) -> CommandResult<()> {
        self.update(|list| {
            for account in list
                .accounts
                .iter_mut()
                .filter(|account| account.id == profile.id)
            {
                account.name.clone_from(&profile.name);
                account.skin = active_skin(profile);
            }
        })
    }

    fn update(&self, change: impl FnOnce(&mut AccountList)) -> CommandResult<()> {
        let mut list = lock(&self.list);
        change(&mut list);
        write_list(&self.path, &list)
    }
}

fn write_list(path: &Path, list: &AccountList) -> CommandResult<()> {
    let bytes = serde_json::to_vec_pretty(list).map_err(selene_core::Error::from)?;
    selene_core::fs::write_atomic_blocking(path, &bytes)?;
    Ok(())
}

fn active_skin(profile: &Profile) -> Option<String> {
    profile
        .skins
        .iter()
        .find(|skin| skin.state == "ACTIVE")
        .map(|skin| skin.url.clone())
}

/// Takes the sign-in tokens of every saved account out of the credential store. The
/// uninstaller asks for it when the player deletes everything, since the store outlives it.
pub fn forget_all(path: &Path) {
    let Some(list) = std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<AccountList>(&bytes).ok())
    else {
        return;
    };
    for account in list.accounts {
        if let Ok(entry) = credential(&account.id) {
            let _ = entry.delete_credential();
        }
    }
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
