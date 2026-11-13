# Provenance

- Source: <https://github.com/yoshitsugu/google-ime-skkserv-rs>
- Revision: `6bd375e1dddcc80f39e97f0d4f47d0bb5af8109b`
- Vendored files: `Cargo.toml`, `Cargo.lock`, `README.md`, `src/`
  (no `.git`, no CI configuration)
- Upstream package: `gskkserv` 0.2.0, authors and `MIT OR Apache-2.0`
  metadata preserved in `Cargo.toml`.
- Redistribution license for this copy: Apache License 2.0
  (`LICENSE-APACHE`). The upstream MIT option is not used here.

## Changes made for the bundled stdio integration

- Added `--stdio` and `--encoding=<euc-jp|utf-8>` command line options.
  `--stdio` serves the SKK wire protocol on stdin/stdout and never calls the
  TCP listener, daemonizes, binds a port, writes a PID file, or prints startup
  text. stdout carries protocol bytes only. TCP mode keeps its upstream
  EUC-JP behavior and cache; a non-EUC-JP `--encoding` is rejected in TCP mode.
- `--stdio` uses a generic `Read`/`Write` loop so it can be tested with
  in-memory framing. Requests are bounded to 4096 reading bytes; malformed,
  oversized, invalidly encoded or unsupported-code frames write a standalone
  `0`, flush and stop. `0` exits and EOF exits cleanly. HTTP/JSON failures
  write `0` and continue when request framing was valid.
- Google CGI requests now append a trailing comma to the `text` parameter so
  the reading is treated as one conversion span, use one pooled blocking
  client with a 2 second timeout, and read the body through
  `Read::take(1 MiB + 1)` before UTF-8/JSON parsing.
- Added a stdio-only response builder: exactly one JSON segment with a
  nonempty candidate list, candidate filtering (empty, control characters,
  more than 1024 UTF-8 bytes), strict encoding (EUC-JP-unrepresentable
  candidates skipped), a 256 candidate / 64 KiB output cap, and canonical SKK
  `(concat "...\057...\073...")` escaping for literal `/` and `;` (backslash
  and double quote escaped inside the expression).
- Diagnostics no longer print search results, readings or URL query data.
- Tests added for stdio framing, UTF-8/EUC-JP bytes, escaping, filtering,
  caps, multi-segment rejection, and local HTTP query/status/body limits.
