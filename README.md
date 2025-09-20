# Pathway

<div align="center">
  <img src="./assets/pathway-logo.png" alt="Pathway Logo" width="180">
  
  **🚀 Smart URL router for developers**
  
  *Launch URLs in the right browser with the right profile, every time*
</div>

<div align="center">
  
[![CI Status](https://github.com/guria/pathway/actions/workflows/ci.yml/badge.svg)](https://github.com/guria/pathway/actions/workflows/ci.yml)
[![Code Quality](https://github.com/guria/pathway/actions/workflows/quality.yml/badge.svg)](https://github.com/guria/pathway/actions/workflows/quality.yml)
![Rust 1.82+](https://img.shields.io/badge/rust-1.82+-orange.svg)
![Platform Support](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)

</div>

---

## ✨ Features

### 🎯 Smart Browser Selection
- Auto-detect available browsers on your system
- Support for Chrome, Firefox, Safari, and more
- Zero-config for basic usage
- Custom browser command support *(Coming Soon)*
- Smart URL routing with profiles *(Coming Soon)*

### 👤 Profile Management
- Named profiles (`--profile "Work"`)
- Temporary isolated profiles
- Custom user data directories
- Incognito and guest modes

### 🔒 Security & Reliability
- URL validation with scheme restrictions
- Path traversal protection
- Safe error handling and fallbacks

## 🚀 Quick Start

```bash
# Basic usage - system default browser
pathway launch https://example.com

# Specific browser
pathway launch --browser chrome https://github.com

# With profile
pathway launch --browser chrome --profile "Work" https://slack.com

# Incognito mode
pathway launch --browser chrome --incognito https://banking.example.com

# Multiple URLs
pathway launch --browser chrome https://github.com https://stackoverflow.com
```

## 📦 Installation

### macOS App Bundle (Native)
Download the pre-built macOS app bundle or build from source:

```bash
# Build macOS app bundle (requires Xcode command line tools)
./packaging/macos/build_app.sh

# Install the generated .pkg file or drag Pathway.app to Applications
```

The macOS app bundle provides:
- Native macOS integration with URL scheme handling
- Ability to set Pathway as default browser
- Proper app bundle packaging with code signing support
- Both manual (.zip) and installer (.pkg) distribution formats

### Command Line (Cross-platform)
```bash
# Clone and build
git clone https://github.com/guria/pathway.git
cd pathway/core
cargo build --release

# Install system-wide
cargo install --path .
pathway --version
```

## 📖 Usage

<details>
<summary><strong>Browser Management</strong></summary>

```bash
# List available browsers
pathway browser list

# Check browser availability
pathway browser check chrome

# JSON output for scripting
pathway browser list --format json
```
</details>

<details>
<summary><strong>Profile & Window Options</strong></summary>

```bash
# Named profiles
pathway launch --browser chrome --profile "Development" https://localhost:3000

# Temporary profile
pathway launch --browser chrome --temp-profile https://example.com

# Custom user directory
pathway launch --browser firefox --user-dir ~/my-profile https://example.com

# Window options
pathway launch --browser chrome --new-window https://example.com
pathway launch --browser chrome --incognito https://example.com
pathway launch --browser chrome --kiosk https://dashboard.example.com
```
</details>

## 🔧 Configuration

### JSON Output
All commands support `--format json` for programmatic integration:

```json
{
  "action": "launch",
  "status": "success", 
  "urls": ["https://example.com/"],
  "browser": {
    "name": "chrome",
    "channel": "stable",
    "path": "/Applications/Google Chrome.app"
  },
  "profile": {
    "type": "named",
    "name": "Work"
  },
  "window_options": {
    "new_window": true,
    "incognito": false,
    "kiosk": false
  }
}
```

## 📋 Current Milestone: macOS Native App Bundle

**Status:** ✅ **Completed** - Full macOS app bundle support with native integration

### What's New
- **Complete macOS App Bundle**: Native `.app` bundle with proper Info.plist configuration
- **Swift URL Handler Shim**: PathwayShim.swift provides seamless URL scheme handling
- **Installer Package Creation**: Automated `.pkg` installer generation for easy distribution
- **Default Browser Integration**: Can be set as system default browser with infinite loop protection
- **Enhanced Build Pipeline**: Automated CI/CD workflow for macOS app builds
- **Robust Error Handling**: Improved logging and error management throughout the system

### Technical Implementation
- Added `packaging/macos/` directory with complete build infrastructure
- Swift shim application handles URL events and forwards to Rust CLI
- Automated build script creates both ZIP archives and PKG installers
- Enhanced conflict resolution for system default browser scenarios
- OSLog-compatible logging for macOS 10.15+ compatibility

### Files Added/Modified
- `packaging/macos/build_app.sh` - Complete app bundle build automation
- `packaging/macos/PathwayShim.swift` - Swift URL handling application
- `packaging/macos/Info.plist` - macOS app bundle metadata
- `packaging/macos/PathwayShim.entitlements` - Code signing entitlements
- Enhanced CI workflows for macOS builds

---

## 🛠️ Development

```bash
# Setup
git clone https://github.com/guria/pathway.git
cd pathway/core

# Build and test
cargo build --release
cargo test

# Code quality
cargo fmt
cargo clippy -- -D warnings
```

## 🤝 Contributing

Contributions welcome! For major changes, please open an issue first.

**Quick setup:**
1. Install Rust 1.82+
2. Fork and clone the repository  
3. Run `cargo test` to verify setup
4. Make changes, run `cargo fmt` and `cargo clippy`
5. Submit a pull request

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## 🙏 Acknowledgments

Built with [Rust](https://rustlang.org/) for performance and safety, developed in collaboration with various AI tools and agents including GitHub Copilot, Claude, and other modern development assistants.

---

<p align="center">
  <strong>⭐ Star this project if you find it useful!</strong>
</p>
