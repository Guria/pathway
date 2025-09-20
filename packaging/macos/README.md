# Pathway macOS Bundle

This directory contains the resources required to build the `Pathway.app` bundle.

## Contents

- `build_app.sh` &ndash; orchestrates compiling the universal Rust binary and Swift shim, assembles the bundle, signs it, and produces a distributable archive.
- `Info.plist` &ndash; template used to generate the bundle metadata.
- `PathwayShim.swift` &ndash; background-only Cocoa application that forwards URL events to the bundled `pathway` binary.
- `PathwayShim.entitlements` &ndash; entitlements used for ad-hoc code signing.

## Building locally

Run the build script on a macOS host with the Xcode command line tools installed:

```bash
./packaging/macos/build_app.sh
```

The resulting bundle is written to `packaging/macos/build/Pathway.app` and a ready-to-ship archive is created in `packaging/macos/dist/`.
