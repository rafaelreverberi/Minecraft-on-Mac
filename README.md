# Minecraft on Mac — managed installation preview

Use the packaged app: CrossOver installed and licensed, then **Sign In, Install & Play**. The launcher downloads the owned game and Microsoft components, prepares its own environment, and verifies the installation. Destination Macs need no developer tools. See [managed installation](docs/MANAGED-INSTALLATION.md) and [acceptance evidence](docs/ACCEPTANCE.md).

The app targets Apple Silicon and macOS 14+. It is a private ad-hoc signed preview. A full fresh Microsoft-account download, decryption, dedicated environment and live gameplay passed on the development Mac; an independent second-Mac run and public notarization remain unverified.

Managed installations use native Microsoft login, entitlement-first acquisition, persistent local decryption, pinned official components, a dedicated CrossOver environment, and a native Keychain credential bridge. No reference game folder or prepared reference bottle is required on the destination Mac. Setup keeps the current version available until all new-file and ABI checks pass. Unsupported Store versions fail closed. Saved games live in the owned bottle and survive version removal or a managed reinstall.

External installations and APFS snapshots remain available for migration and rollback. Their existing accounts and files are preserved. The native account only switches authentication for newly managed installations. Full account/bottle/save uninstall and Bedrock support are not implemented.

Open `build/Minecraft on Mac.app`. Install and activate CrossOver if necessary, then click **Sign In, Install & Play**. Updates are checked through the real Store service. Use Versions for rollback/removal and Settings for interrupted staging cleanup. Only game build 1.1.1.0 currently has a pinned compatibility profile. Unknown newer builds need a tested launcher release before installation.

## Build and test (developers only)

```sh
python3 scripts/package_app.py
swift test
cargo test --locked --release --manifest-path NativeHelper/Cargo.toml
MINECRAFT_LIVE_REFERENCE_TEST=1 swift test --filter testLiveReferenceWhenExplicitlyEnabled
```

Release packaging requires Xcode/Swift, Rust and an x86_64 MinGW compiler. These tools are **not required to run the packaged app**. Packaging builds the native helper and synthetic Windows probes, copies SwiftPM resources into the app, pins helper/probe hashes and ad-hoc signs the private preview. It includes no game data, Microsoft DLLs or CrossOver bottles. Developer ID signing/notarization are not configured; do not publish this preview as a signed production release.

`--smoke-launch-current` is an explicit integration-test argument on the packaged app: it invokes the same model Play action after loading Current. It does not establish visual menu/gameplay success.

See [audit](docs/AUDIT.md), [architecture](ARCHITECTURE.md), [security](SECURITY.md), [acceptance evidence](docs/ACCEPTANCE.md) and [remaining phases](docs/ROADMAP.md).
