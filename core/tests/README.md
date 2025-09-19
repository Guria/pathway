# Test Organization

The test suite is organized into two main files:

## Core Integration Tests (`tests/integration.rs`)

Browser-agnostic tests that validate core functionality:

- Basic URL validation tests
- Help command tests  
- Browser listing
- Error handling tests
- System default browser warnings

## Browser Integration Tests (`tests/browser_integration.rs`)

Tests that work with specific browsers when available:

- Chrome-specific functionality tests
- Safari-specific functionality tests  
- Firefox-specific functionality tests
- Browser profile management tests
- Window options tests with actual browsers

These tests will automatically skip if the required browser is not available on the system.

## Local Development

### Running Tests

```bash
# All tests (recommended)
cargo test

# Unit tests only
cargo test --lib

# Specific test file
cargo test --test integration
cargo test --test browser_integration

# Specific test by name
cargo test test_launch_https_url
```

### Local Quality Checks

```bash
# Full development workflow
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo doc --no-deps --document-private-items --all-features
```

## Test Count Summary

- **Unit Tests**: 4 tests (URL validation, path traversal, etc.)
- **Core Integration Tests**: 8 tests (help commands, browser listing, etc.)  
- **Browser Integration Tests**: 11 tests (Chrome/Safari/Firefox specific)
- **Total**: 23 tests