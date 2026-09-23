# Atlas

Atlas is a native Rust desktop launcher project for Linux and Windows. Its interface uses `egui` / `eframe`; it does not use a webview.

## Current foundation

- Native desktop window with Overview, Game Versions, and Performance pages.
- Fetches Mojang's official version manifest and caches it in the platform's local application data directory. The manifest is ordered newest first, and the cached list loads at startup. Releases, snapshots, and legacy entries are shown.
- Installs and caches vanilla Minecraft versions or Fabric profiles into isolated per-version Atlas instances. Install work runs in the background and reports progress in the app.
- Includes an optional Modrinth-backed library: Sodium, Lithium, FerriteCore, ImmediatelyFast, Entity Culling, Mod Menu, AppleSkin, BetterF3, Shulker Box Tooltip, and Zoomify. The default selection is a compact set of performance and everyday QoL mods. Required dependencies are installed automatically and file checksums are verified.
- Memory allocation, frame limit, VSync, background saver, and local profile name preferences save to the platform's configuration directory.
- The upstream `mc-launcher-core` 0.1.2 source is included under its MIT license. Atlas uses the standard library for CPU architecture and Linux kernel version detection so the installer builds with the Rust version available here.

## Run

Install Rust and the platform graphics development dependencies required by `eframe`, then run:

```sh
cargo run
```

## Install and update

- **Windows setup:** Download `atlas-client-windows-x86_64-setup.exe`. The dark Atlas setup wizard installs per-user, adds an Atlas Start Menu shortcut, and offers an optional desktop shortcut. Uninstall Atlas from Windows Settings.
- **Windows portable:** Download and extract `atlas-client-windows-x86_64.zip`, then run `atlas-client.exe` from that folder.
- **Linux online installer:** Download `atlas-client-linux-install.sh` and run `sh atlas-client-linux-install.sh`. It fetches the latest Linux package from GitHub, verifies its SHA-256 checksum, and adds Atlas to your applications menu. `curl`, Python 3, and `sha256sum` are required.
- **Linux portable:** Download and extract `atlas-client-linux-x86_64.tar.gz`, then run `./atlas-client` or `./install.sh` to add it to your applications menu.

Atlas checks GitHub Releases when it starts. When a newer release is available, the Overview page shows its version and release notes with an **Update Atlas** button. The app verifies the downloaded package before handing off to the Windows setup wizard or replacing and restarting the Linux executable. Portable Windows copies update in place when their folder is writable.

Releases are built for x86-64 Linux and Windows. Pushing a version tag such as `v0.1.0` builds separate setup and portable packages and publishes SHA-256 checksums alongside them.

Version installation, Fabric profile setup, mod selection, and mod download are wired up. Authenticated launch and Java runtime selection are the next launcher milestones. Atlas will require Microsoft sign-in before it can launch, and its registered client ID is awaiting approval for Minecraft Services.
