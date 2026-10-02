# Acceptance evidence — 2 October 2026

This record distinguishes code/package tests from interactive game acceptance.

| Requirement | Evidence | Result |
|---|---|---|
| Existing installation appears | Packaged app registered 1.1.1.0, 9,672,194,544 logical bytes, bundled profile and current UUID in durable library | Verified through app-created metadata; UI visual check unavailable |
| Native Play service starts shipping EXE | Packaged app `--smoke-launch-current` invoked AppModel.play, recorded installationVerified/launchStarted, real WinGDK shipping process observed | Process start verified; actual button interaction/menu not visually checked |
| Correct account label | Preserved runtime's cached authenticated profile code and existing game login | Prior user-confirmed reference; new UI-native account sign-in pending |
| Real ownership | Native helper uses content license before package-version lookup; Keychain-only signed-out status test passed | Code/build/typed protocol verified; live owned account pending |
| Fresh install/decryption | Production controls gated | Pending secure Windows runtime broker and verified extraction helper |
| Compatibility profile | Original EXE and five DLL hashes pinned; game-root and bottle runtime checked; x86_64 PE check | Live reference verification passed |
| Existing ABI tests | Exact production-writing fixtures extracted in copied source tree | Five macOS tests and five Win64/Wine tests passed |
| XCurl/runtime | 61 names and ordinals; easy-handle creation; empty and nonempty ACCEPT_ENCODING; seven actual wrapper exports/loadability | Live Wine probes passed without network/account APIs |
| Menu/error 0063 | Game process launched; raw output discarded | Native visual confirmation pending |
| Versions/rollback/removal | Disposable PE/game fixtures: APFS snapshot, unchanged current, switch, damage/failure, DLL repair, rollback, removal/current guard, external unregister | Automated pass; actual two different Store versions pending |
| Saves/login/bottle safety | Version removal constrained to managed receipt; external registration removal leaves files; no save/account/bottle cleanup path invoked | Code/fixture evidence; real save persistence pending |
| Diagnostics/signatures | Allowlisted report, invalid account schema rejection, signed catalog tamper/wrong-key/rollback rejection | Automated pass |
| Build/package | Swift 6 release arm64 app, locked Rust helper, resource/probe/helper digests, ad-hoc codesign verification | Passed; Developer ID/notarization pending |
| Reference preservation | Rehashed original source inventory, shipping EXE and five DLLs after native launch | All unchanged |
| Distribution contents | Packaged app inspected for DLLs | Zero Microsoft DLLs; only own helper/synthetic probe executables |

Final automated suite: 17 Swift tests, with the live reference test separately exercised successfully before launching the game. The offline run skips that one live test and passes the other 16; two Rust protocol tests pass. Synthetic destructive tests run in disposable folders and do not target the installed game.

The native UI tool failed twice with `Sky Computer Use native pipe closed before response`, so no screenshot/button/menu claim is made. The app and launched game were left available for user review. No fresh download, account sign-in, credential migration, forced termination, original DLL replacement or real game uninstall was performed.
