<p align="center">
  <img src="./assets/pathway-logo.svg" alt="Pathway Logo" width="200">
</p>

<h1 align="center">Pathway</h1>

<p align="center">
  <strong>🚀 Smart URL router for developers</strong>
</p>

<p align="center">
  Launch URLs in the right browser with the right profile, every time
</p>

<p align="center">
  <a href="https://github.com/Guria/pathway/actions/workflows/ci.yml">
    <img src="https://github.com/Guria/pathway/workflows/CI/badge.svg" alt="CI Status">
  </a>
  <a href="https://github.com/Guria/pathway/actions/workflows/quality.yml">
    <img src="https://github.com/Guria/pathway/workflows/Quality/badge.svg" alt="Code Quality">
  </a>
  <img src="https://img.shields.io/badge/rust-1.89+-orange.svg" alt="Rust Version">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg" alt="Platform Support">
</p>

---

## ✨ Features

**🎯 Smart Browser Selection**
- Automatically detect available browsers on your system
- Allow define browser command manually (Coming Soon)
- Route URLs to specific browsers with custom profiles (Coming Soon)
- Support for Chrome, Firefox, Safari, and more
- Zero-config for basic usage with explicit arguments

**👤 Profile Management**
- Launch with named profiles (`--profile "Work"`)
- Create temporary isolated profiles
- Use custom user data directories
- Guest mode and incognito support

**🔒 Secure & Reliable**
- URL validation with scheme restrictions
- Path traversal protection
- Safe fallbacks on errors

## 🚀 Quick Start

### Basic Usage
```bash
# Launch URL with system default browser
pathway launch https://example.com

# Use specific browser
pathway launch --browser chrome https://github.com

# Launch with a specific profile
pathway launch --browser chrome --profile "Work" https://slack.com
```

### Advanced Examples
```bash
# Incognito mode for sensitive browsing
pathway launch --browser chrome --incognito https://banking.example.com

# Temporary profile for testing
pathway launch --browser firefox --temp-profile https://localhost:3000

# Multiple URLs at once
pathway launch --browser chrome https://github.com https://stackoverflow.com
```

## 📦 Installation

### From Source
```bash
git clone https://github.com/Guria/pathway.git
cd pathway/core
cargo build --release
./target/release/pathway --help
```

### System Install
```bash
cd pathway/core
cargo install --path .
pathway --version
```

## 📖 Usage

### Browser Management
```bash
# List available browsers
pathway browser list

# Check if a browser is available
pathway browser check chrome

# Get detailed browser information
pathway browser list --format json
```

### Profile Management
```bash
# List profiles for a browser
pathway profile --browser chrome list

# Launch with named profile
pathway launch --browser chrome --profile "Development" https://localhost:3000

# Create temporary profile
pathway launch --browser chrome --temp-profile https://example.com

# Use custom user directory
pathway launch --browser firefox --user-dir ~/my-custom-profile https://example.com
```

### Window Options
```bash
# Open in new window
pathway launch --browser chrome --new-window https://example.com

# Incognito/private mode
pathway launch --browser chrome --incognito https://example.com

# Kiosk mode (fullscreen)
pathway launch --browser chrome --kiosk https://dashboard.example.com
```

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

## 🛠️ Development

### Building from Source
```bash
git clone https://github.com/Guria/pathway.git
cd pathway/core
cargo build --release
```

### Running Tests
```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run with verbose output
cargo test -- --nocapture
```

### Code Quality
```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check formatting
cargo fmt --check
```

## 🤝 Contributing

We welcome contributions! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Setup
1. Install Rust 1.89+
2. Clone the repository
3. Run `cargo test` to ensure everything works
4. Make your changes
5. Run `cargo fmt` and `cargo clippy`
6. Submit a pull request

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

- Built with [Rust](https://rustlang.org/) for performance and safety
- Built in collaboration with wide set of LLM models, agents and AI editors, including:
  - OpenAI ChatGPT (GPT-5 Naming and logo)
  - Anthropic Claude (Opus 4.1 to outline CLI design and implementation milestones)
  - Open Codex agent (extension and reviewer)
  - GitHub Copilot Agent (extension and reviewer)
  - CoderabbitAI (Best in class reviewer)
  - Claude Code CLI
  - `opencode` CLI
  - Cursor
  - VS Code

---

<p align="center">
  <strong>⭐ Star this project if you find it useful!</strong>
</p>
