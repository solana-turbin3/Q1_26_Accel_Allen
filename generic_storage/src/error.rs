use std::fmt;

#[derive(Debug)]
pub enum StorageError {
    Borsh(std::io::Error),
    Wincode(String),
    Json(serde_json::Error),
    NoData,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Borsh(e) => write!(f, "borsh: {e}"),
            StorageError::Wincode(e) => write!(f, "wincode: {e}"),
            StorageError::Json(e) => write!(f, "json: {e}"),
            StorageError::NoData => write!(f, "no data stored"),
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::Borsh(e)
    }
}

impl From<wincode::WriteError> for StorageError {
    fn from(e: wincode::WriteError) -> Self {
        StorageError::Wincode(e.to_string())
    }
}

impl From<wincode::ReadError> for StorageError {
    fn from(e: wincode::ReadError) -> Self {
        StorageError::Wincode(e.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(e: serde_json::Error) -> Self {
        StorageError::Json(e)
    }
}
