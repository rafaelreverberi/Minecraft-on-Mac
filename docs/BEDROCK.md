# Managed Bedrock

The compatibility profile targets Minecraft for Windows (Store 9NBLGGH2JHXJ,
Microsoft.MinecraftUWP), executable Minecraft.Windows.exe, version 1.26.5203.0
(Bedrock 26.52), Apple silicon, CrossOver 26.3.x, D3DMetal. A fresh licensed
launcher-managed installation and launch now succeeded on the user's Mac; the
user confirmed the main menu and real profile. World save/reopen, controller and
multiplayer acceptance remain unverified. Older versions are rejected. Newer versions are
reported by authenticated package discovery but remain compatibility-unverified
until a new profile and successful probes exist. Your selected installation is
preserved when a check or installation fails.

## Storage and migration

All launcher storage remains in ~/Library/Application Support/Minecraft on Mac.
Game files: Games/<gameId>/Versions/<version>-<UUID>/Game.
Each version also contains Components, metadata.json, download-receipt.json and
package-integrity.json. Downloads start in .staging-UUID under the same game's
Versions directory. Interrupted Bedrock transactions and retained environments remain visible on the
Bedrock sidebar page. The original Dungeons page and its settings presentation
are preserved.
Compatibility/bedrock/<profileId> stores immutable registered WineGDK components;
Dungeons retains its existing runtime and profile. Tools/accounts/<SHA256> stores
the one signed native helper used for sign-in, credential brokerage and Bedrock
account service. Diagnostics contain fixed events and exit status only.

Schema 1 is decoded and validated before migration. library.schema1.backup.json
preserves its exact bytes. Schema 2 writes currentByGameId and
retainedEnvironmentsByGameId; Dungeons's current UUID, paths and environments
are preserved. Corrupt or unsupported databases produce an error and are never
reset. Different games cannot share an environment or current selection.

Owned Bedrock environments are named MinecraftMac-bedrock-<UUID>, with a
schema-2 ownership receipt identifying Bedrock. Worlds/settings reside in the
bottle's drive_c/users/crossover/AppData/Roaming/Minecraft Bedrock and survive
version deletion. Deleting an environment and all saves is explicit, off by
default, receipt checked, and refused when any version still uses it. Reusing a
verified Bedrock environment does not reinstall prerequisites during staging.
New builds may upgrade worlds. Selecting an older executable does not revert
world data. Back up worlds before the first launch of a newer build.

## Installation and integrity

1. Native Microsoft sign-in uses this launcher's private macOS Keychain namespace.
2. Verify a real content license, discover the official Store package/version,
   enforce the version/profile gate, then obtain the licensed content key in memory.
3. Download the full encrypted MSIXVC into transaction-owned SSD storage. Enforce
   service size, package limits and a service-provided digest or revision-pinned
   full digest. No release bootstrap bypass is available for Bedrock.
4. Parse XVD/segment metadata, reject unsafe/duplicate paths and excessive file
   counts/sizes, and extract every file with decrypt_all=true. Protected files
   use the same generic Reader/Writer algorithm as mount_mem_fd, writing ordinary
   new SSD files. Output is separate from the encrypted input, flushed, fsynced
   and length checked; available page hashes are checked before decryption.
5. Retrieve pinned Microsoft GDK 2604.4.7897 and curl 8.22.0_2 from official
   publishers. Extract exact bounded archive members and publisher license texts.
6. Verify the receipt/file inventory. Add only bundled open-source WineGDK
   components, the user's GDK threading DLL and curl CA bundle.
7. Prepare a new owned Windows 11 64-bit D3DMetal environment, register WinRT
   classes and install Game/Installers/GameInputRedist.msi from this account's
   downloaded game. Verify GameInput, runtime profile and WinRT/export probes.
8. Only after all checks succeed rename staging atomically and commit that game's
   selection. Failure preserves the former selection. Encrypted containers are
   removed after successful extraction/receipt creation, never after a failed
   materialization. Large interrupted downloads remain identifiable in staging.

Microsoft's legacy public CDN may provide no authenticated package digest. Such
a package requires a reviewed full revision pin before release installation can
proceed. The launcher deliberately refuses an unpinned HTTP package. The repository pins Dungeons and the separately reviewed Bedrock
1.26.5203.0 revision. The Bedrock review authenticates the retail RSA-PSS/SHA256
header using the public key from our pinned official GDK, verifies its full
Merkle hash tree and every encrypted content page, validates the unused reserved
and mutable pages against the canonical baseline, then records the whole-file
SHA256. See bedrock-package-pin-1.26.5203.json and scripts/review_bedrock_pin.py.
No game data or key from another local installation is used. Never disable certificate or integrity verification to get
around this boundary.

## Play and repair

Play verifies the selected game's persistent inventory, profile, CrossOver version,
owned receipt, GameInput and WinRT/export probes, checks Microsoft ownership,
starts this same helper as the Xodus-compatible account service in a new private
0700 session directory, then executes CrossOver Wine with the persistent
Game/Minecraft.Windows.exe and game-specific overrides. The private socket has
0600 permissions and same-user peer checking. Existing sockets are never unlinked;
unknown services are never killed. The launcher terminates only its own child.
No Xodus run command, RAM volume, hdiutil ram://, WINE_DLL_FILE_MAP, Python bridge,
per-launch game decryption, allocator patch or Metal HUD is used.

Repair authenticates the game account, verifies the owned version/environment
receipts and checks all non-compatibility game hashes before replacing only owned
bundled compatibility DLLs/GameInput. Changed game data requires a new licensed
managed copy; no local reference game is an implicit repair source.

## Build and licensing

scripts/build_bedrock.py builds WineGDK b5d23b074cfd5e28e79acceaaefaf41a26ce6272
from a digest-checked source archive using llvm-mingw 20260922. The research
macOS compatibility patch (LGPL-2.1-or-later) adds build/linkage, account-policy,
IPC and async-lifetime fixes. Our private-account-socket patch isolates IPC from
/tmp/xodus.sock. No allocator performance patch is applied. WineGDK components
implement WinRT factories, JSON/storage/textinput/applicationmodel and GameInput;
xgameruntime.dll retains its builtin identity to pair with xgameruntime.so.
Other Wine DLLs get the audited external-loader marker for CrossOver loading.
The GDK threading dependency is downloaded by the user's installer, not bundled.

AccountService derives from Xodus 64d39eb87a56c7d0d7e7fde0b233654ac5477b0f,
GPL-3.0-only, adapted to the existing audited vendored Xodus library and private
Keychain namespace. No extra account store, token files, license dumps or
tracing subscriber is introduced. Wire messages are bounded by the upstream
16-bit size field and the service applies connection timeouts.

Bundled provenance records repository/revision, patch digests, compiler version,
artifact SHA256s and license. Build tools are development dependencies only;
packaged users need neither Homebrew, Rust, Python nor compilers. Packaging bundles
open-source components and probes, never game files, GDK binaries, credentials,
worlds or license keys. Publisher terms still govern downloaded dependencies.
Permanent SSD preparation reduces per-launch memory/materialization work; no FPS
increase is claimed without an A/B test.

## Acceptance status

Automated tests are synthetic/offline unless explicitly enabled. Live acceptance
requires the user's opt-in and own Microsoft account. Use a new launcher-owned
environment, never BedrockMacOS or the research application data. Confirm a fresh
official download, menu/profile, world create/save/reopen, controller and online
sign-in; multiplayer requires manual confirmation. Inspect processes/mounts for
absence of a new RAM volume and verify the executable originates under
Games/bedrock/Versions. Until recorded, these are unverified, not successful.

### Initial verification, 2 October 2026

- Swift: 37 tests, zero failures, two existing opt-in live tests skipped.
- Native helper: six tests passed; MSIXVC: 20 tests passed, including protected
  persistent extraction and corruption/truncation rejection fixtures.
- Credential bridge: two tests passed. Windows runtime tests in a new disposable
  CrossOver bottle: nine passed, two explicitly live tests ignored.
- Bedrock WinRT/GameInput-export/GDK-export probe passed in a separate disposable
  CrossOver 26.3 environment; this did not download or launch the game.
- Release app packaging and ad-hoc strict signature verification passed.
- DungeonsView matches the repository's original view exactly after normalizing
  explicit game-routing references. No Dungeons presentation changes were made.
- The user authorized a fresh managed live test. The signed packaged helper
  confirmed a real content license and available build 1.26.5203.0. Installation
  stopped with BEDROCK_PACKAGE_INTEGRITY_UNVERIFIED before downloading game data:
  the response supplied no acceptable full SHA256 and no reviewed Bedrock
  revision pin exists. No game environment was provisioned and no Bedrock game
  launch, world persistence, controller or multiplayer acceptance was performed.

That initial attempt is historical. The reviewed pin and corrected packaging
below resolved both blockers. Neither the manual Bedrock installation nor its
bottle was used as a package source or changed.

### Reviewed package pin fix

The original missing-hash refusal was correct for the unreviewed revision but
could not be solved by reconnecting or signing in again. The baseline revision
4650c7c5-9163-48e9-9e89-6a5584d9833f now has a reviewed full SHA256:
af5a72dab1a6e72fb63f931f5d80961b85090142994b93d60d8cd7a3d5a5f6c4.
Microsoft retail RSA-PSS/SHA256 authenticated the header; 2,975 hash-table pages
and all 502,386 encrypted content pages verified against its signed root.
The review downloaded a fresh encrypted 2,069,975,040-byte official container.
It did not decrypt a research installation or trust an HTTP digest blindly.
Production downloading still verifies the entire container before decryption.
The Dungeons pin, compatibility components and environment are unchanged.
New or changed unreviewed Bedrock revisions still fail safely.

Developer inspection commands (package-audit, sdk-public-audit and
bedrock-review-download) exist only behind bootstrap-pins. Release packaging
builds without that feature and refuses engineering bootstrap distribution.
The review-download command cannot install or decrypt a package. Its downloaded
SHA256 is explicitly unverified until the independent signature/hash-tree review
passes. No secrets, game content or proprietary GDK binaries enter the repository.
The review manifest includes only public metadata, public-key hashes and digests.

The packaging step restores the hashed Bedrock Mach-O Unix library after Xcode's
Release copy phase, which otherwise strips .so resources despite SwiftPM .copy.
It re-signs the resource bundle and verifies every final Bedrock artifact after
all copy/signing steps. A missing Bedrock resource bundle or changed artifact
fails packaging. `--reuse-compatibility` verifies and retains existing artifacts
when changing only the installer, preserving Dungeons DLL/probe bytes exactly.

### Successful fresh live acceptance after both fixes

The normal packaged launcher downloaded another complete official container and
verified the reviewed SHA256 before persistent materialization. Native acquisition
completed, including licensed components. The corrected package passed runtime
provenance checks, created a new UUID-owned CrossOver Bedrock environment,
installed GameInput from this downloaded game, passed WinRT/runtime probes,
verified 12,189 persistent game/runtime files and atomically selected Bedrock
1.26.5203.0. The game ran directly from Games/bedrock/Versions/.../Game with the
private native account service. The user confirmed the main menu and actual
profile. This confirmation is user-observed; native UI automation returned
"native pipe closed" and supplied no screenshot.

Dungeons UI/runtime/profile/probes are byte-for-byte unchanged by this fix. Its
library entries, current selection and retained environments matched the recorded
pre-test state after the Bedrock commit. All six existing critical Dungeons
executable/runtime files matched their previously recorded digests. No Dungeons
repair/reinstallation, manual Bedrock modification or credential migration ran.

Verification: 37 Swift tests, zero failures, two existing opt-in live skips; six
native-helper tests in both normal and engineering configurations; 20 MSIXVC
tests; four synthetic pin-review tests across tree depths/run boundaries and
root/node/content corruption/truncation; actual Microsoft signature verification
plus changed signature/flags/root/header-byte rejection; final packaged artifact
SHA256/provenance and strict ad-hoc signature checks. The release helper rejects
engineering inspection commands. World save/reopen, controller and multiplayer
remain separate live acceptance boundaries. Interrupted owned staging folders
remain available for explicit cleanup on the Bedrock page.
