use crate::error::{PathwayError, Result};
use crate::filesystem::{FileSystem, RealFileSystem};
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
    validate_url_with_fs(input, &RealFileSystem)
}

pub fn validate_url_with_fs<F: FileSystem>(input: &str, fs: &F) -> Result<ValidatedUrl> {
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
        match fs.canonicalize(&path_buf) {
            Ok(canonical) => {
                // Check if file exists
                if !fs.exists(&canonical) {
                    warning = Some(format!("File not found: {}", canonical.display()));
                    warn!("File not found: {}", canonical.display());
                }
                format!("file://{}", canonical.display())
            }
            Err(_) => {
                // If canonicalization fails, check if it's because the file doesn't exist
                if !fs.exists(&path_buf) {
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
    // Normalize to ASCII lowercase to match percent-encodings regardless of case.
    let p = path.to_ascii_lowercase();
    p.contains("../")
        || p.contains("..\\")
        || p.contains("....")
        || p.contains("%2e%2e")
        || p.contains("%2e%2e%2f")
        || p.contains("%2e%2e%5c")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::mock::MockFileSystem;

    #[test]
    fn test_valid_urls() {
        let fs = MockFileSystem::new();
        assert!(validate_url_with_fs("https://example.com", &fs).is_ok());
        assert!(validate_url_with_fs("http://localhost:3000/api", &fs).is_ok());

        // Test file URL with mock file system
        let mut fs = MockFileSystem::new();
        fs.add_file("/etc/hosts", b"test content");
        assert!(validate_url_with_fs("file:///etc/hosts", &fs).is_ok());
    }

    #[test]
    fn test_auto_scheme_detection() {
        let fs = MockFileSystem::new();
        assert!(validate_url_with_fs("example.com", &fs).is_ok());
        assert!(validate_url_with_fs("/tmp/test.html", &fs).is_ok());
        assert!(validate_url_with_fs("./relative/path", &fs).is_ok());
    }

    #[test]
    fn test_dangerous_schemes() {
        let fs = MockFileSystem::new();
        assert!(validate_url_with_fs("javascript:alert(1)", &fs).is_err());
        assert!(validate_url_with_fs("data:text/html,<h1>test</h1>", &fs).is_err());
        assert!(validate_url_with_fs("ftp://example.com", &fs).is_err());
    }

    #[test]
    fn test_path_traversal() {
        let fs = MockFileSystem::new();
        assert!(validate_url_with_fs("file:///../etc/passwd", &fs).is_err());
        assert!(validate_url_with_fs("file:///tmp/../../../etc/passwd", &fs).is_err());
        // Test case-insensitive percent-encoding detection
        assert!(validate_url_with_fs("file:///%2E%2E/etc/passwd", &fs).is_err());
        assert!(validate_url_with_fs("file:///%2E%2E%2F../etc/passwd", &fs).is_err());
    }
}
