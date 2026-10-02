# Gold reference audit — 2 October 2026

Read-only inspection; original game, compatibility installation, credentials, and bottle unchanged.
Game identity: Microsoft.MinecraftDungeons2, 1.1.1.0. Shipping executable is WinGDK, not root bootstrapper.

Required: minimal XCurl proxy (61 exports, intercepts only curl_easy_setopt), genuine GDK 2504.6 XCurl2504 sidecar, rebuilt xgameruntime wrapper, unchanged delegated xgameruntime.gdk and libHttpClient.GDK. Wrapper also exists at game root and bottle system32. WinHTTP option 118 rejects decompression, so proxy supplies an empty ACCEPT_ENCODING string only for non-null values on unsupported WinHTTP. NULL is unsafe in the genuine backend. Token termination and cached authenticated gamertag components are source-level Rust fixes.

Portable: compiled x86_64 proxy/runtime and ABI fixtures; explicit hosted Wine launch flags. Hardcoded: user game path, Dungeons2Xbox bottle, SDK extraction location, source folders, backup path and local toolchains. Native services must own discovery, manifests, staging, version selection, diagnostics and process supervision. Keep C/Rust ABI work compiled; never replace with random patches.

Security blocker found in source: runtime Cargo.toml explicitly enables xodus key-chain-file. Windows runtime credentials therefore have a file-backed path, unlike macOS Xodus which defaults to native Keychain. Do not copy/read credential files into this repository. Preserving the reference launch is a development migration mode, not proof of production Keychain compliance. A memory-only Windows runtime + native Keychain IPC broker is required before public release. The existing game login must not be silently signed out or migrated.

Xodus package get_packages uses real Xbox authorization and license::get_license uses Microsoft's content license flow; never substitute package presence for entitlement. CLI streaming performs some remote metadata work before get_license, reports internal failures by returning from a void function, and can still exit success. It offers neither a safe typed completion receipt nor structured account/progress API. Therefore production Install/Update must fail closed until an isolated helper supplies entitlement-before-download, verified package receipts, bounded errors, and safe extraction. Do not blindly wrap its exit code as success.

Existing test_native.py belongs to an older Win64 adapter/IPC experiment, not the proven WinGDK configuration. Relevant tests: compatibility/test_token_abi.py and xcurl tests/gdk_exports.c. Preserve the five extracted production-writing ABI fixtures and the 61 export name/ordinal resolution checks. Existing success report covers login/menu and correct label, not gameplay/save/multiplayer.

Reference source snapshots and hash inventory are in reference/. No game data, proprietary DLLs, account data or raw debug logs are included. License attribution for snapshot files remains with their original projects. Shipping those snapshots/binaries requires license review.

Microsoft documents the decompression option at https://learn.microsoft.com/en-us/windows/win32/winhttp/option-flags#winhttp_option_decompression. Native credential storage follows https://developer.apple.com/documentation/security/keychain-services.
