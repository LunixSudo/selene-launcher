use reqwest::StatusCode;
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

    #[error("data packs need a world to go into")]
    NoWorld,

    #[error("no version of this mod runs on this instance")]
    NoCompatibleVersion,

    #[error("instance {0} is already running")]
    AlreadyRunning(String),

    #[error("a game is running or installing")]
    GameRunning,

    #[error("java not found at {}", .0.display())]
    JavaNotFound(std::path::PathBuf),

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
            Self::NoWorld => "noWorld",
            Self::NoCompatibleVersion => "noCompatibleVersion",
            Self::AlreadyRunning(_) => "alreadyRunning",
            Self::JavaNotFound(_) => "javaNotFound",
            Self::GameRunning => "gameRunning",
            Self::Spawn(_) => "spawn",
            Self::Io(err) | Self::Core(selene_core::Error::Io { source: err, .. }) => io_kind(err),
            Self::Core(
                selene_core::Error::Http(_) | selene_core::Error::ChecksumMismatch { .. },
            ) => "network",
            Self::Core(selene_core::Error::InvalidPack(_)) => "invalidPack",
            Self::Core(selene_core::Error::UnsupportedLoader(_)) => "unsupportedLoader",
            Self::Core(selene_core::Error::LoaderInstaller(_)) => "loaderInstaller",
            Self::Core(selene_core::Error::InvalidSkin) => "invalidSkin",
            Self::Core(selene_core::Error::Auth(error)) => match error {
                AuthError::Denied(_) | AuthError::InvalidCallback => "signInCancelled",
                AuthError::NoXboxAccount => "noXboxAccount",
                AuthError::RegionBlocked => "regionBlocked",
                AuthError::ChildAccount => "childAccount",
                AuthError::GameNotOwned => "gameNotOwned",
                AuthError::SessionExpired => "sessionExpired",
                // Mojang answers 403 until it has approved the app's Azure client ID.
                AuthError::Service {
                    service: "Minecraft Services",
                    status: StatusCode::FORBIDDEN,
                } => "appNotApproved",
                AuthError::Service { .. } => "authService",
            },
            Self::Core(_) => "core",
        }
    }
}

fn io_kind(err: &std::io::Error) -> &'static str {
    if err.kind() == std::io::ErrorKind::StorageFull {
        "diskFull"
    } else {
        "io"
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
