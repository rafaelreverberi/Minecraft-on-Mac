# Launcher design and setup review — 2026-10-02

The game pages now present official bundled artwork, one Play / Install & Play action, and an installed-version picker. Versions are filtered per game, use the existing per-game selection, and retain the original snapshot/deletion confirmations. Verify, Repair, folders, account details, additional installations and Bedrock world/environment cleanup are under Advanced. The global Library navigation is removed; diagnostics are in Settings → Advanced. The official installed Minecraft Launcher icon, adapted with a blue desktop badge, is used for the application and sidebar identity. Sidebar image/text spacing is explicitly 12 points.

The redesign changes SwiftUI presentation, executable artwork resources and icon packaging. v0.3.0 additionally introduces explicit per-game force quit with kernel process identity checks and descendant filtering. Normal launch arguments, credentials, downloader, compatibility gates and runtime binaries are unchanged. External registration and bottle changes are shown only for Dungeons because the existing registration operation supports that game. The new version picker selects existing verified copies; it does not add support for arbitrary game builds.

## Zero setup verdict

- **Dungeons II Windows 1.1.1.0:** the packaged app contains its native helper, compatibility runtime and probes; setup downloads licensed game/runtime components and creates its owned CrossOver environment. No user installation of Rust, Homebrew, Python or Xcode is required. Earlier full fresh acceptance is recorded in MANAGED-INSTALLATION.md and ACCEPTANCE.md.
- **Bedrock for Windows 1.26.5203.0:** the same managed setup exists, including the reviewed package pin, persistent game extraction, native account service and bundled runtime. BEDROCK.md records a successful full fresh download/environment/menu/profile acceptance after its earlier blockers were fixed. Worlds/save reopen and multiplayer still have their separate live acceptance boundaries.
- **CrossOver Beta:** not a general compatibility promise. Detection currently searches CrossOver.app in /Applications and ~/Applications. A separately named CrossOver Beta.app is not detected. Bedrock additionally requires a numeric 26.3.x version; beta-suffixed versions are rejected.
- **Bedrock Preview/Beta:** not an implemented product channel. The existing registry contains the retail Windows Store product only, and compatibility accepts only the pinned baseline builds.
- Requires Apple silicon, macOS 14+, installed/licensed CrossOver, a Microsoft account with entitlement, network and enough space. The app remains ad-hoc signed and unnotarized, so first-run macOS approval is still a setup step.

These conclusions combine current source/bundle checks with the project's recorded earlier fresh-install evidence. This design change did **not** run a new full clean-account download or claim independent second-Mac/Beta acceptance. The user's Bedrock process was already running and was left undisturbed.

## Validation in this change

- Swift build and Release app packaging passed.
- 41 Swift tests: zero failures, two opt-in live tests skipped.
- Existing compatibility artifacts reused only after recorded hashes were checked. Delivered Bedrock provenance and strict ad-hoc signature verification passed.
- Native helper links only to macOS system libraries/frameworks, with no Homebrew runtime dependency.
- Both official art files are bundled once and their delivered bytes match source. Icon is declared in Info.plist and bundled as LauncherMac.icns.
- SwiftUI views were checked in a disposable preview with isolated storage, without invoking account/install/play/delete actions. Installed and empty-library screens, sidebar/game navigation and artwork rendering were visible. The sidebar width and responsive banner composition were corrected after inspection. The native UI automation intermittently returned “native pipe closed”; final Advanced/Versions click-through and an independent minimum-size interaction pass remain unverified.

Artwork provenance and copyright are in Sources/MinecraftMacLauncher/Resources/Artwork/provenance.json. Key art comes from the official Microsoft Store catalog for [Dungeons II](https://www.xbox.com/games/store/minecraft-dungeons-ii/9P5786PJB9RP) and [Minecraft for Windows](https://www.xbox.com/games/store/minecraft-for-windows/9NBLGGH2JHXJ). Mojang/Microsoft artwork is separate from this project's GPL source license. This is an unofficial launcher.

## v0.3.0 follow-up

The red X force-quits only a registered game executable matched to the detected CrossOver runtime, plus its same-user runtime descendants. PID, executable and kernel start seconds/microseconds are rechecked before each SIGKILL. Unrelated processes and wineserver are excluded. The action is intentionally immediate, with an unsaved-progress tooltip and accessible game-specific label. Tests cover reused PIDs, exact executable paths, cross-runtime exclusion, child selection and actual disposal of a signed synthetic game process tree while an unrelated sibling survives. Live force quit of a real game was not performed.

The app icon was edited with the built-in imagegen tool: preserve the official green Creeper icon, add a small blue desktop-monitor badge at bottom right, transparent background. The original asset and provenance are retained.

Final packaged v0.3.0 was opened against the existing library after the game had exited. Correct sidebar spacing and the blue-badge icon were visually confirmed. No game was launched or force-quit in that inspection; the native UI automation still failed on Advanced clicks. Native helper tests: 6 passed; credential bridge tests: 2 passed.
