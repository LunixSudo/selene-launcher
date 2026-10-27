use std::io;
use std::path::PathBuf;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("invalid json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("checksum mismatch for {url}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        url: String,
        expected: String,
        actual: String,
    },

    #[error("java runtime `{component}` is not available for {platform}")]
    JavaUnavailable { component: String, platform: String },

    #[error("this platform is not supported")]
    UnsupportedPlatform,

    #[error(transparent)]
    Auth(#[from] crate::auth::AuthError),
}

impl Error {
    pub(crate) fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
