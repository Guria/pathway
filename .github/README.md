# GitHub Actions CI/CD Setup

This directory contains comprehensive GitHub Actions workflows for the Pathway project.

## Workflows Created

### 1. CI Workflow (`.github/workflows/ci.yml`)
- **Triggers**: Push/PR to main and develop branches
- **Features**:
  - Multi-platform testing (Ubuntu, macOS, Windows)
  - Multi-version testing (stable, beta, nightly Rust)
  - Code formatting checks (rustfmt)
  - Linting (clippy)
  - Tests in debug and release modes
  - Code coverage with Codecov
  - Security audits with cargo-audit
  - MSRV (Minimum Supported Rust Version) verification

### 2. Release Workflow (`.github/workflows/release.yml`)
- **Triggers**: Version tags (v*.*.*)
- **Features**:
  - Automated GitHub release creation
  - Multi-platform binary builds:
    - Linux (x86_64, aarch64, musl)
    - macOS (x86_64, Apple Silicon)
    - Windows (x86_64)
  - Automatic asset uploads
  - Cargo publish preparation (currently dry-run)

### 3. Code Quality Workflow (`.github/workflows/quality.yml`)
- **Triggers**: Push/PR to main and develop branches
- **Features**:
  - Rustfmt formatting checks
  - Clippy linting on all platforms
  - Documentation generation checks
  - Unused dependency detection (cargo-machete)
  - License compliance (cargo-deny)

### 4. Additional Workflows
- **PR Labeler** (`.github/workflows/pr-labeler.yml`): Auto-labels PRs based on changed files
- **Dependency Review** (`.github/workflows/dependency-review.yml`): Security review of dependencies
- **Benchmarks** (`.github/workflows/benchmark.yml`): Performance benchmarking
- **Update Dependencies** (`.github/workflows/update-deps.yml`): Weekly automated dependency updates

## Configuration Files

- **`.github/dependabot.yml`**: Automated dependency updates for Rust and GitHub Actions
- **`.github/labeler.yml`**: Rules for auto-labeling PRs
- **`core/deny.toml`**: License and security configuration for cargo-deny
- **`rust-toolchain.toml`**: Ensures consistent Rust version
- **`.rustfmt.toml`**: Code formatting rules

## Local Tool Execution Results

### ✅ Successfully Run
- `cargo fmt --all -- --check` - Code formatting verified
- `cargo clippy --all-targets --all-features -- -D warnings` - No linting issues
- `cargo test --verbose` - All tests pass
- `cargo test --release --verbose` - All tests pass in release mode
- `cargo doc --no-deps --document-private-items --all-features` - Documentation builds

### ✅ Additional Tools
These tools are compatible with stable Rust toolchains and provide extended functionality:
- `cargo-machete` - For checking unused dependencies
- `cargo-deny` - For license and security checks  
- `cargo-audit` - For security vulnerability scanning

All tools work with recent stable Rust versions (1.70+).

## Setup Requirements

### GitHub Secrets
Add these secrets to your repository:
- `CODECOV_TOKEN` - For code coverage reporting
- `CARGO_REGISTRY_TOKEN` - For publishing to crates.io (when ready)

### Branch Protection
Consider enabling branch protection rules requiring:
- CI checks to pass
- Code review before merging
- Up-to-date branches

## Usage

All workflows will run automatically based on their triggers. You can also manually trigger:
- CI workflow via workflow_dispatch
- Release workflow via workflow_dispatch with tag input
- Benchmark workflow via workflow_dispatch

## Notes

- The release workflow currently does dry-run for crates.io publishing
- Uncomment the actual `cargo publish` line when ready to publish
- Binary stripping is configured for Linux and macOS builds to reduce size
- Cross-compilation is set up for ARM64 Linux targets
- SHA256 checksums are generated alongside release assets for supply-chain verification
