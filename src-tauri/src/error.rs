use serde::{Serialize, Serializer};

/// Errors sent to the UI. `kind` picks the translated message, `message` has the details.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("not signed in")]
    NotSignedIn,

    #[error("unknown version {0}")]
    UnknownVersion(String),

    #[error("unknown instance {0}")]
    UnknownInstance(String),

    #[error("instance {0} is already running")]
    AlreadyRunning(String),

    #[error("could not start the game: {0}")]
    Spawn(#[from] std::io::Error),

    #[error(transparent)]
    Core(#[from] selene_core::Error),
}

pub type CommandResult<T> = Result<T, CommandError>;

impl CommandError {
    fn kind(&self) -> &'static str {
        match self {
            Self::NotSignedIn => "notSignedIn",
            Self::UnknownVersion(_) => "unknownVersion",
            Self::UnknownInstance(_) => "unknownInstance",
            Self::AlreadyRunning(_) => "alreadyRunning",
            Self::Spawn(_) => "spawn",
            Self::Core(selene_core::Error::Http(_)) => "network",
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
