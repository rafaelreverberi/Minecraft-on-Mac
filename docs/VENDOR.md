# Vendored source provenance

Xodus local source baseline: a3afa0569332e32ce2677c0edc643ef85477ee3e, including the local audited source modifications. The separate helper contains the Xodus library/proto/build files, GPLv3 license and the CLI login/license/package/webview pieces. Cargo.lock pins transitive releases and Git revisions.

Launcher modifications: private Keychain service namespace; file-backend compile prohibition; remove token/device-token stdout prints; add gamertag accessor without account-ID exposure; omit interactive subproduct package selection; structured fixed replies; suppress panic payloads; HTTPS-only client with bounded request timeout; incognito Microsoft-only auth navigation.

Runtime source baseline: f4fef062ed0d560f0d2cb4bb929f13b41489fbee with audited local token/profile fixes. Only relevant source/build instructions and synthetic fixtures are preserved in docs/reference; the original compiled DLL is not distributed. XCurl minimal proxy and ABI/export test sources retain their MIT project notices. New runtime loader probe targets the actual seven InitializeApiImpl/QueryApiImpl exports, not imagined flat XUser exports.

Reference manifest carries per-file SHA256 values, making the local modifications and captured build state auditable independent of baseline Git revisions. No authentication cache, game data or raw logs were vendored.

Managed installer additions: Xodus 0dd901be runtime library snapshot preserves the working API/model serialization, with its Windows Keychain backend replaced by credential-bridge and plaintext backing disabled. Current msixvc/msixvc-common source provides persistent extraction; local writes now fail on error rather than looping/logging indefinitely. No developer-only backup source or proprietary SDK binaries are vendored. Shared credential-bridge uses ChaCha20-Poly1305; ZIP/MSI/CAB parsing and SHA verification are statically included in the native helper. Cargo lockfiles pin all build dependencies. Microsoft SDK/archive/component URLs and digests are in NativeHelper/src/acquire.rs; license terms remain those of Microsoft.

Release packaging also collects the locked native-helper and Win64-runtime Rust dependency declarations and available original license/notice texts into the app’s Notices directory. scripts/collect_notices.py records package names, versions and upstream URLs without build-host filesystem paths. Root GPL and runtime/proxy MIT notices remain included.

Bedrock: WineGDK b5d23b074cfd5e28e79acceaaefaf41a26ce6272, LGPL-2.1-or-later;
Roritharr/bedrock-on-macos macOS compatibility patch plus launcher-owned private
socket patch (Compatibility/Bedrock/patches). Build source archive SHA256 and
artifact/patch digests are recorded in Bedrock/provenance.json. llvm-mingw 20260922
is a build-time tool only. No experimental allocator patch is used.

Xodus account wire service sources from revision
64d39eb87a56c7d0d7e7fde0b233654ac5477b0f, GPL-3.0-only, are adapted in
Compatibility/Bedrock/AccountService to use the existing native helper identity,
Keychain namespace and audited Xodus library. No external reference checkout was
modified. See docs/BEDROCK.md for each component's purpose and proprietary
publisher dependency boundaries. Corresponding source is reproducible from the
pinned source archive and the patches published in this repository.
