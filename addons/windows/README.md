# Windows TSF frontend

An experimental x64 TSF frontend for ordinary desktop applications.
It uses the same Rust/CSKK input engine as the Linux frontends.
Installer and ZIP development builds are available from [GitHub Releases](https://github.com/yoshitsugu/tgskk/releases) as Windows previews.
They are unsigned; interactive application compatibility is still being tested.

The frontend supports kana input, okurigana, dictionary conversion, word entry,
persistent learning, local AI ranking and SKK servers. The first four candidates
stay inline; subsequent candidates appear in pages of seven. Space and `x` cycle
inline candidates or move between pages; Page Up/Down, Up/Down and `1`–`7` are
also available. AI-reordered candidate pages carry a small `AI-ranked` label.

A per-logon `tgskk-broker.exe` shares dictionaries and background workers between
applications. Each application has its own composition state. Dictionary saves,
model loading, scoring and network requests run off the input thread. The TIP
uses a local dictionary while connecting; it adopts the broker only between
compositions. An IPC failure cancels the current composition and returns to
local input. The failed key is not replayed because the broker may already have
processed it. Local fallback has no persistent learning, AI or server access.

Elevated applications, AppContainer applications, secure desktop, UI-less mode,
32-bit applications and ARM64 are outside this build's supported scope. Mouse
candidate selection is not implemented.
The language bar shows the input mode and offers ON/OFF, kana/ASCII/full-width
modes and Settings. The half-width/full-width key toggles IME ON/OFF; dedicated
IME ON and IME OFF keys are also supported. Windows compartment changes are
synchronized with the input engine. Turning OFF cancels pending composition.
ASCII mode passes ordinary printable keys directly to the application.
Mode-switch commands still go through the input engine.
Unavailable optional InputScope metadata permits local dictionary input and local AI.
Password/PIN/private scopes, malformed descriptors and operational query
failures disable input.
AI requires a collapsed selection; network access additionally requires a known
non-private scope by default. `SkkservWithoutInputScope=true` opts into server
lookups when optional scope metadata is unavailable (including `E_FAIL`). This
does not override password/PIN/private scopes, malformed descriptors, operational
failures, or explicit unknown scope values. The settings window exposes the same
option, disabled by default. Explicit unknown scope descriptors disable both
forms of assistance.
Only bounded text left of the caret is used for local AI; servers receive readings,
never surrounding text.

## Install

Open a **Windows preview** in [GitHub Releases](https://github.com/yoshitsugu/tgskk/releases)
and download only `tgskk-*-windows-x64-setup.exe` and `SHA256SUMS`.
A GitHub account is not required. Source archives and the manual-registration ZIP
are separate downloads and are not needed for ordinary installation.
Run the installer with administrator permission.
It installs the x64 frontend, broker, settings app, large dictionary, verified
AI model, Google helper and app-local Microsoft C++ runtime under Program Files.
Add Japanese in Windows language settings if needed, then select tgskk as its
input method. The files are unsigned; compare the accompanying SHA256SUMS with
`Get-FileHash` before installation.

Before updating or removing tgskk, finish composition and sign out of Windows.
Other signed-in users must also close applications using tgskk. Setup refuses
to replace binaries in use and never forcibly terminates input applications or
the broker. Updates and uninstall preserve `%LOCALAPPDATA%\tgskk`, including
settings, learned entries and backups. Uninstall through Windows Installed apps.
The ZIP has the same payload for manual registration; moving a registered DLL
requires unregistering the previous path first.

## Settings

Open **tgskk settings** from the Start menu. It edits dictionary paths and
encoding, AI and model settings, Google/custom SKK servers, and the number of
candidates shown before the list (0–64, default 4).
Saving validates the complete configuration and replaces it atomically;
invalid values or write errors preserve the previous file and user dictionary.
Sign out and back in to apply changes to all input applications and the broker.

## Build

Use a Visual Studio developer shell with the C++ desktop workload, Windows SDK,
CMake, and Rust 1.88 or newer. The Rust toolchain must target
`x86_64-pc-windows-msvc`.

```powershell
cargo build --release -p tgskk-ffi --locked
cmake -S addons/windows -B build/windows -A x64 `
  -DTGSKK_RUST_LIBRARY="$pwd/target/release/tgskk_ffi.lib"
cmake --build build/windows --config Release
ctest --test-dir build/windows -C Release --output-on-failure --no-tests=error
```

For manual testing in a disposable Windows environment, place an EUC-JP
`SKK-JISYO.L` beside `tgskk-tip.dll`. Alternatively, set `TGSKK_DICTIONARY` to an
absolute dictionary path before launching the input application. Register the
DLL from an elevated shell, then select tgskk in the Japanese input methods:

```powershell
regsvr32 "$pwd/build/windows/Release/tgskk-tip.dll"
```

Keep `tgskk-broker.exe` beside the DLL. It starts automatically for normal
medium-integrity desktop applications. Hosts that forbid child-process breakaway
from a job may require starting the broker from a normal terminal first:

```powershell
& "$pwd/build/windows/Release/tgskk-broker.exe"
```

To remove the registration, close applications using tgskk and run:

```powershell
regsvr32 /u "$pwd/build/windows/Release/tgskk-tip.dll"
```

Unregister the old DLL before registering a build from a different directory.
Do not move or delete a registered DLL. `TGSKK_DEBUG=1` writes TSF failure stages
and HRESULTs to stderr as well as the debugger output, without input text.

## Configuration

Settings are read at application/broker startup from
`%LOCALAPPDATA%\tgskk\windows.ini` (UTF-8). Relative paths are resolved from
that directory. `TGSKK_WINDOWS_HOME` can point to another absolute directory for
a separate configuration. Close input applications and stop the broker before
changing settings or replacing its executable.

```ini
[General]
Dictionary=C:\dictionaries\SKK-JISYO.L
Encoding=euc-jp
AI=true
Skkserv=Google
SkkservWithoutInputScope=false
```

AI and Google lookup are enabled by default. Missing optional binaries or a
missing model leave dictionary input available and emit a diagnostic.
The installer supplies shared model weights. A per-user model at
`%LOCALAPPDATA%\tgskk\models\zenz-v3.2-small-Q5_K_M.gguf` takes precedence.
For manual builds, place `tgskk-zenz-worker.exe` beside the broker and use either
that per-user model location or a `models` directory beside the broker.
Build the worker from [native/zenz](../../native/zenz) using CMake/MSVC; the
[Windows workflow](../../.github/workflows/windows.yml) shows the pinned source
and model preparation. `Worker` and `Model` override their absolute or relative
paths. Set `AI=false` to disable assistance.

Build the bundled Google helper and copy it beside the broker:

```powershell
cargo build --release --manifest-path third_party/gskkserv/Cargo.toml `
  --target-dir target --locked
Copy-Item target/release/gskkserv.exe `
  build/windows/Release/tgskk-google-skkserv.exe
```

`GoogleHelper` overrides its path. Set `Skkserv=Disabled` to opt out, or configure
another server:

```ini
Skkserv=Custom
Host=127.0.0.1
Port=1178
ServerEncoding=euc-jp
```

`ServerEncoding=utf-8` is also supported. Local dictionary hits never query a
server. A missing-candidate lookup keeps the reading visible while fetching in
the background; press Space again to use its cached result. Displayed candidate
order never changes afterward. A cached miss enters word entry; Shift+Space,
or Space after three seconds of unresolved lookup, also allows word entry.

The UTF-8 user dictionary defaults to `SKK-JISYO.user` in the configuration
directory; `UserDictionary` overrides it. Saves are asynchronous and coalesced
after one second of idle input. The previous file remains as `.BAK`.
A persistent `.lock` file prevents simultaneous writers, including brokers with
different configurations pointing at the same dictionary. A second writer uses
a read-only snapshot. Do not delete the lock file while a broker is running.
Stop the broker normally (Ctrl+C in its terminal) to flush pending learning;
force termination can lose recent learning. Unsaved changes remain in memory
and retry after save errors, but cannot survive a crash or failed shutdown.

Broker requests have a 50 ms processing/IPC failure deadline; operating-system scheduling
and completion of canceled I/O can exceed it. This is a failure cutoff, not a
scoring wait: model/server work is never awaited by an ordinary key. The input
owner runs one Windows priority step above the scorer. Connection setup is asynchronous.
Failures are visible through debugger output (`TGSKK_DEBUG=1` also uses stderr).
No input text is included in broker connection/save diagnostics.

## Tests

The Windows workflow builds with MSVC and runs the common Rust tests, the C ABI
smoke test, managed child-process deadline tests and the real-model scoring
suite. Medium-integrity integration tests exercise the authenticated broker,
shared learning, atomic-save failure/retry, restart, stale/malformed IPC, session
isolation, delayed UTF-8/EUC-JP servers, local-hit bypass and fallback. They also
measure warm key RPC latency and run real-model context ranking through the
broker. A native TSF run asserts that its keys reached the broker. The scoring comparison builds the pinned pre-batching implementation
against the same Windows backend; Linux floating-point reference values are
checked by the Linux worker tests.

`test-windows-tsf.exe` exercises a real TSF document with an `ITextStoreACP`
fixture. It registers and unregisters the DLL, and adds a temporary profile in
the installed input language so a Japanese language pack is not required on the
test runner. Native broker/AI TSF tests run under a disposable standard account,
with registration prepared separately by an administrator. The CI-only launcher
removes the account/profile and restores its temporary file/desktop access after
draining child processes. Run registration tests in a disposable Windows environment. Its second argument is
an EUC-JP test dictionary; the workflow shows how to prepare this fixture.

On the Windows 2025 runner with this fixture, rejecting a composition retains a
TSF notification reference even in an independent API-only control. This also retains the document
context. Rejection leaves the input state and document unchanged, but DLL unloading
after this path is not guaranteed. The workflow checks normal input with strict
unload assertions, and tests rejection in a separate process against the control;
additional retained service references or edit-session objects fail the test.
The rejection test modes use exit code 2 to distinguish this observed retention
from a test failure (1) or full release (0).

Automated TSF tests do not establish compatibility with individual applications.
Interactive Windows 11 application testing is required before regular use.

## Publish a Windows preview

Push a tag named `windows-preview-vVERSION-COMMIT`, where VERSION matches the
workspace version and COMMIT is the first 12 characters of the tagged commit ID.
The Windows workflow rebuilds and tests the binaries and installer, runs the
quality and performance checks, verifies every release checksum, then publishes
a prerelease with separate installer, ZIP and complete source downloads.
The release stays a draft until all files have been uploaded. Existing releases
and the stable latest release are not replaced.

For branch/PR builds, `windows-x64-installer` is the smaller Actions artifact
containing only the installer and checksums. `windows-x64-distribution` also
contains the ZIP and sources.
