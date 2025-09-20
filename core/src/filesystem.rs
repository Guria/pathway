use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Abstraction over file system operations to enable testing with virtual file systems.
///
/// This trait provides the minimal set of file system operations needed by the application,
/// allowing for dependency injection of either a real file system or a mock for testing.
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
    fn metadata(&self, path: &Path) -> io::Result<fs::Metadata>;
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
        fs::create_dir_all(path)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        fs::write(path, contents)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        path.canonicalize()
    }

    fn metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        fs::metadata(path)
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Error, ErrorKind};

    /// Mock file system for testing
    #[derive(Debug, Clone, Default)]
    pub struct MockFileSystem {
        files: HashMap<PathBuf, Vec<u8>>,
        directories: HashMap<PathBuf, bool>,
    }

    impl MockFileSystem {
        pub fn new() -> Self {
            let mut fs = Self::default();
            // Add root directory by default
            fs.directories.insert(PathBuf::from("/"), true);
            fs
        }

        /// Add a file to the mock file system
        pub fn add_file<P: AsRef<Path>>(&mut self, path: P, contents: &[u8]) {
            let path = path.as_ref().to_path_buf();
            self.files.insert(path.clone(), contents.to_vec());

            // Also add parent directories
            if let Some(parent) = path.parent() {
                self.add_dir(parent);
            }
        }

        /// Add a directory to the mock file system
        pub fn add_dir<P: AsRef<Path>>(&mut self, path: P) {
            let path = path.as_ref().to_path_buf();
            self.directories.insert(path.clone(), true);

            // Also add parent directories
            if let Some(parent) = path.parent() {
                if parent != Path::new("/") {
                    self.add_dir(parent);
                }
            }
        }

        /// Check if a file exists in the mock file system
        pub fn has_file<P: AsRef<Path>>(&self, path: P) -> bool {
            self.files.contains_key(path.as_ref())
        }

        /// Check if a directory exists in the mock file system
        pub fn has_dir<P: AsRef<Path>>(&self, path: P) -> bool {
            self.directories.contains_key(path.as_ref())
        }

        /// Remove a file from the mock file system
        pub fn remove_file<P: AsRef<Path>>(&mut self, path: P) {
            self.files.remove(path.as_ref());
        }

        /// Remove a directory from the mock file system
        pub fn remove_dir<P: AsRef<Path>>(&mut self, path: P) {
            self.directories.remove(path.as_ref());
        }
    }

    impl FileSystem for MockFileSystem {
        fn exists(&self, path: &Path) -> bool {
            self.files.contains_key(path) || self.directories.contains_key(path)
        }

        fn is_dir(&self, path: &Path) -> bool {
            self.directories.contains_key(path)
        }

        fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
            // Mock implementation - always succeeds
            Ok(())
        }

        fn remove_file(&self, _path: &Path) -> io::Result<()> {
            // Mock implementation - always succeeds
            Ok(())
        }

        fn write(&self, _path: &Path, _contents: &[u8]) -> io::Result<()> {
            // Mock implementation - always succeeds
            Ok(())
        }

        fn read_to_string(&self, path: &Path) -> io::Result<String> {
            if let Some(contents) = self.files.get(path) {
                String::from_utf8(contents.clone())
                    .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid UTF-8"))
            } else {
                Err(Error::new(ErrorKind::NotFound, "File not found"))
            }
        }

        fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
            // Simple mock - just return the path as absolute
            if path.is_absolute() {
                Ok(path.to_path_buf())
            } else {
                Ok(PathBuf::from("/").join(path))
            }
        }

        fn metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
            if self.exists(path) {
                // For mock purposes, we'll create a minimal metadata
                // This is a placeholder - in practice you might need a mock metadata struct
                Err(Error::new(
                    ErrorKind::Unsupported,
                    "Mock metadata not implemented",
                ))
            } else {
                Err(Error::new(ErrorKind::NotFound, "File not found"))
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_mock_filesystem_basic_operations() {
            let mut fs = MockFileSystem::new();

            // Test directory operations
            fs.add_dir("/test/dir");
            assert!(fs.exists(Path::new("/test/dir")));
            assert!(fs.is_dir(Path::new("/test/dir")));
            assert!(fs.exists(Path::new("/test"))); // Parent should exist too

            // Test file operations
            fs.add_file("/test/file.txt", b"test content");
            assert!(fs.exists(Path::new("/test/file.txt")));
            assert!(!fs.is_dir(Path::new("/test/file.txt")));
            assert!(fs.has_file("/test/file.txt"));

            // Test read_to_string
            let content = fs.read_to_string(Path::new("/test/file.txt")).unwrap();
            assert_eq!(content, "test content");

            // Test non-existent file
            assert!(!fs.exists(Path::new("/nonexistent")));
            assert!(fs.read_to_string(Path::new("/nonexistent")).is_err());
        }

        #[test]
        fn test_mock_filesystem_file_operations() {
            let fs = MockFileSystem::new();

            // Test write operation (mock always succeeds)
            assert!(fs.write(Path::new("/test.txt"), b"content").is_ok());

            // Test remove operations (mock always succeeds)
            assert!(fs.remove_file(Path::new("/test.txt")).is_ok());

            // Test create_dir_all (mock always succeeds)
            assert!(fs.create_dir_all(Path::new("/deep/nested/path")).is_ok());
        }

        #[test]
        fn test_real_filesystem_delegation() {
            let fs = RealFileSystem;

            // Test that exists delegates to Path::exists
            // Use root path which should always exist
            assert!(fs.exists(Path::new("/")));

            // Test canonicalize with current directory
            let current_dir = std::env::current_dir().unwrap();
            let canonical = fs.canonicalize(&current_dir).unwrap();
            assert_eq!(canonical, current_dir);
        }
    }
}
