use thiserror::Error;

#[derive(Error, Debug)]
pub enum PathwayError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Unsupported scheme: {0}")]
    UnsupportedScheme(String),

    #[error("Path traversal detected in file URL: {0}")]
    PathTraversal(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Failed to canonicalize path: {0}")]
    CanonicalizationError(#[from] std::io::Error),

    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
}

pub type Result<T> = std::result::Result<T, PathwayError>;
