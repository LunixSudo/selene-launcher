use selene_core::launch::LaunchAccount;

use crate::error::{CommandError, CommandResult};

/// The account the next game starts with.
pub fn launch_account() -> CommandResult<LaunchAccount> {
    dev_account().ok_or(CommandError::NotSignedIn)
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
