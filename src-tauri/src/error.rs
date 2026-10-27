use selene_core::auth::AuthError;
use serde::{Serialize, Serializer};

/// Errors sent to the UI. `kind` picks the translated message, `message` has the details.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("not signed in")]
    NotSignedIn,

    #[error("sign-in was cancelled")]
    SignInCancelled,

    #[error("credential store: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("unknown version {0}")]
    UnknownVersion(String),

    #[error("unknown instance {0}")]
    UnknownInstance(String),

    #[error("this instance has no mod loader")]
    NoLoader,

    #[error("no version of this mod runs on this instance")]
    NoCompatibleVersion,

    #[error("instance {0} is already running")]
    AlreadyRunning(String),

    #[error("could not start the game: {0}")]
    Spawn(#[source] std::io::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Core(#[from] selene_core::Error),
}

pub type CommandResult<T> = Result<T, CommandError>;

impl CommandError {
    fn kind(&self) -> &'static str {
        match self {
            Self::NotSignedIn => "notSignedIn",
            Self::SignInCancelled => "signInCancelled",
            Self::Keyring(_) => "keyring",
            Self::UnknownVersion(_) => "unknownVersion",
            Self::UnknownInstance(_) => "unknownInstance",
            Self::NoLoader => "noLoader",
            Self::NoCompatibleVersion => "noCompatibleVersion",
            Self::AlreadyRunning(_) => "alreadyRunning",
            Self::Spawn(_) => "spawn",
            Self::Io(_) => "io",
            Self::Core(selene_core::Error::Http(_)) => "network",
            Self::Core(selene_core::Error::Auth(error)) => match error {
                AuthError::Denied(_) | AuthError::InvalidCallback => "signInCancelled",
                AuthError::NoXboxAccount => "noXboxAccount",
                AuthError::RegionBlocked => "regionBlocked",
                AuthError::ChildAccount => "childAccount",
                AuthError::GameNotOwned => "gameNotOwned",
                AuthError::Service { .. } => "authService",
            },
            Self::Core(_) => "core",
        }
    }
}

impl Serialize for CommandError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("CommandError", 2)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}
