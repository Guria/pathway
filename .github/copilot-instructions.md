# Pathway URL Router - GitHub Copilot Instructions

Pathway is a lightweight Rust CLI tool for URL validation and routing with browser launching capabilities.

## Maintaining These Instructions
- Update when major features/structure changes occur
- Remove outdated information immediately  
- Keep commands/examples current with actual codebase
- Verify all referenced files/paths exist

## Working Effectively

### Bootstrap, Build, and Test
- Navigate to the `core/` directory before running cargo commands.
- Build debug version: `cargo build` — first build may take ~1–3 minutes depending on network and cache.
- Build release version: `cargo build --release` — first build may take ~1–3 minutes.
- Run test suite: `cargo test` — first run may take ~10–60 seconds.
- Run unit tests only: `cargo test --lib` -- takes <1 second
- Run integration tests only: `cargo test --test integration` -- takes <1 second

### Code Quality and Linting
- Format code: `cargo fmt`
- Check formatting: `cargo fmt --check` -- passes cleanly with configured .rustfmt.toml
- Run linter: `cargo clippy -D warnings` — fail on any warnings.
- Fix linting issues: `cargo clippy --fix`
- ALWAYS run `cargo fmt` and `cargo clippy -D warnings` before committing or CI will fail

### Run the Application
- Help: `cargo run -- --help`
- Basic URL validation: `cargo run -- https://example.com`
- JSON output: `cargo run -- --format json https://example.com`
- Verbose mode: `cargo run -- --verbose example.com`
- Multiple URLs: `cargo run -- https://a.com https://b.com`
- Test error handling: `cargo run -- 'javascript:alert(1)'` (should fail)
- Install globally: `cargo install --path .`
- Run installed binary: `pathway --version`

## Validation Scenarios

It is recommended to test the following scenarios after making changes to ensure core functionality works:

### Basic Functionality Test
```bash
cd core/
cargo run -- https://example.com
# Expected: SUCCESS with "URL validated: https://example.com/ (scheme: https)"

cargo run -- --format json https://example.com  
# Expected: SUCCESS with JSON output containing "status":"valid"

cargo run -- 'javascript:alert(1)'
# Expected: FAILURE with "Unsupported scheme: javascript"
```

### Auto-Detection Test
```bash
cargo run -- example.com
# Expected: SUCCESS with auto-detection to https://example.com/

cargo run -- /tmp
# Expected: SUCCESS with auto-detection to file:///tmp
```

### Error Handling Test
```bash
cargo run -- 'file:///../etc/passwd'
# Expected: FAILURE with "Path traversal detected"

cargo run -- ftp://example.com
# Expected: FAILURE with "Unsupported scheme: ftp"
```

## Repository Structure

### Key Directories and Files
```text
pathway/
├── README.md              # Project overview and status
├── .mise.toml            # Rust 1.82 toolchain configuration
├── .rustfmt.toml         # Rust formatting configuration
├── rust-toolchain.toml   # Rust toolchain specification
├── .gitignore            # Standard Rust gitignore
├── .github/              # GitHub workflows and configurations
│   ├── workflows/        # CI/CD workflows
│   │   ├── ci.yml        # Main CI pipeline (test on multiple platforms)
│   │   ├── quality.yml   # Code quality checks (fmt, clippy, audit)
│   │   ├── release.yml   # Automated releases and binaries
│   │   ├── benchmark.yml # Performance benchmarking
│   │   ├── dependency-review.yml # Dependency security review
│   │   ├── pr-labeler.yml # Auto-label PRs
│   │   └── update-deps.yml # Dependabot updates
│   ├── dependabot.yml    # Dependabot configuration
│   └── labeler.yml       # PR labeling rules
└── core/                 # Main Rust project
    ├── Cargo.toml        # Dependencies and project metadata
    ├── deny.toml         # License and security configuration
    ├── src/
    │   ├── main.rs       # CLI entry point
    │   ├── lib.rs        # Library exports
    │   ├── url.rs        # Core URL validation logic
    │   ├── error.rs      # Error types
    │   ├── logging.rs    # Tracing/logging setup
    │   └── browser/      # Browser launching functionality
    │       ├── mod.rs    # Cross-platform browser interface
    │       ├── macos.rs  # macOS browser support
    │       ├── linux.rs  # Linux browser support
    │       ├── windows.rs # Windows browser support
    │       └── unknown.rs # Fallback for unknown platforms
    └── tests/
        └── integration.rs # Integration tests
```

### Important Code Locations
- CLI interface: `core/src/main.rs`
- URL validation: `core/src/url.rs` (http/https/file schemes, path traversal detection)
- Browser launching: `core/src/browser/` (cross-platform browser support)
- Error handling: `core/src/error.rs`
- Tests: `core/tests/integration.rs`
- CI/CD: `.github/workflows/`

## Build System and Dependencies

### Key Dependencies
- `clap 4.5` - CLI argument parsing with derive macros
- `url 2.5` - URL parsing and validation
- `tracing/tracing-subscriber` - Structured logging
- `serde/serde_json` - JSON serialization for output
- `thiserror/anyhow` - Error handling

### Development Dependencies
- `assert_cmd 2.0` - CLI testing framework
- `predicates 3.1` - Test assertions

### Build Artifacts
- Debug binary: `core/target/debug/pathway`
- Release binary: `core/target/release/pathway`

## Common Commands Reference

### Frequently Used Commands
```bash
# Full build and test cycle (use for CI validation)
cd core/ && cargo build && cargo test && cargo clippy && cargo fmt --check

# Quick development cycle
cd core/ && cargo check && cargo test

# Release preparation
cd core/ && cargo build --release && cargo test --release

# Install for system-wide use
cd core/ && cargo install --path .
```

### Expected Output Examples
```bash
$ cargo run -- https://example.com
# <timestamp>  INFO pathway: URL validated: https://example.com/ (scheme: https)

$ cargo run -- --format json https://example.com
# [{"original":"https://example.com","url":"https://example.com/","normalized":"https://example.com/","scheme":"https","status":"valid"}]

$ cargo run -- --verbose example.com
# <timestamp> DEBUG pathway::url: Input: "example.com"
# <timestamp> DEBUG pathway::url: Auto-detected scheme: https://example.com
# <timestamp> DEBUG pathway::url: Normalized: https://example.com/
# <timestamp>  INFO pathway: URL validated: https://example.com/ (scheme: https)
```

## Security & Performance
- **Security**: Blocks dangerous schemes, detects path traversal, normalizes URLs
- **Performance**: Fast validation, first build ~1-3min, incremental builds are fast

## CI/CD
- **Required**: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` must pass
- **Workflows**: Multi-platform CI, quality checks, automated releases, dependency updates
- **Security**: Automated audits, license compliance via `cargo-deny`

## Development Tips
- Work in `core/` directory for Rust commands
- Use `cargo check` for fast syntax validation
- Test both valid and invalid URLs when modifying validation logic
- Logging goes to stderr, JSON output goes to stdout