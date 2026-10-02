# Minecraft on Mac

An unofficial native macOS launcher for the supported, legitimately owned Minecraft Dungeons II Windows Store build. Sign in with your own Microsoft account; the launcher checks entitlement, downloads the game and Microsoft components, prepares a dedicated CrossOver environment, and starts the game.

**[Download v0.2.1](https://github.com/rafaelreverberi/Minecraft-on-Mac/releases/tag/v0.2.1)** · [Installation details](docs/MANAGED-INSTALLATION.md) · [Test evidence](docs/ACCEPTANCE.md)

## Requirements and setup

- Apple Silicon Mac, macOS 14 or newer.
- [CrossOver](https://www.codeweavers.com/crossover), separately installed in Applications or ~/Applications, opened and licensed.
- A Microsoft account with the required game entitlement; an internet connection.
- At least 35 GB available during preparation. The installed version occupies roughly 9.6 GB, plus its separate environment (about 0.37 GB in the tested installation). Sizes vary with saves and prerequisites.

1. Download the app ZIP from the release, extract it and move Minecraft on Mac.app to Applications.
2. Open the app. This is an **ad-hoc signed, unnotarized preview**, so macOS may require explicit approval in Privacy & Security before the first launch. No Developer ID signing is claimed.
3. Choose **Sign In, Install & Play**. Enter passwords and verification codes only in Microsoft's sign-in window.
4. Wait for the download, decryption, environment preparation and checks. Subsequent starts use **Start Game**.

You do not need a prepared Minecraft folder, an existing game bottle, Homebrew, Rust, Python or Xcode. Game data, Microsoft DLLs, licenses and credentials are not included in release assets. Only build 1.1.1.0 and its pinned package revision are supported; unsupported versions are rejected rather than patched blindly. Bedrock and Intel are not supported.

## Where files are stored

| Data | Location |
|---|---|
| Managed game versions, their component cache and receipts | `~/Library/Application Support/Minecraft on Mac/Games/dungeons2/Versions/<version>-<UUID>/` |
| Decrypted game inside a version | `Game/` |
| Library, diagnostics, temporary downloads | `~/Library/Application Support/Minecraft on Mac/` |
| Owned CrossOver game environment | `~/Library/Application Support/CrossOver/Bottles/MinecraftMac-<UUID>/` |
| Account-specific local saves | Inside that environment: `drive_c/users/<user>/AppData/Local/Minecraft on Mac/Dungeons2/<authenticated XUID>/savedata/` |
| Native Microsoft credentials | macOS Keychain, service `MinecraftMacLauncher Microsoft` |

Versions shows the actual path and **Show in Finder**. Sizes describe logical file bytes, including the version's component cache; APFS snapshots may share blocks, so deleting a snapshot can reclaim less than the displayed size. The environment is separate from that size.

## Versions and deletion

- **Downloaded installation:** an official package downloaded and managed by this launcher.
- **Existing local installation:** your original folder registered for launch; it is not a second download or part of the app bundle.
- **Local snapshot:** a separate APFS copy made with Create Snapshot.
- **Current:** the copy selected for Start Game. It does not mean a newer game build; multiple copies can have the same version number.

Quit the game, open **Versions**, and choose **Delete Version**. Confirming removes the complete managed version folder, including game files, downloaded component cache and metadata. You can delete Current directly; its selection is cleared. Interrupted removal is recovered or exposed as receipt-backed temporary cleanup in Settings.

Saved games and the owned environment are **kept by default**, recorded under **Settings → Storage**, and reused on the next installation. To uninstall everything belonging to that game environment, enable **Also delete this game's environment and all its saves** in the deletion dialog. This is permanent and is allowed only when no other registered version uses the environment. A retained environment can also be deleted later in Settings. Microsoft Keychain login remains separate; sign out through Account if desired.

**Unregister** on an existing local installation removes only its library entry. Original files and user-owned environments are never deleted. Deletion is blocked while the game is running. Older snapshots containing in-folder saves must have those saves preserved/migrated first.

## Privacy and verification

Managed Windows authentication uses an encrypted ephemeral native Keychain bridge; it creates no plaintext credential cache. No telemetry or raw game/authentication log uploading is implemented. The original installation/account is not imported or modified. See [security boundaries](SECURITY.md) and [component provenance/licenses](docs/VENDOR.md).

A full fresh download, persistent decryption, dedicated environment and real-account gameplay passed on the development Mac and were confirmed by the user. v0.2.1 deletion tests use disposable fixtures, including shared/foreign environments, host symlinks and interrupted removal. Independent second-Mac acceptance, a second real Store version, save/restart acceptance and public notarization remain unverified.

## Build and test

End users should download the release. Builders need an Apple Silicon build host, Xcode/Swift 6, Python 3, Rust/rustup, protobuf `protoc`, an x86_64 MinGW compiler and the official GDK 2504.6 XCurl headers. Install the Windows Rust target with `rustup target add x86_64-pc-windows-gnu`. Set `MML_XCURL_INCLUDE` to the directory containing the official `XCurl.h`; no SDK headers or binaries are vendored.

```sh
swift test
cargo test --locked --manifest-path NativeHelper/Cargo.toml
cargo test --locked --manifest-path Shared/CredentialBridge/Cargo.toml
MML_XCURL_INCLUDE=/path/to/official/XCurl/Include python3 scripts/package_app.py
```

The packager builds the helper, Windows compatibility binaries and synthetic probes, checks resource hashes and ad-hoc signs the app. It does not install it or launch a game. GitHub CI runs offline Swift tests and bridge tests; actual CrossOver/account tests are opt-in and documented separately. See [architecture](ARCHITECTURE.md) and [roadmap](docs/ROADMAP.md).

GPL-3.0-only, with the MIT and other upstream notices retained. Release assets include corresponding source; dependencies are pinned by lockfiles. This is an unofficial project, not affiliated with or endorsed by Microsoft, Mojang or CodeWeavers. Minecraft and CrossOver trademarks belong to their respective owners.

## Minecraft: Bedrock

Bedrock is integrated into the same managed library, with its own current version,
owned environment and saves. The compatibility baseline is **1.26.5203.0**
(Bedrock 26.52), **CrossOver 26.3.x**, Apple silicon and D3DMetal. Older builds
are rejected; newer builds remain “Compatibility not verified yet”. Full licensed
packages are decrypted once into persistent SSD files during installation; normal
Play does not use Xodus streaming/RAM-disk materialization. No FPS improvement is
claimed and no experimental allocator patch is enabled.

See [Bedrock installation, storage, security and acceptance](docs/BEDROCK.md).
Live gameplay in this launcher and admission of the official Bedrock package digest
must be verified before calling this integration release-ready. The research
baseline is [Bedrock on macOS](https://github.com/Roritharr/bedrock-on-macos);
its manually installed files and account are never reused.
