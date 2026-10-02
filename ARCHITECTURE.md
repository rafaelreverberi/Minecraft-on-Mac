# Architecture

SwiftUI views → MainActor AppModel → Swift actor services → isolated native executables.

`LauncherCore` has no UI dependency. Game adapters describe identity/support/launch contracts; Bedrock has no executable or Store download path. LibraryStore owns database mutations, compatibility verification, copy-on-write staging and version selection. GameProcess owns direct CrossOver process supervision. CrossOver discovers hosted Wine and prepared user bottles. RuntimeSelfTests uses bundled release-built probes in temporary directories. Diagnostics accepts only fixed enum events. NativeAccount validates a bounded, typed reply from the bundled hash-pinned Rust helper.

The Rust helper vendors an isolated snapshot of the existing Xodus library and login/license/package mechanisms. It does not invoke xodus run, dump license keys, or run the streaming CLI. Private Keychain namespace, disabled token prints, suppressed panic/stderr payloads, HTTPS-only client, nonpersistent auth WebView and navigation allowlist contain auth data inside that process. Only gamertag, sign-in status, real content-license result, version and fixed errors cross into UI memory. The helper performs real licensing before package lookup; neither operation downloads game bytes.

Persistent layout:

```
Application Support/Minecraft on Mac/
  .library-lock
  library.json
  Games/dungeons2/Versions/
    .staging-UUID/metadata.json + Game/
    1.1.1.0-UUID/metadata.json + Game/
  Compatibility/
  Cache/
  Logs/events.json
  Tools/
```

UUID suffixes support independent snapshots of the same version. Each managed directory carries an installation receipt. The atomic library.json Current UUID is authoritative; a filesystem Current symlink is deliberately unnecessary. External installs have references only; they are never cleanup targets. Saves and account storage live outside managed version trees.

Snapshot pipeline: verify source → stage receipt → APFS clone → verify profile/runtime → inventory every file hash → rename staging to final → atomically register. Current is not switched automatically. Staging remains visible after interruption. Repair replaces only damaged/missing profile DLLs atomically, rechecks the full inventory and never writes a user bottle. Structural/hash/ABI readiness is distinct from interactive gameplay confirmation.

Future remote catalogs use detached Ed25519 signatures with a release-pinned public key and monotonic revision. No remote endpoint or trust-on-first-use key is enabled now. Signed metadata alone will not authorize unsigned executable updates.
