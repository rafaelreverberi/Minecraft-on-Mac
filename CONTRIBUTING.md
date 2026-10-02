# Contributing

Keep the gold-reference installation untouched. Work in this new repository and disposable test directories. Never commit decrypted games, credentials, actual account identifiers, proprietary DLLs, bottles or raw logs.

Build/test commands are in README. Swift 6 strict concurrency is enabled. Add tests for safety contracts and meaningful behavior; use synthetic game/PE fixtures for destructive cases. Live CrossOver tests require explicit `MINECRAFT_LIVE_REFERENCE_TEST=1` and a prepared reference bottle. Never start a second game instance.

Treat entitlement errors as unverified ownership; never infer ownership from an installed executable or available package. Preserve the real Microsoft authentication flow and existing compatibility ABI. Native account data must not enter diagnostics. New game adapters must define an honest support boundary.

No runtime Homebrew/toolchain installer exists. Development dependencies are installed separately by the developer, with their own consent. Release outputs must include the open-source helper/probes and corresponding source/license notices, and exclude Microsoft game/runtime data.
