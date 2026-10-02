Minecraft on Mac 0.3.0 simplifies the native launcher for Minecraft Dungeons II and Minecraft: Bedrock for Windows. Each game now has its own artwork, Play action, version selection and Versions manager. Maintenance tools are tucked into Advanced instead of crowding the game page.

- Official offline game banners, corrected sidebar image/text spacing and the Minecraft Launcher icon adapted with a blue desktop badge.
- Independent per-game installed-version selection, snapshots, inspection and deletion. No global Library navigation.
- A red X beside Play appears only for the identified running game. It force-quits that registered game's process and its CrossOver descendants after checking owner, executable and kernel start time. Unrelated processes and shared wineserver are excluded. **Unsaved progress can be lost.**
- Updated README covering both supported Windows games, the native SwiftUI/AppKit + Rust stack, managed setup, storage and compatibility limits.
- Includes the previously validated Bedrock package pin/runtime packaging fixes and successful fresh-install evidence. Normal launch arguments, authentication, download and runtime DLL behavior are unchanged by the presentation/quit additions.

Download the arm64 app ZIP, extract it and move Minecraft on Mac.app to Applications. Install, open and license CrossOver separately. The packaged app handles Microsoft sign-in, entitlement, official content downloads and its own game environment; no Rust, Python, Homebrew or Xcode is needed by end users.

Supported: Apple silicon, macOS 14+, Dungeons II 1.1.1.0 and Bedrock for Windows 1.26.5203.0. Bedrock requires CrossOver 26.3.x / D3DMetal. Bedrock Preview/Beta and a separately named CrossOver Beta.app are not supported. This remains an **ad-hoc signed, unnotarized preview**; macOS may require first-run Privacy & Security approval.

Validation: 41 Swift tests with zero failures and two opt-in live skips; six native-helper tests; two credential-bridge tests; Release packaging, delivered runtime provenance and strict ad-hoc signature checks. Actual force-quit behavior was tested only with disposable native process fixtures, including child termination and survival of an unrelated sibling. Sidebar/icon rendering was inspected in the real packaged app; full UI click-through encountered native automation failures. No new complete game download, world save/reopen or multiplayer acceptance is claimed for this release.

App ZIP and corresponding source ZIP are accompanied by SHA256SUMS-0.3.0.txt. This project is unofficial and not affiliated with Mojang, Microsoft or CodeWeavers.
