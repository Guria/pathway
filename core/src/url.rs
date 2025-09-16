use crate::error::{PathwayError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};
use url::Url;

const DANGEROUS_SCHEMES: &[&str] = &[
    "javascript",
    "data",
    "vbscript",
    "about",
    "blob",
    "ftp",
    "sftp",
    "ssh",
    "telnet",
];

const SUPPORTED_SCHEMES: &[&str] = &["http", "https", "file"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedUrl {
    pub original: String,
    pub url: String,
    pub normalized: String,
    pub scheme: String,
    pub status: ValidationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationStatus {
    Valid,
    Invalid,
}

pub fn validate_url(input: &str) -> Result<ValidatedUrl> {
    debug!("Input: \"{}\"", input);

    // Check for path traversal in the original input first
    if input.starts_with("file://") && contains_path_traversal(input) {
        return Err(PathwayError::PathTraversal(input.to_string()));
    }

    // Try to parse as-is first
    let url = match Url::parse(input) {
        Ok(url) => url,
        Err(_) => {
            // Auto-detect scheme
            let with_scheme = auto_detect_scheme(input)?;
            debug!("Auto-detected scheme: {}", with_scheme);
            Url::parse(&with_scheme)?
        }
    };

    // Check for dangerous schemes
    if DANGEROUS_SCHEMES.contains(&url.scheme()) {
        return Err(PathwayError::UnsupportedScheme(url.scheme().to_string()));
    }

    // Check for supported schemes
    if !SUPPORTED_SCHEMES.contains(&url.scheme()) {
        return Err(PathwayError::UnsupportedScheme(url.scheme().to_string()));
    }

    let mut warning = None;

    // Special handling for file URLs
    let normalized = if url.scheme() == "file" {
        let path = url.path();

        // Check for path traversal
        if contains_path_traversal(path) {
            return Err(PathwayError::PathTraversal(path.to_string()));
        }

        // Try to canonicalize the path
        let path_buf = PathBuf::from(path);
        match path_buf.canonicalize() {
            Ok(canonical) => {
                // Check if file exists
                if !canonical.exists() {
                    warning = Some(format!("File not found: {}", canonical.display()));
                    warn!("File not found: {}", canonical.display());
                }
                format!("file://{}", canonical.display())
            }
            Err(_) => {
                // If canonicalization fails, check if it's because the file doesn't exist
                if !path_buf.exists() {
                    warning = Some(format!("File not found: {}", path_buf.display()));
                    warn!("File not found: {}", path_buf.display());
                }
                url.to_string()
            }
        }
    } else {
        url.to_string()
    };

    debug!("Normalized: {}", normalized);

    Ok(ValidatedUrl {
        original: input.to_string(),
        url: url.to_string(),
        normalized,
        scheme: url.scheme().to_string(),
        status: ValidationStatus::Valid,
        warning,
    })
}

fn auto_detect_scheme(input: &str) -> Result<String> {
    // Check if it's a file path
    if input.starts_with('/') || input.starts_with("./") || input.starts_with("../") {
        // It's a file path
        let path = Path::new(input);
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };

        Ok(format!("file://{}", absolute.display()))
    } else if !input.contains("://") && (input.contains('.') || input.contains("localhost")) {
        // Likely a domain name
        Ok(format!("https://{}", input))
    } else {
        Err(PathwayError::InvalidUrl(format!(
            "Cannot auto-detect scheme for: {}",
            input
        )))
    }
}

fn contains_path_traversal(path: &str) -> bool {
    // Check for various path traversal patterns
    path.contains("../")
        || path.contains("..\\")
        || path.contains("....")
        || path.contains("%2e%2e")
        || path.contains("%2e%2e%2f")
        || path.contains("%2e%2e%5c")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_urls() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://localhost:3000/api").is_ok());
        assert!(validate_url("file:///etc/hosts").is_ok());
    }

    #[test]
    fn test_auto_scheme_detection() {
        assert!(validate_url("example.com").is_ok());
        assert!(validate_url("/tmp/test.html").is_ok());
        assert!(validate_url("./relative/path").is_ok());
    }

    #[test]
    fn test_dangerous_schemes() {
        assert!(validate_url("javascript:alert(1)").is_err());
        assert!(validate_url("data:text/html,<h1>test</h1>").is_err());
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_path_traversal() {
        assert!(validate_url("file:///../etc/passwd").is_err());
        assert!(validate_url("file:///tmp/../../../etc/passwd").is_err());
    }
}
