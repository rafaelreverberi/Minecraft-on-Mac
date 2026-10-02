# Security and release boundaries

No ownership bypass, fake XUser, modified TLS validation, token logging or redistribution of decrypted data is implemented. The gold reference remains outside the new repository.

Native Microsoft credentials use Xodus's macOS Keychain backend under `MinecraftMacLauncher Microsoft`. The helper cannot be built for release with the file backend enabled. Windows runtime credentials are an explicitly documented migration blocker: the existing binary uses a file-backend feature. Play preserves this existing setup and therefore this preview does not meet the final Keychain-only runtime requirement. No credential file is opened or copied by launcher services.

The native helper's login uses the existing Microsoft session exchange, an incognito WebView and HTTPS navigation limited to Microsoft's account hosts. TLS uses the normal trusted certificate chain. Package/content-license calls use real user authorization. Service failures/panics return fixed errors; no raw exception or response payload is forwarded. Account label is UI memory only and excluded from manifests and diagnostics. Sign-out deletes only this helper's namespace; switching the game account is pending migration.

Child stdout/stderr from games and Wine probes goes to the null device. Native-account stdout is a maximum 16 KiB typed JSON reply. Unknown error codes, invalid versions and inconsistent entitlement states are rejected. Auth helper and ABI probes are pinned by local SHA256; this protects against accidental binary drift, not replacement of the entire unsigned development app. Production requires Developer ID signing, hardened runtime and notarization.

Library manifests are written atomically with mode 0600 in an app-data directory created as 0700. Multiple writers are rejected by an OS file lock. Destructive game operations require both a matching recorded managed path and matching UUID receipt. Path traversal and symlink trees are rejected. These checks do not claim protection against a hostile same-user process racing filesystem checks; a production extraction engine should use descriptor-relative no-follow writes and enforce archive path/size/integrity limits.

Unknown game builds cannot borrow the reference patch profile. No remote executable updater is enabled. Future signed catalogs require a pinned Ed25519 key and reject rollback revisions. No telemetry/crash uploading exists.

Public release is blocked on secure Windows credential IPC, structured verified package extraction, official component provenance, interactive fresh-install/update acceptance, licensing inventory and code-signing/notarization. Report security issues privately to the repository owner; no public reporting endpoint is configured.
