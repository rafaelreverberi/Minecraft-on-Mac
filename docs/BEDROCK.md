# Managed Bedrock

The compatibility profile targets Minecraft for Windows (Store 9NBLGGH2JHXJ,
Microsoft.MinecraftUWP), executable Minecraft.Windows.exe, version 1.26.5203.0
(Bedrock 26.52), Apple silicon, CrossOver 26.3.x, D3DMetal. This is the user's
reported working research baseline, not a claim that this launcher has passed
live gameplay acceptance. Older versions are rejected. Newer versions are
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
proceed. The launcher deliberately refuses an unpinned HTTP package. The current
repository has a Dungeons pin; the Bedrock pin must come from an authenticated
Microsoft response or a separately reviewed official acquisition, not another
local installation. Never disable certificate or integrity verification to get
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

### Local verification, 2 October 2026

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

This integration is not yet fully accepted. Resolving the official acquisition
integrity anchor and completing the fresh installation/gameplay acceptance are
required before claiming the definition of done. Neither the manual Bedrock
installation nor its bottle was used as a package source or changed.
