# Minecraft Dungeons II GDK / CrossOver compatibility fix

Final live result, 2 October 2026: the user confirmed that the minimal build reaches the game and displays the correct account label. The game was left running. Gameplay, save persistence, and multiplayer have not been tested.

## Proven cause and installed fix

The first failing direct GDK request was PlayFab `/Client/LoginWithXbox`: curl 55, HTTP 0. GDK XCurl attempted WinHTTP option 118 (`WINHTTP_OPTION_DECOMPRESSION`), which this CrossOver bottle rejected with error 12009 (`ERROR_WINHTTP_INVALID_OPTION`). A standalone probe reproduced rejection for decompression masks 0, 1, 2, and 3 without sending a request.

The final proxy probes that capability without network traffic. When unsupported, it changes only non-null `CURLOPT_ACCEPT_ENCODING` values to an empty string, clearing the GDK backend's decompression flags. An empty string is necessary for this backend: its null-value path crashed in the diagnostic test. All other options pass through unchanged. There are no WinHTTP hooks or network trace logs in the minimal proxy.

The proxy preserves all 61 GDK export names and ordinals, forwarding 60 functions to the genuine Microsoft GDK 2504.6 `XCurl2504.dll`. It intercepts only `curl_easy_setopt`. It does not change credentials, request bodies, headers, TLS verification, or entitlement logic.

With the workaround, diagnostic evidence showed:

| Operation | curl | HTTP | Declared / submitted bytes | Transfer-Encoding / chunked |
|---|---:|---:|---:|---|
| PlayFab `/Client/LoginWithXbox` | 0 | 200 | 2694 / 2694 | absent / no |
| Vex `/login/playfab` | 0 | 200 | 2901 / 2901 | absent / no |

The Content-Length/chunked hypothesis was not the cause of this failure. Safe numerical and public-route evidence is retained in `success-metadata.log` and `winhttp-failure-metadata.log`.

The runtime also explicitly terminates ANSI token result buffers with NUL and returns cached components from the authenticated Xbox profile instead of the upstream hardcoded author label. Capacity checks prevent overwriting gamertag buffers. Unsafe token/header stdout logging was removed; existing safe runtime status diagnostics remain. Authentication and ownership checks use the existing account flow.

`libHttpClient.GDK.dll` and the delegated `xgameruntime.gdk.dll` were left unchanged. No Steam session or `xodus run` is used.

## Launch

```sh
python3 /Users/rafael/Downloads/MinecraftTest/compatibility/launch.py
```

This directly launches `Dungeons/Binaries/WinGDK/Dungeons-WinGDK-Shipping.exe` in bottle `Dungeons2Xbox`, with its WinGDK directory as the working directory. It explicitly passes CrossOver's `--dll` and `--debugmsg` flags because the hosted wrapper resets the corresponding environment settings. Auth-sensitive logging variables and stale DLL mapping variables are removed. Raw game stdout/stderr is discarded.

For a filtered DLL-resolution check only:

```sh
python3 /Users/rafael/Downloads/MinecraftTest/compatibility/launch.py --loader
```

Only the known compatibility DLL loader lines are retained in `loader.log`. Do not start a second game instance while one is running.

## Rebuild and reinstall

Required tools are the existing stable Rust Windows GNU target, MinGW cross compiler, Python 3, and the locally extracted GDK 2504 Update 6 SDK. The build uses MinGW, avoiding the broken Rust `lld-link` discovery path.

```sh
cd /Users/rafael/Downloads/MinecraftTest/xgameruntime-rs-pr12
RUSTC="$(rustup which --toolchain stable rustc)" \
RUSTDOC="$(rustup which --toolchain stable rustdoc)" \
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$(command -v x86_64-w64-mingw32-gcc)" \
rustup run stable cargo build --release --features xuser --target x86_64-pc-windows-gnu

python3 /Users/rafael/Downloads/MinecraftTest/xcurl2504-framing-test/scripts/build.py --minimal
python3 /Users/rafael/Downloads/MinecraftTest/compatibility/manage.py stop
python3 /Users/rafael/Downloads/MinecraftTest/compatibility/manage.py install-minimal
python3 /Users/rafael/Downloads/MinecraftTest/compatibility/launch.py
```

The installer copies the two XCurl files to WinGDK and the rebuilt runtime wrapper to WinGDK, the game root, and the bottle's system32. It refuses to install while the game is running. The optional diagnostic build is separate; the minimal build is the confirmed final configuration.

## Verification

Completed: release builds; exact export comparison and native Wine resolution of all 61 names and ordinals; a native easy-handle/empty-encoding probe; Python syntax checks; five synthetic ABI tests on macOS and the same five on Win64 under Wine; final live login and account-label confirmation by the user.

The ABI tests cover token structure layout, ANSI termination in a dirty buffer, UTF-16 code-unit counts and termination, gamertag capacity checks, and valid empty suffix versus missing profile name. They extract the relevant production writing logic and use synthetic data, without account or network access. They are not a complete test of every runtime API.

```sh
python3 /Users/rafael/Downloads/MinecraftTest/compatibility/test_token_abi.py
python3 /Users/rafael/Downloads/MinecraftTest/compatibility/manage.py report
```

The first command runs the macOS tests and cross-compiles `compatibility/token-abi-tests.exe` for an equivalent Win64 run. No real account data is printed.

## Rollback and backups

Original DLLs, source snapshots, and three save-directory snapshots are in:

`/Users/rafael/Downloads/MinecraftTest/compat-backups/20261002-000620`

```sh
python3 /Users/rafael/Downloads/MinecraftTest/compatibility/manage.py stop
python3 /Users/rafael/Downloads/MinecraftTest/compatibility/manage.py restore
```

Rollback restores the pre-investigation XCurl and runtime wrapper DLLs. It refuses to overwrite managed DLLs that have changed since installation. Current saves are not rolled back. The inert `XCurl2504.dll` sidecar is retained because its prior existence was not recorded; the restored genuine XCurl does not reference it.

## Final WinGDK SHA256 values

| File | SHA256 |
|---|---|
| XCurl.dll | ebab17d3b96356134b283faac615917feb21541357620a04f6bf7a1a0b63597e |
| XCurl2504.dll | cd41c0a777861bee4eaf36e6cd8a26ffadaf78a5e4b0f7eb47d34768a3091d80 |
| xgameruntime.dll | a1d14d5bd93975ed969361d2a0f75bd45e1979f7a8f6ae423a49ae9ad37c893f |
| xgameruntime.gdk.dll | 815d0c5b0aa5c84eb6104168da551a4922f49f8dd02dbdf3bbc5119beec11b59 |
| libHttpClient.GDK.dll | ca94296a372096153806422f5236144c695dfa9cd77607ef1761afc0e5a184b1 |

References: [Microsoft WinHTTP option documentation](https://learn.microsoft.com/en-us/windows/win32/winhttp/option-flags#winhttp_option_decompression), [runtime PR #12](https://github.com/minecraft-linux/xgameruntime-rs/pull/12). Local compatibility behavior and success above are established by this investigation's probes and live tests.
