use std::io;
use std::path::{Path, PathBuf};

#[cfg(test)]
use mockall::automock;

/// File system abstraction for testing.
#[cfg_attr(test, automock)]
pub trait FileSystem {
    /// Check if a path exists
    fn exists(&self, path: &Path) -> bool;

    /// Check if a path is a directory
    fn is_dir(&self, path: &Path) -> bool;

    /// Create a directory and all parent directories as needed
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Remove a file
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Write content to a file, creating it if necessary
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()>;

    /// Read the entire contents of a file into a string
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// Canonicalize a path, returning the absolute form with all components resolved
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    /// Get metadata for a file or directory
    fn metadata(&self, path: &Path) -> io::Result<std::fs::Metadata>;
}

/// Real file system implementation that delegates to std::fs
#[derive(Debug, Clone, Default)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        std::fs::write(path, contents)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        path.canonicalize()
    }

    fn metadata(&self, path: &Path) -> io::Result<std::fs::Metadata> {
        std::fs::metadata(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_filesystem_delegation() {
        let fs = RealFileSystem;

        // Test that exists delegates to Path::exists
        // Use current directory which should always exist
        let current_dir = std::env::current_dir().unwrap();
        assert!(fs.exists(&current_dir));

        // Test canonicalize - verify it returns a canonical form
        // On Windows, canonicalize may return UNC paths, so we just verify it succeeds
        // and that the canonical path exists
        let canonical = fs.canonicalize(&current_dir).unwrap();
        assert!(canonical.is_absolute());
        assert!(fs.exists(&canonical));
    }

    #[test]
    fn test_mock_filesystem() {
        let mut mock_fs = MockFileSystem::new();

        // Setup expectations
        mock_fs
            .expect_exists()
            .with(mockall::predicate::eq(Path::new("/test/file.txt")))
            .return_const(true);

        mock_fs
            .expect_is_dir()
            .with(mockall::predicate::eq(Path::new("/test/file.txt")))
            .return_const(false);

        // Test the mock
        assert!(mock_fs.exists(Path::new("/test/file.txt")));
        assert!(!mock_fs.is_dir(Path::new("/test/file.txt")));
    }
}
