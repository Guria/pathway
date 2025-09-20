# Virtual File System for Testing

This document describes the virtual file system abstraction introduced to make tests more robust and isolated.

## Problem Statement

Previously, tests relied on the real file system of the host machine, which caused several issues:

1. **Environment dependency**: Tests would pass or fail depending on what files/directories existed on the host
2. **Potential harm**: Tests could create files in system directories
3. **Non-deterministic results**: Test outcomes depended on the current state of the host file system
4. **Cleanup issues**: Tests might leave artifacts in the file system

## Solution: File System Abstraction

We introduced a `FileSystem` trait that abstracts file system operations, allowing dependency injection of either:
- `RealFileSystem`: Delegates to standard library file operations for production use
- `MockFileSystem`: Provides controlled, predictable behavior for testing

### Core Components

#### `FileSystem` trait (`src/filesystem.rs`)
```rust
pub trait FileSystem {
    fn exists(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()>;
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
    fn metadata(&self, path: &Path) -> io::Result<fs::Metadata>;
}
```

#### `RealFileSystem`
- Default implementation for production use
- Delegates all operations to `std::fs` functions
- Maintains existing behavior for production code

#### `MockFileSystem`
- Test implementation with predictable behavior
- Maintains virtual files and directories in memory
- Always succeeds for write operations (configurable in tests)
- Allows pre-populating with test data

### Usage in Code

#### Production Code
Functions that need file system access now have two versions:
- Public function using `RealFileSystem` (maintains API compatibility)
- Internal function accepting any `FileSystem` implementation

Example:
```rust
// Public API - unchanged for backwards compatibility
pub fn validate_url(input: &str) -> Result<ValidatedUrl> {
    validate_url_with_fs(input, &RealFileSystem)
}

// Internal implementation accepting file system
pub fn validate_url_with_fs<F: FileSystem>(input: &str, fs: &F) -> Result<ValidatedUrl> {
    // Use fs.exists(), fs.canonicalize(), etc.
}
```

#### Test Code
Tests can now use `MockFileSystem` for controlled, isolated testing:

```rust
#[test]
fn test_file_validation() {
    let mut fs = MockFileSystem::new();
    fs.add_file("/test/file.txt", b"test content");
    
    let result = validate_url_with_fs("file:///test/file.txt", &fs);
    assert!(result.is_ok());
}
```

### Benefits

1. **Isolated tests**: No dependency on host file system state
2. **Predictable results**: Tests always behave the same way
3. **Fast execution**: No actual disk I/O in unit tests
4. **No cleanup needed**: Virtual file system exists only in memory
5. **Easy test data setup**: Pre-populate with exactly the files needed

### Files Updated

- `src/filesystem.rs`: New file system abstraction
- `src/lib.rs`: Export new filesystem module
- `src/url.rs`: Updated to use file system abstraction
- `src/profile.rs`: Updated `prepare_custom_directory` to use abstraction
- `tests/virtual_filesystem_integration.rs`: New integration tests using temp directories

### Testing Strategy

1. **Unit tests**: Use `MockFileSystem` for fast, isolated testing
2. **Integration tests**: Use `TempDir` for testing real file operations without polluting host system
3. **Existing tests**: Continue to work unchanged (they use temp directories or real browsers)

### Future Improvements

- Extend file system abstraction to browser detection modules
- Create more sophisticated mock implementations that track operations
- Add file system state validation in tests
- Consider using the abstraction for profile discovery functions