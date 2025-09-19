# Pathway Browser Router - Implementation Plan

## Overview

**Purpose**: A lightweight URL routing agent that opens URLs in the appropriate browser/profile based on configurable rules.

**Core Principle**: Simple, predictable, and fast. Portable dotfiles-like configuration.

## Current Status

✅ **Milestone 1 Complete**: Core CLI with URL validation
- Basic Rust CLI that validates and logs URLs
- URL validation with scheme restrictions
- Structured logging with `tracing`
- Comprehensive test suite

✅ **Milestone 2 Complete**: Browser discovery & launch
- Detects common browsers per-platform and reports system default
- `--browser`, `--channel`, and `--system-default` flags control routing
- `--list-browsers`, `--check-browser`, and `--no-launch` add diagnostics
- Launches URLs via platform-appropriate commands with verbose command logging

✅ **Milestone 3 Complete**: Browser profiles & advanced launch options
- Browser profile management (named, custom directories, temporary, guest)
- Window management options (new-window, incognito, kiosk)
- Browser-specific feature support and validation warnings
- Profile discovery from browser configuration files
- Enhanced JSON output with profile and window option details

## Features

### URL Launching
```bash
# Basic URL launching (system default browser)
pathway launch https://example.com
pathway launch file:///path/to/local.html

# Browser selection
pathway launch --browser chrome https://example.com
pathway launch --browser firefox --channel dev https://example.com

# Profile options
pathway launch --browser chrome --profile "Work" https://example.com
pathway launch --browser chrome --user-dir ~/dev-profile https://localhost:3000
pathway launch --browser chrome --temp-profile https://example.com
pathway launch --browser chrome --guest https://example.com

# Window management
pathway launch --browser chrome --new-window https://example.com
pathway launch --browser chrome --incognito https://example.com
pathway launch --browser chrome --kiosk https://presentation.com

# Combined options
pathway launch --browser chrome --profile "Work" --new-window --incognito https://example.com
pathway launch --browser chrome --temp-profile --new-window --kiosk https://example.com

# Validate without launching
pathway launch --no-launch https://example.com
```

### Browser Management
```bash
# List all detected browsers
pathway browser list

# Check if specific browser is available
pathway browser check chrome
pathway browser check firefox --channel dev

# JSON output for scripting
pathway browser list --format json
```

### Profile Management
```bash
# List profiles for a browser
pathway profile --browser chrome list
pathway profile --browser helium list

# Get profile details
pathway profile --browser chrome info "Work"
pathway profile --browser helium info "Personal"

# List profiles from custom directory
pathway profile --browser chrome --user-dir ~/custom-profiles list

# JSON output for scripting
pathway profile --browser chrome --format json list
```

## Browser Support

### Chromium-Based Browsers
**Supported**: Chrome, Edge, Brave, Vivaldi, Arc, Helium, Opera, Chromium

**Features**:
- ✅ Named profiles (`--profile-directory`)
- ✅ Custom directories (`--user-data-dir`) 
- ✅ Temporary profiles
- ✅ Guest mode (`--guest`)
- ✅ Incognito mode (`--incognito`)
- ✅ Window management (`--new-window`, `--kiosk`)

### Firefox/Waterfox
**Features**:
- ✅ Named profiles (`-P "ProfileName"`)
- ✅ Custom directories (`--profile /path`)
- ✅ Temporary profiles
- ✅ Private windows (`--private-window`)
- ✅ Window management (`--new-window`, `--kiosk`)
- ⚠️ No guest mode (warns to use `--incognito`)

### Safari
**Features**:
- ✅ Basic URL launching (via `open -b com.apple.Safari`)
- ✅ New window support (`--new`)
- ⚠️ No profile support (system limitation)
- ⚠️ No command-line incognito (requires manual activation)
- ⚠️ No kiosk mode support

### Other Browsers
**Features**:
- ✅ Basic URL launching
- ⚠️ Limited profile/window option support (warnings shown)

## Validation & Warnings

The system provides intelligent warnings for unsupported combinations:

```bash
# Safari limitations
$ pathway launch --browser safari --temp-profile https://example.com
WARN: Safari does not support temporary profiles

# Firefox limitations  
$ pathway launch --browser firefox --guest https://example.com
WARN: Firefox does not support guest mode (use --incognito for private browsing)

# System default limitations
$ pathway launch --temp-profile https://example.com
WARN: Profile options require specifying a browser with --browser

# Conflict resolution
$ pathway launch --browser chrome --profile "Work" --incognito https://example.com
WARN: Incognito mode ignores profile selection
```

## JSON Output

All commands support `--format json` for programmatic use:

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
  },
  "warnings": ["..."]
}
```

## Installation

### Building from Source
```bash
git clone https://github.com/guria/pathway.git
cd pathway/core
cargo build --release
./target/release/pathway --help
```

### Usage
```bash
# Show help
pathway --help

# Show subcommand help
pathway launch --help
pathway browser --help
pathway profile --help

# List detected browsers
pathway browser list

# Validate URLs without launching
pathway launch --no-launch https://example.com

# Launch with verbose logging
pathway launch --verbose https://example.com
```

## Profile Management Deep Dive

### Profile Discovery
Pathway automatically discovers browser profiles from standard locations:

**Chrome/Chromium browsers**:
- macOS: `~/Library/Application Support/[Browser]/`
- Linux: `~/.config/[browser]/`
- Windows: `%APPDATA%/[Browser]/`
- Profiles parsed from `Local State` JSON

**Firefox**:
- Profiles discovered from `profiles.ini`
- Cross-platform profile location detection

### Profile Resolution
When you specify a profile by display name, Pathway resolves it to the actual directory:

```bash
# Shows mapping between display names and directories
$ pathway profile --browser helium list
Helium profiles:
  Personal [Default] (default)    # Display name "Personal" → Directory "Default"
  Work [Profile 1]                # Display name "Work" → Directory "Profile 1"
  Gaming [Profile 2]              # Display name "Gaming" → Directory "Profile 2"
```

### Custom Directory Support
Use `--user-dir` for isolated browser sessions:

```bash
# Create isolated development environment
pathway launch --browser chrome --user-dir ~/project-a-browser https://localhost:3000

# List profiles in custom directory
pathway profile --browser chrome --user-dir ~/custom-profiles list
```

### Temporary Profiles
Perfect for testing and privacy:

```bash
# Creates unique temporary directory
$ pathway launch --browser chrome --temp-profile https://example.com
INFO: Created temporary profile directory: /tmp/pathway_profile_abc123
INFO: Launching in Google Chrome with temporary profile (/tmp/pathway_profile_abc123)
```

## Technical Details

### Conflict Resolution
Pathway handles conflicting options with clear precedence rules:

1. `--incognito` overrides profile selection (with warning)
2. `--user-dir` overrides `--profile`
3. `--temp-profile` overrides both `--profile` and `--user-dir`
4. `--guest` mode ignores profile settings

### Security
- Path validation prevents directory traversal attacks
- Permission checks before directory creation
- Profile isolation maintained
- Temporary profiles auto-cleanup
- Safe fallbacks on errors

### Performance
- Profile detection: < 50ms
- Directory creation: < 100ms  
- Launch overhead: < 100ms
- Scales to hundreds of profiles

## Development

### Running Tests
```bash
cd core
cargo test
```

### Adding New Browser Support
1. Add browser to `BrowserKind` enum
2. Update platform detection in `browser/[platform].rs`
3. Add profile discovery logic if needed
4. Update validation warnings

## Future Roadmap

- **Milestone 4**: URL routing rules and domain-based browser selection
- **Milestone 5**: Configuration file support and rule persistence
- **Milestone 6**: Advanced routing with regex patterns and conditions

## Contributing

This project follows standard Rust development practices:
- `cargo fmt` for formatting
- `cargo clippy` for linting  
- `cargo test` for testing
- Comprehensive integration tests required for new features

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

