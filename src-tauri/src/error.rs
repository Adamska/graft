use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no repository is open")]
    NoRepository,
    #[error("{0}")]
    Git(String),
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// gix surfaces a distinct error type per operation; the UI only ever shows the message.
pub trait GitResultExt<T> {
    fn git(self) -> Result<T>;
}

impl<T, E: std::fmt::Display> GitResultExt<T> for std::result::Result<T, E> {
    fn git(self) -> Result<T> {
        self.map_err(|error| Error::Git(error.to_string()))
    }
}
