# Pathway macOS Bundle

This directory contains the resources required to build the `Pathway.app` bundle and installer package.

## Contents

- `build_app.sh` – builds the app bundle and creates distributable archives (.zip, .pkg)
- `Info.plist` – app bundle metadata template
- `PathwayShim.swift` – Swift app that forwards URL events to the Rust CLI binary
- `PathwayShim.entitlements` – code signing entitlements

## Building

Run the build script on macOS with Xcode command line tools:

```bash
./packaging/macos/build_app.sh
```

Output files in `packaging/macos/dist/`:
- `Pathway-{version}.zip` – App bundle archive (manual installation)
- `Pathway-{version}.pkg` – macOS installer package (recommended)

## Installation

### PKG Installer (Recommended)
1. Double-click the `.pkg` file
2. Follow the installer prompts (admin privileges required)

### ZIP Archive (Manual)
1. Extract the ZIP file
2. Drag `Pathway.app` to the Applications folder

## Setting as Default Browser

After installation:
1. Open **System Preferences** → **General** → **Default web browser**
2. Select **Pathway Browser Router**

The app includes infinite loop protection when set as the default browser.
