use std::path::PathBuf;

/// Errors produced by the core layer.
#[derive(Debug, thiserror::Error)]
pub enum SpError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid data: {0}")]
    InvalidData(String),

    #[error("unsupported database version {found}, expected {expected}")]
    DatabaseVersion { found: i64, expected: i64 },

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("XML error: {0}")]
    Xml(String),

    #[error("file not found: {0}")]
    NotFound(PathBuf),
}

pub type Result<T> = std::result::Result<T, SpError>;

impl From<quick_xml::Error> for SpError {
    fn from(value: quick_xml::Error) -> Self {
        SpError::Xml(value.to_string())
    }
}

impl From<quick_xml::events::attributes::AttrError> for SpError {
    fn from(value: quick_xml::events::attributes::AttrError) -> Self {
        SpError::Xml(value.to_string())
    }
}

impl From<std::string::FromUtf8Error> for SpError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        SpError::InvalidData(value.to_string())
    }
}

impl From<std::str::Utf8Error> for SpError {
    fn from(value: std::str::Utf8Error) -> Self {
        SpError::InvalidData(value.to_string())
    }
}
