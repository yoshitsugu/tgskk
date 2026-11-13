# Windows TSF frontend

An experimental x64 TSF frontend for ordinary desktop applications.
It uses the same Rust/CSKK input engine as the Linux frontends.
This is a developer build, without an installer or signed release binaries.

The frontend supports kana input, okurigana, dictionary conversion, in-session
word entry, and keyboard candidate selection. The first four candidates stay
inline; subsequent candidates appear in pages of seven. Space and `x` cycle
inline candidates or move between pages; Page Up/Down, Up/Down and `1`–`7` are
also available.

User dictionary learning and persistence, AI assistance, SKK server integration, mouse
candidate selection and Windows language-bar controls are not connected to this
frontend. The native AI worker and managed helper transport can be built and
tested independently. Elevated applications, AppContainer applications, secure
desktop, UI-less mode, 32-bit applications and ARM64 are outside this build's
supported scope.
New words can be committed, but are not remembered for subsequent conversions.
ASCII mode currently commits each printable key through a short TSF composition;
application completion and undo behavior may differ from ordinary keyboard input.
Fields whose InputScope query fails disable input, as do password/private fields.

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

To remove the registration, close applications using tgskk and run:

```powershell
regsvr32 /u "$pwd/build/windows/Release/tgskk-tip.dll"
```

Unregister the old DLL before registering a build from a different directory.
Do not move or delete a registered DLL. `TGSKK_DEBUG=1` writes TSF failure stages
and HRESULTs to stderr as well as the debugger output, without input text.

## Tests

The Windows workflow builds with MSVC and runs the common Rust tests, the C ABI
smoke test, managed child-process deadline tests and the real-model scoring
suite. The scoring comparison builds the pinned pre-batching implementation
against the same Windows backend; Linux floating-point reference values are
checked by the Linux worker tests.

`test-windows-tsf.exe` exercises a real TSF document with an `ITextStoreACP`
fixture. It registers and unregisters the DLL, and adds a temporary profile in
the installed input language so a Japanese language pack is not required on the
test runner. Run it in a disposable Windows environment. Its second argument is
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
