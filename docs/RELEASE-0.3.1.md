Minecraft on Mac 0.3.1 fixes Bedrock failing to start with **GAME_EXITED / status 99** after a CrossOver environment update.

CrossOver can reset Bedrock's WinRT registrations to its builtin Windows libraries. Those libraries can activate the required factory but do not implement the optional API query Bedrock makes during startup, leading to an unhandled exception. The launcher now restores its verified Bedrock WinRT registrations before runtime checks and Play, in the launcher-owned Bedrock environment only.

- Existing Bedrock installations recover automatically on the next Play. No game download, reinstallation or save reset is required.
- Dungeons II launch behavior, compatibility libraries, account helper and existing installations remain unchanged. The released runtime DLLs/probes and signed native helper are byte-for-byte identical to 0.3.0.
- Adds an actual WinRT API regression probe and a registration test that works in offline CI.
- Packaging can retain an existing signed native helper only after checking its recorded SHA-256 and signature.

**Update:** extract the arm64 app ZIP and replace Minecraft on Mac.app in Applications, then reopen it and select Bedrock → Play. Keep your existing game library and CrossOver environments. CrossOver remains a separate licensed requirement.

**Validation:** 42 Swift tests, zero failures, two existing opt-in skips on the development Mac; the isolated faulty start reproduced status 99, and the corrected API probe returned success. The user confirmed Bedrock's main menu and real profile after the registration repair. Release packaging, final runtime provenance, shared helper identity and strict ad-hoc signature were checked. Dungeons selection, receipts and checked executable/runtime hashes remained unchanged. World save/reopen, controller, multiplayer and an independent second-Mac test are not newly verified by this patch.

Supported: Apple silicon, macOS 14+, Dungeons II 1.1.1.0 and Bedrock for Windows 1.26.5203.0. Bedrock requires CrossOver 26.3.x / D3DMetal. This remains an **ad-hoc signed, unnotarized preview**.

Includes the arm64 app ZIP, corresponding source ZIP and SHA256SUMS-0.3.1.txt. No Microsoft game files, licensed components, credentials or worlds are included. Unofficial; not affiliated with Mojang, Microsoft or CodeWeavers.
