# Pathway URL Router - GitHub Copilot Instructions

Pathway is a lightweight Rust CLI application for URL validation and routing. Always reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.

## Working Effectively

### Bootstrap, Build, and Test the Repository
- Install Rust toolchain: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y`
- Load Rust environment: `source ~/.cargo/env`
- Verify installation: `rustc --version && cargo --version`
- Navigate to core directory: `cd core/`
- Build debug version: `cargo build` -- takes 38 seconds first time. NEVER CANCEL. Set timeout to 90+ minutes for first build.
- Build release version: `cargo build --release` -- takes 25 seconds first time. NEVER CANCEL. Set timeout to 90+ minutes.
- Run test suite: `cargo test` -- takes 11 seconds. NEVER CANCEL. Set timeout to 30+ minutes.
- Run unit tests only: `cargo test --lib` -- takes <1 second
- Run integration tests only: `cargo test --test integration` -- takes <1 second

### Code Quality and Linting
- Format code: `cargo fmt`
- Check formatting: `cargo fmt --check` -- passes cleanly with configured .rustfmt.toml
- Run linter: `cargo clippy` -- takes 8 seconds. Should pass without warnings.
- Fix linting issues: `cargo clippy --fix`
- ALWAYS run `cargo fmt` and `cargo clippy` before committing or CI will fail

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

ALWAYS test the following scenarios after making changes to ensure core functionality works:

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
```
/home/runner/work/pathway/pathway/
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
    ├── Cargo.lock        # Locked dependency versions
    ├── src/
    │   ├── main.rs       # CLI entry point with clap argument parsing
    │   ├── lib.rs        # Library exports
    │   ├── url.rs        # Core URL validation logic
    │   ├── error.rs      # Error types with thiserror
    │   └── logging.rs    # Tracing/logging setup
    └── tests/
        └── integration.rs # 15 integration tests using assert_cmd
```

### Important Code Locations
- CLI argument parsing: `core/src/main.rs` (clap-based)
- URL validation logic: `core/src/url.rs` (supports http/https/file schemes)
- Error handling: `core/src/error.rs` (PathwayError enum)
- Security validation: Path traversal detection in `core/src/url.rs`
- Test coverage: Comprehensive integration tests in `core/tests/integration.rs`
- CI/CD workflows: `.github/workflows/` (comprehensive automation)
- Code formatting: `.rustfmt.toml` (project-specific formatting rules)

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
- Test coverage: All 19 tests must pass (4 unit + 15 integration)

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
# 2025-09-16T21:00:10.032646Z  INFO pathway: URL validated: https://example.com/ (scheme: https)

$ cargo run -- --format json https://example.com
# [{"original":"https://example.com","url":"https://example.com/","normalized":"https://example.com/","scheme":"https","status":"valid"}]

$ cargo run -- --verbose example.com
# 2025-09-16T20:59:40.984854Z DEBUG pathway::url: Input: "example.com"
# 2025-09-16T20:59:40.984916Z DEBUG pathway::url: Auto-detected scheme: https://example.com
# 2025-09-16T20:59:40.984969Z DEBUG pathway::url: Normalized: https://example.com/
# 2025-09-16T20:59:40.984992Z  INFO pathway: URL validated: https://example.com/ (scheme: https)
```

## Security Features
- Rejects dangerous schemes: javascript, data, vbscript, about, blob, ftp, sftp, ssh, telnet
- Path traversal detection for file:// URLs
- URL normalization and validation
- Safe auto-scheme detection (adds https:// for domains, file:// for paths)

## Performance Characteristics
- First build: ~38 seconds (dependencies download)
- Incremental builds: <1 second
- Release build: ~25 seconds first time, <1 second subsequent
- Test suite: ~11 seconds (19 tests) first time, <1 second subsequent
- Release test suite: ~17 seconds first time
- Runtime: <200ms for URL validation

## CI/CD Requirements
- Code must pass `cargo fmt --check` (configured with .rustfmt.toml)
- Code must pass `cargo clippy` without warnings
- All tests must pass with `cargo test`
- Release build must succeed with `cargo build --release`
- Security audit must pass: `cargo audit`
- Dependencies are automatically updated via Dependabot

## GitHub Actions Workflows
The repository includes comprehensive CI/CD automation:

### Core Workflows
- **CI (`ci.yml`)**: Runs tests on Ubuntu, macOS, Windows with stable, beta, nightly Rust
- **Quality (`quality.yml`)**: Enforces code formatting, linting, and security audits
- **Release (`release.yml`)**: Automated releases with cross-platform binaries
- **Dependency Review**: Security scanning for new dependencies

### Development Workflows  
- **Benchmark (`benchmark.yml`)**: Performance regression testing
- **PR Labeler**: Auto-labels PRs based on file changes
- **Update Dependencies**: Automated dependency updates via Dependabot

### Local CI Validation
Run these commands to match CI requirements:
```bash
cd core/
cargo fmt --check    # Must pass
cargo clippy --      # Must have no warnings  
cargo test          # All tests must pass
cargo install cargo-audit  # Install security audit tool (if not present)
cargo audit         # Security audit must pass
```

## Development Tips
- Always work in the `core/` directory for Rust commands
- Use `cargo check` for fast syntax validation during development
- Use `cargo run -- --help` to test CLI changes quickly
- Test both valid and invalid URLs when modifying validation logic
- Integration tests use `assert_cmd` to test the CLI as a black box
- Logging goes to stderr, JSON output goes to stdout