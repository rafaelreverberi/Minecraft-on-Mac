# Minecraft on Mac — native development preview

A SwiftUI macOS launcher with an isolated Rust Microsoft package-account helper. Targets Apple Silicon and macOS 14+. This is a working development milestone, **not a production release or complete fresh-install launcher**.

## What works

- Native game library with Dungeons II and a clearly unavailable Bedrock placeholder.
- Discovery/registration of the gold-reference local Dungeons installation. Arbitrary user-selected paths and prepared CrossOver bottles are supported when they match the pinned profile.
- Direct hosted CrossOver launch of `Dungeons/Binaries/WinGDK/Dungeons-WinGDK-Shipping.exe`; reference DLL/debug/workdir flags preserved. Raw game output is discarded.
- Hash/PE/bottle validation, the five preserved synthetic production-writing ABI tests under Wine, all 61 XCurl names/ordinals, empty/nonempty encoding negotiation, and seven runtime loader exports.
- Durable library, exclusive writer lock, separate APFS snapshots with full file hash inventories, atomic current-version selection, manual rollback, interrupted-staging cleanup, compatibility-only repair of managed snapshots from a separate verified local source.
- Manifest-based managed-version removal; external registration removal leaves its game files intact. Saves, accounts and user bottles are preserved.
- Native Microsoft sign-in/sign-out and content-license/package-version check using Xodus's real service flow. Credentials use a dedicated native Keychain namespace. Failed checks never imply ownership. Interactive/service validation requires the user to sign in.
- Fixed-schema sanitized diagnostics, on-demand dependency discovery, optional developer details, signature verification API for future compatibility catalogs.

## Current boundaries

The existing Windows runtime was compiled with Xodus's `key-chain-file` feature. The launcher does not read, copy, migrate or remove its credential store. Reference Play is a development migration mode preserving that existing account flow. Public release needs a memory-only Windows credential backend with a native Keychain broker. Native package-account sign-in is **separate from the game's existing sign-in** until that migration is validated.

Fresh downloads, extraction, updates from the Store, managed bottle provisioning/reset and full launcher-data removal are not exposed as supported operations. The audited Xodus streaming CLI needs an entitlement-first structured interface, verified extraction receipts and stronger failure/path/integrity handling. Its current exit status cannot prove successful extraction. No game is downloaded just to display an Install button.

Only build 1.1.1.0 and its exact hashes are supported by the local profile. Managed snapshots currently copy an already installed, legitimately obtained local game; they do not establish ownership. Unknown builds fail closed. Gameplay/save/multiplayer and a new native Microsoft account require separate live acceptance checks.

## Run the built app

Open `build/Minecraft on Mac.app`. CrossOver must already be installed and the selected bottle must contain the reference runtime. The first-run migration candidate is the legacy installation under your home `Games/MinecraftDungeons2-FullyDecrypted`; if absent, choose a folder in the app. This is a migration discovery rule, not a fixed user path.

Use Versions to create an APFS snapshot, select it, repair it, or remove it. Select a different version or explicitly clear Current before removing the selected installation. An external reference offers Unregister instead of deleting files.

The package-account panel offers Sign In, Check Ownership / Updates and Sign Out. To switch that account, sign out and sign in again. This does not switch the reference game's account.

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
