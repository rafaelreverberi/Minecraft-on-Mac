# Acceptance evidence — 2 October 2026, 0.2.0

The managed path was exercised with the user's native Microsoft sign-in and a fresh official package, without copying reference credentials, game data or an existing bottle.

| Requirement | Observed evidence |
|---|---|
| Real account/ownership | User completed Microsoft sign-in. Native Keychain status, actual content license and Store lookup succeeded. |
| Full fresh download | 9,601,531,904-byte official encrypted package downloaded. Revision 55640c99-2b99-4abd-ba3b-ed4198e427d3, normalized build 1.1.1.0. |
| Package integrity | Untouched reference header/hash-table prefix matched. Every stored data page passed hash-table verification before decryption; full SHA-256 recorded and now embedded as the release pin. Release acquisition rejects unknown digestless revisions. |
| Persistent extraction | Complete local extraction succeeded; roughly 9.6 GB of game data and 113 recorded game/compatibility files. No streaming mount or original game-folder dependency. |
| Components/environment | Pinned official GDK/NuGet/Visual C++ downloads, native extraction, a new UUID-owned Windows 11 CrossOver bottle, VC installation and all three ABI/export/load probes succeeded. |
| Managed Play | Packaged native launcher recorded installationVerified and launchStarted; actual shipping process and per-launch Keychain broker observed. User explicitly confirmed menu, real profile and gameplay without error 0063. |
| Saves | Five save files observed in stable account storage inside the owned bottle, outside version directories. Save contents/account IDs were not read or logged. Cross-version persistence and a full save/restart acceptance cycle remain unverified. |
| Original preservation | Original source inventory and critical game files rehashed unchanged. Original account and bottle were not migrated or overwritten. |
| Automated suite | 21 Swift tests: 19 offline passes, two opt-in live skips. Four native helper tests and two encrypted bridge tests passed. Disposable CrossOver provisioning/probes were separately exercised. |
| Native/Windows credential bridge | Live Wine client/macOS server exchange passed with a synthetic fixture. Real managed gameplay subsequently succeeded with native Keychain credentials. No plaintext credential backend is enabled for managed installs. |
| Package/signing | arm64 release app, own runtime/proxy, helper and synthetic probes; checked resource digests and ad-hoc codesign. No Microsoft DLLs, game data, account cache or bottle included. |

Live testing found and fixed Store composite version parsing, missing FileHash, legacy encrypted CDN transport handling, an ignored nested NTFS dependency patch and optional redistributable executables incorrectly subjected to the shipping x64 requirement. Optional installers remain hash-verified. A regression test accepts their differing architecture and rejects tampering.

The user confirmed gameplay in the fresh installation. Native UI automation is unavailable (Sky Computer Use native pipe closed before response), so that visual evidence is user confirmation, not an automated screenshot. The game was left running for the user; it was not forcibly stopped.

Remaining boundaries: independent second Apple Silicon Mac, real multi-version Store update/rollback, full save/restart persistence cycle, Intel support, download range resume, and Developer ID/hardened runtime/notarization. Only the pinned 1.1.1.0 build is supported. CrossOver must separately be installed/licensed. This is a private preview, not a public notarized release.

## 0.2.1 version management

Versions now distinguishes downloaded installations, local snapshots, existing local registrations and Current selection; paths, Finder access and full version-folder logical size are visible. Delete Version removes all version files/cache/receipts, including Current, and clears its selection. Optional owned-environment/save deletion rejects shared and foreign receipts. Retained environments are recorded, reused on reinstall and separately removable in Storage. Tests verify current removal, cache deletion, default save preservation, explicit environment deletion, host symlink target preservation, and recovery of a deletion interrupted before database commit. 25 Swift tests passed (23 offline, two opt-in live tests skipped). The release bundle built and passed ad-hoc signing verification. No real user version or save was removed for testing.

The 0.2.1 packaged runtime/proxy also passed all three live probes in a newly provisioned disposable CrossOver environment (40.205 seconds). Release notices include collected locked Rust dependency declarations and original license texts. Public preview publication is distinct from production signing/notarization.

## Managed Bedrock feature branch (incomplete live acceptance)

Bedrock's game-scoped database migration, version gates, persistent protected
extraction fixtures and bundled provenance checks pass the 37-test Swift suite
(two pre-existing opt-in live skips), six native-helper tests and 20 MSIXVC tests.
The native credential bridge passes two tests. Disposable CrossOver runtime
unit tests pass nine tests with two explicitly live tests ignored; a separate
Bedrock WinRT/export probe succeeds under CrossOver 26.3. The release app builds
and passes strict ad-hoc signature verification. These checks are not Bedrock
gameplay evidence. Original DungeonsView presentation is unchanged, verified by
source comparison after normalizing game-routing references.

The user explicitly authorized fresh managed live acceptance. Microsoft confirmed
ownership and package 1.26.5203.0; the installer refused the package with
BEDROCK_PACKAGE_INTEGRITY_UNVERIFIED because there was no acceptable authenticated
full SHA256 or reviewed Bedrock revision pin. It stopped before download,
materialization, environment provisioning or game launch. Fresh Bedrock gameplay,
world save/reopen, controller and multiplayer remain unverified. See BEDROCK.md
for storage, security and acquisition boundaries. Do not describe this feature
as having met the full acceptance requirements yet.
