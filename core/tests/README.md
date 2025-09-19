# Test Organization for GitHub Actions

The test suite has been split into two separate files to support running in different environments:

## Core Integration Tests (`tests/integration.rs`)

Browser-agnostic tests that should run in any environment, including CI/CD:

- Basic URL validation tests
- Help command tests  
- Browser listing (doesn't require specific browsers)
- Error handling tests
- System default browser warnings

**Run with:** `cargo test --test integration`

These tests are safe to run in GitHub Actions and other CI environments where specific browsers may not be installed.

## Browser-Dependent Tests (`tests/browser_integration.rs`)

Tests that require specific browsers to be installed:

- Chrome-specific functionality tests
- Safari-specific functionality tests  
- Firefox-specific functionality tests
- Browser profile management tests
- Window options tests with actual browsers

**Run with:** `cargo test --test browser_integration`

These tests will automatically skip if the required browser is not available on the system.

## Using Exclusion in Cargo

You can use cargo's exclusion features to run specific test suites:

### Core tests only (recommended for CI)
```bash
# Run unit tests + core integration tests (excludes browser tests)
cargo test --lib --test integration

# Same thing with verbose output
cargo test --lib --test integration --verbose
```

### Browser tests only
```bash
# Run only browser-dependent tests
cargo test --test browser_integration
```

### All tests except browser tests
```bash
# Alternative: run unit tests and core integration only
cargo test --lib --test integration
```

## GitHub Actions Usage

For GitHub Actions, the CI now uses exclusion to run reliable tests:

### Basic CI (always run) - Updated
```yaml
- name: Run core tests
  run: cargo test --lib --test integration --verbose

- name: Run browser tests (optional)
  run: cargo test --test browser_integration --verbose
  continue-on-error: true  # Don't fail CI if browsers aren't available
```

### Extended CI with browsers (when browsers are installed)
```yaml
- name: Install Chrome (Ubuntu)
  run: |
    wget -q -O - https://dl.google.com/linux/linux_signing_key.pub | sudo apt-key add -
    sudo apt-get update
    sudo apt-get install google-chrome-stable

- name: Run all tests including browser tests
  run: cargo test --verbose
```

## Local Development

When developing locally, you can run different combinations:

```bash
# Core functionality only (fast, reliable)
cargo test --lib --test integration

# Browser-specific tests only  
cargo test --test browser_integration

# All tests (will skip unavailable browser tests gracefully)
cargo test

# All unit tests only
cargo test --lib

# Specific test by name
cargo test test_launch_https_url
```

## Test Count Summary

- **Unit Tests**: 4 tests (URL validation, path traversal, etc.)
- **Core Integration Tests**: 8 tests (help commands, browser listing, etc.)  
- **Browser Integration Tests**: 11 tests (Chrome/Safari/Firefox specific)
- **Total**: 23 tests