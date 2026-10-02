# Phase status

0: audited and hashed reference, source snapshots preserved, originals unchanged.
1: native SwiftUI shell implemented and compiled as an app bundle.
2: durable/exclusively locked database, managed snapshots, receipts and staging implemented.
3: reference-faithful Play service implemented; live readiness tests pass. Native UI automation unavailable in this environment.
4: native Keychain account helper, real license check and package-version API implemented. Interactive Microsoft sign-in not yet validated. Secure Windows runtime migration pending.
5–6: fresh verified download/decryption and zero-install acceptance pending. Do not use existing CLI exit code as a receipt.
7: installed snapshot selection/rollback works; actual Microsoft second-version download/update acceptance pending.
8: per-version removal and compatibility-only repair implemented and tested on disposable fixtures. Full data/bottle/save/account uninstall combinations pending; no saves/login/bottle deletion is exposed.
9: sanitized diagnostics and on-demand dependency discovery implemented. Automatic runtime-component provisioning pending.
10: Swift/Rust tests, release bundle and private ad-hoc signing implemented. Public signing/notarization and full acceptance pending.

Next implementation gate: memory-only Windows runtime backed by a native Keychain broker, built in a separate compatibility revision and tested in a managed clone/dedicated bottle. Then replace CLI streaming with an entitlement-first typed helper: strict official HTTPS origins, validated content identity, archive path/size checks, encrypted package digest, persistent full decryption, per-file receipts, byte/speed progress, cancellation, safe crash recovery. A new version may become Current only after its pinned profile and required self-tests succeed.
