# Minecraft on Mac

An unofficial native macOS launcher for **Minecraft Dungeons II** and **Minecraft: Bedrock for Windows**, using your legitimately owned Microsoft Store games and separately installed CrossOver.

> [!WARNING]
> **Unofficial project. Use at your own risk.**
>
> - Not affiliated with, endorsed by or approved by Microsoft, Mojang Studios, Xbox or CodeWeavers. "Minecraft", "Xbox" and "Microsoft" are trademarks of the Microsoft group of companies. "CrossOver" is a trademark of CodeWeavers. All other trademarks belong to their owners.
> - You must own the game on your Microsoft account. This launcher includes no game files, licences or paid content and unlocks nothing; it only uses the copy you already own.
> - CrossOver is a separate paid product from CodeWeavers and is not included.
> - Third-party launchers may go against the terms of the game or platform. Microsoft can change or block how this works at any time, and that could affect your account.
> - Provided "as is", without warranty of any kind, under the GPL-3.0 licence in [LICENSE](LICENSE). The authors and distributors are not liable for any damage, data loss, account action or other problem resulting from its use.
> - Back up your worlds before installing or switching game versions.

**[Download the latest release](../../releases/latest)** · [Quick start](docs/INSTALL.md) · [Changelog](CHANGELOG.md) · [Installation details](docs/MANAGED-INSTALLATION.md) · [Bedrock details](docs/BEDROCK.md) · [Test evidence](docs/ACCEPTANCE.md)

## Install and play

1. Install, open and license [CrossOver](https://www.codeweavers.com/crossover) in `/Applications` or `~/Applications`.
2. Download the Apple silicon DMG from the release, open it and drag **Minecraft on Mac.app** to Applications. An app ZIP is also available.
3. Open the app, choose **Dungeons II** or **Bedrock Edition**, then **Install & Play**. Enter passwords and verification codes only in Microsoft's sign-in window.
4. The launcher checks your entitlement, downloads the official game and Microsoft components, decrypts the game to persistent SSD files, prepares a dedicated CrossOver environment and verifies the installation. Later starts use **Play**.

You do not need an existing game folder or bottle, Homebrew, Rust, Python or Xcode to use the packaged app. Game data, Microsoft runtime downloads, licenses and credentials are not included in release assets.

The app is an **ad-hoc signed, unnotarized preview**. macOS may require explicit approval in Privacy & Security on first launch. Developer ID signing and notarization are not claimed.

## Supported setup

| Game | Latest tested Windows build | CrossOver |
|---|---|---|
| Minecraft Dungeons II | 1.1.2.0, pinned package revision | Separately installed and licensed CrossOver; see recorded live acceptance |
| Minecraft: Bedrock | 1.26.5203.0, reviewed package pin | 26.3.x, D3DMetal |

Apple silicon, macOS 14+, an entitled Microsoft account, an internet connection and at least 35 GB free during installation are required. Intel and Bedrock Preview/Beta are not supported. Newer game builds can be explicitly installed as untested trials; gameplay compatibility is not guaranteed. Each game has its own selected version, environment and saves.

CrossOver detection currently looks for **CrossOver.app**. A separately named CrossOver Beta.app is not detected, and Bedrock rejects beta-suffixed version strings. General CrossOver Beta support is not claimed.

Dungeons II 1.1.2.0 was fully downloaded and verified, installed through the launcher, and successfully gameplay-tested by the user. Full fresh managed downloads and owned environments were verified on the development Mac for both supported games; Bedrock's menu and actual profile were confirmed by the user after its runtime/package fixes. Independent second-Mac acceptance, Bedrock world save/reopen, controller and multiplayer remain separate unverified boundaries. See [Bedrock acceptance](docs/BEDROCK.md) and the [design/setup review](docs/DESIGN-REVIEW.md).

## A simpler launcher

- Official bundled game banners and an adapted Minecraft Launcher icon with a blue desktop badge. Artwork renders offline.
- **Play** starts the selected installed copy. **Select version** opens the Versions panel, where you choose which copy to use.
- **Versions & Downloads** opens its own page for both games. **Installed** lists copies, **Select for Play** chooses one, and the Play button shows exactly which version will start.
- A new launcher offers the latest tested release when an older version is selected. Choose **Download & Select**, reuse an already installed copy, or **Later**. Downloads select the new copy only after successful verification.
- **Install a Version** groups the recommended **Latest tested version** download and Microsoft's latest package. Newer releases are marked **Not tested · may work**. Downloads do not start the game or change the current Play selection; after installation the page returns to **Installed**.
- **Advanced** is a separate maintenance page for verification, repair, folders, account checks and import options. Versions and Advanced cannot be open together. Diagnostics are under Settings → Advanced.
- A small **red X** appears next to Play only for the game identified as running from a registered installation. It force-quits that game's process and its matching CrossOver descendants after rechecking PID, owner, executable and kernel start time. Unsaved progress can be lost. Other games, unrelated processes and the shared Wine server are excluded; this is not a global CrossOver shutdown.

Authentication retains the native Microsoft account flow. The native account service/broker is cleaned up through the existing launch lifecycle when a launcher-owned session exits.

## Versions, storage and saves

| Data | Location |
|---|---|
| Managed versions, component cache and receipts | `~/Library/Application Support/Minecraft on Mac/Games/<game>/Versions/<version>-<UUID>/` |
| Persistent game files | `Game/` inside the managed version |
| Library, diagnostics, temporary downloads | `~/Library/Application Support/Minecraft on Mac/` |
| Owned game environments and saves | `~/Library/Application Support/CrossOver/Bottles/MinecraftMac-<UUID>/` or `MinecraftMac-bedrock-<UUID>/` |
| Native Microsoft credentials | macOS Keychain, service `MinecraftMacLauncher Microsoft` |

**Downloaded installation** is an official managed package. **Existing local installation** is a registered original folder. **Local snapshot** is an APFS copy. **Selected for Play** identifies the copy used by Play, not a newer game release. The version row's menu → Show in Finder reveals the files. Displayed sizes are logical file bytes; APFS snapshots can share blocks and the environment is separate.

Quit the game and use **Versions → Manage → Delete Version**. Confirmation removes the complete managed version folder, component cache and metadata. Saved games and the owned environment are kept by default and reused on the next installation. The deletion dialog can also remove the owned environment and all saves, only when no other version uses it. Retained Dungeons environments are in Settings → Storage; Bedrock environments/world cleanup are in Bedrock → Advanced.

**Unregister** removes only an external installation's library entry. Original files and user-owned environments are never deleted. Dungeons external registration/bottle changes are under Advanced → Import & selection options; the existing external-import operation does not support Bedrock. Deletion is blocked while a game runs. Back up Bedrock worlds before changing versions: selecting an older executable does not restore world data.

## Privacy

Microsoft authentication uses the native macOS Keychain bridge and private account service. No plaintext credential cache or raw authentication/game-log upload is implemented. Game data and credentials stay local; the launcher talks to Microsoft for sign-in, ownership and licensed content downloads. See [security boundaries](SECURITY.md) and [component provenance/licenses](docs/VENDOR.md).

## Stack and development

The application uses **Swift 6, SwiftUI and AppKit** on macOS. LauncherCore manages local version storage, verification, guarded process control and CrossOver integration. The bundled Rust native helper handles Microsoft sign-in, entitlement/package acquisition and persistent materialization. The Bedrock WineGDK compatibility libraries and Dungeons compatibility runtime/probes are built and hash-checked during packaging. No Electron or web runtime is required to use the launcher.

Builders need an Apple silicon host, Xcode/Swift 6, Python 3, Rust/rustup, protobuf `protoc`, an x86_64 MinGW compiler and official GDK 2504.6 XCurl headers. Install the Windows Rust target with `rustup target add x86_64-pc-windows-gnu`. Set `MML_XCURL_INCLUDE` to the directory containing `XCurl.h`; Microsoft SDK headers/binaries are not vendored.

```sh
swift test
cargo test --locked --manifest-path NativeHelper/Cargo.toml
cargo test --locked --manifest-path Shared/CredentialBridge/Cargo.toml
MML_XCURL_INCLUDE=/path/to/official/XCurl/Include python3 scripts/package_app.py
```

For Swift-only work, `python3 scripts/package_app.py --reuse-compatibility` reuses existing runtime/probe artifacts **only after verifying their recorded digests**. Packaging builds the native helper and Swift executable, restores and checks Bedrock resource bytes, bundles artwork/notices and verifies the ad-hoc signature. Add `--reuse-native-helper /path/to/verified/minecraft-native-helper` to retain the existing signed account helper after checking its recorded digest and signature. It does not install the app or launch a game.

After committing the release source and building the app with its matching version, `python3 scripts/package_release.py 0.4.0` creates the app ZIP, drag-to-Applications DMG, corresponding committed source ZIP and SHA256 manifest. It refuses dirty source trees, version mismatches and overwritten assets.

Offline Swift, native-helper/signature and credential-bridge checks run in GitHub CI. Force-quit tests use disposable signed fixture processes and verify that their children exit while an unrelated process survives. Live game/account checks are opt-in and distinct from fixture tests. See [architecture](ARCHITECTURE.md) and [roadmap](docs/ROADMAP.md).

GPL-3.0-only, with upstream notices retained. Releases include corresponding source. Official Mojang/Microsoft artwork and the adapted launcher icon are separate from the GPL code license; their provenance is bundled in the artwork resource directory. This project is not affiliated with or endorsed by Microsoft, Mojang or CodeWeavers. Minecraft and CrossOver trademarks belong to their respective owners.
