use crate::error::{PathwayError, Result};
use crate::filesystem::FileSystem;
use serde::{Deserialize, Serialize};
use std::path::Path;
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

pub fn validate_url<F: FileSystem>(input: &str, fs: &F) -> Result<ValidatedUrl> {
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
        // Use to_file_path() for proper cross-platform file path handling
        let path_buf = match url.to_file_path() {
            Ok(path) => path,
            Err(_) => {
                return Err(PathwayError::InvalidUrl(format!(
                    "Invalid file URL: {}",
                    input
                )));
            }
        };

        // Check for path traversal using the string representation
        let path_str = path_buf.to_string_lossy();
        if contains_path_traversal(&path_str) {
            return Err(PathwayError::PathTraversal(path_str.to_string()));
        }
        // Try to canonicalize the path
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
    use crate::filesystem::MockFileSystem;

    #[test]
    fn test_valid_urls() {
        let mut mock_fs = MockFileSystem::new();

        // Setup mock expectations for file URL test
        mock_fs
            .expect_exists()
            .with(mockall::predicate::eq(std::path::Path::new("/etc/hosts")))
            .return_const(true);
        mock_fs
            .expect_canonicalize()
            .with(mockall::predicate::eq(std::path::Path::new("/etc/hosts")))
            .returning(|path| Ok(path.to_path_buf()));

        assert!(validate_url("https://example.com", &mock_fs).is_ok());
        assert!(validate_url("http://localhost:3000/api", &mock_fs).is_ok());

        // Test file URL with mock file system
        assert!(validate_url("file:///etc/hosts", &mock_fs).is_ok());
    }

    #[test]
    fn test_auto_scheme_detection() {
        let mut mock_fs = MockFileSystem::new();

        // Mock exists calls to return false (file doesn't exist)
        mock_fs.expect_exists().returning(|_| false);

        // For auto-detection tests, we need to handle canonicalize calls for file paths
        mock_fs.expect_canonicalize().returning(|path| {
            // Return absolute path for relative paths
            if path.is_absolute() {
                Ok(path.to_path_buf())
            } else {
                Ok(std::env::current_dir().unwrap().join(path))
            }
        });

        assert!(validate_url("example.com", &mock_fs).is_ok());
        assert!(validate_url("/tmp/test.html", &mock_fs).is_ok());
        assert!(validate_url("./relative/path", &mock_fs).is_ok());
    }

    #[test]
    fn test_dangerous_schemes() {
        let mock_fs = MockFileSystem::new();
        assert!(validate_url("javascript:alert(1)", &mock_fs).is_err());
        assert!(validate_url("data:text/html,<h1>test</h1>", &mock_fs).is_err());
        assert!(validate_url("ftp://example.com", &mock_fs).is_err());
    }

    #[test]
    fn test_path_traversal() {
        let mock_fs = MockFileSystem::new();
        assert!(validate_url("file:///../etc/passwd", &mock_fs).is_err());
        assert!(validate_url("file:///tmp/../../../etc/passwd", &mock_fs).is_err());
        // Test case-insensitive percent-encoding detection
        assert!(validate_url("file:///%2E%2E/etc/passwd", &mock_fs).is_err());
        assert!(validate_url("file:///%2E%2E%2F../etc/passwd", &mock_fs).is_err());
    }

    #[test]
    fn test_file_not_found_warning() {
        let mut mock_fs = MockFileSystem::new();

        // Setup mock to simulate file not existing
        mock_fs
            .expect_exists()
            .with(mockall::predicate::eq(std::path::Path::new("/nonexistent")))
            .return_const(false);
        mock_fs
            .expect_canonicalize()
            .with(mockall::predicate::eq(std::path::Path::new("/nonexistent")))
            .returning(|_| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "File not found",
                ))
            });

        let result = validate_url("file:///nonexistent", &mock_fs).unwrap();
        assert!(result.warning.is_some());
        assert!(result.warning.unwrap().contains("File not found"));
    }
}
